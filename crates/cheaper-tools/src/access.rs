//! Port of `packages/tool-runtime/src/access.ts`.
//!
//! writeScope の canonical identity と display、manifest/scope 認可の唯一の
//! 実装(`ResourceAccess`)。TS の async メソッドは全て同期 FS で完結する
//! (T2-1 と同じく core `resolve_workspace_path`/`canonicalize_workspace_root`
//! は sync)ため sync `Result` として移植する。`observe`/`snapshot` が共有
//! 状態を更新するため、行バッファは `Mutex`、prepare 結果は `OnceLock` に
//! 保持する(`AccessObserver` が sync `Fn + Send + Sync` の前提)。
//!
//! Slice 境界について:
//! - `ReadStrictness`・`LeasePreparation`・`invalidResource` は
//!   `crate::resources`(T2-2)が所有する。`ReadStrictness` は
//!   `access::ReadStrictness` として再 export し(TS import path と揃える)、
//!   表現ドリフトを防ぐ。
//! - `operation()` が呼ぶ catalog の `resources()` 宣言導出は
//!   `catalog.rs`(T2-4)が所有する。ここでは typed declaration を
//!   `normalize_resource_manifest` が受け取る untyped JSON への写しに
//!   限定し(T2-7 での seam 統合)、policy 導出は重複させない。

use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

use cheaper_core::resources::normalization::{
    normalize_resource_manifest, ResourceError, ResourceManifestDeclaration, ScopeDeclaration,
};
use cheaper_core::resources::predicates::claim_contains;
use cheaper_core::resources::types::{
    ClaimMode, Resource, ResourceClaim, ResourceManifest, ResourceScope,
};
use cheaper_core::tools::vocabulary::ToolName;
use cheaper_core::worker::validate::is_mutating_tool;
use cheaper_core::workspace::deny_paths::PathDenylist;
use cheaper_core::workspace::paths::{
    canonicalize_workspace_root, resolve_workspace_path, SymlinkPolicy,
};
use serde::Serialize;
use serde_json::{json, Map, Value};

use crate::catalog::ToolCatalog;
use crate::resources::LeasePreparation;
use crate::walker::{rethrow_as_denied, DeniedError};
use crate::ToolError;

/// `ReadStrictness` は `crate::resources` が所有する。`access.rs` からの
/// import(TS の `access.ts` import path)を保つため再 export する。
pub use crate::resources::ReadStrictness;

/// `AccessObserver`: `(path, mode, scope?, support?) => Promise<void>`。
/// observe 失敗(scope 拒否等)は `Err` で呼び出し側へ伝播する。
/// Rust 側の `observe` は全て同期処理のため callback も同期 `Fn`。
pub type AccessObserver<'a> = dyn Fn(&str, ClaimMode, Option<ResourceScope>, bool) -> Result<(), ToolError>
    + Send
    + Sync
    + 'a;

/// `AccessObservation`。field 順は TS の object literal 順
/// (`operationId?(先頭)・path・claim・declared・allowed・support`)を
/// serde 出力にも反映する。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessObservation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    pub path: String,
    pub claim: ResourceClaim,
    pub declared: bool,
    pub allowed: bool,
    pub support: bool,
}

/// `AccessSnapshot`。TS literal の key 順(`total`・`dropped`・`observations`)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AccessSnapshot {
    pub total: usize,
    pub dropped: usize,
    pub observations: Vec<AccessObservation>,
}

/// `PreparedResourceScope extends LeasePreparation`。
/// base fields(`claims`・`read_strictness`)は flat に展開する。
#[derive(Debug, Clone)]
pub struct PreparedResourceScope {
    pub root: String,
    pub manifest: ResourceManifest,
    /// `writeScope` 由来の literal writeClaims。`write_scope` 未指定時は
    /// `None`(TS `undefined`)。
    pub write_claims: Option<Vec<ResourceClaim>>,
    /// `LeasePreparation["claims"]`(reads + writes + produces)。
    pub claims: Vec<ResourceClaim>,
    /// `LeasePreparation["readStrictness"]`。
    pub read_strictness: ReadStrictness,
}

impl PreparedResourceScope {
    /// `LeasePreparation` 部分を切り出す(TS の structural typing 相当)。
    pub fn lease_preparation(&self) -> LeasePreparation {
        LeasePreparation {
            claims: self.claims.clone(),
            read_strictness: self.read_strictness,
        }
    }
}

/// `ResourceScopeOptions`。
#[derive(Debug, Clone, Default)]
pub struct ResourceScopeOptions {
    /// `ResourceManifestInput`(untyped JSON。normalize が検証する)。
    pub resource_manifest: Option<Value>,
    /// `ReadonlySet<string>`。insertion order 保持・重複除去は ctor で行う。
    pub write_scope: Option<Vec<String>>,
    pub read_strictness: Option<ReadStrictness>,
}

/// `ScopeAccessOptions extends ResourceScopeOptions`。
/// base fields は `scope` に保持する(二重定義防止)。
pub struct ScopeAccessOptions {
    /// `ResourceScopeOptions` 部分(TS の extends 基底)。
    pub scope: ResourceScopeOptions,
    pub root: String,
    pub symlink_policy: SymlinkPolicy,
    pub names: Vec<ToolName>,
    pub denylist: Option<PathDenylist>,
    /// tool ごとの `resources()` 宣言導出の所有者(catalog.ts parity)。
    /// `operation()` が catalog の declaration を untyped JSON 化して
    /// `normalize_resource_manifest` へ渡す。
    pub catalog: ToolCatalog,
}

/// `ActiveResourceAccess`。
#[derive(Debug, Clone)]
pub struct ActiveResourceAccess {
    pub id: String,
    pub claims: Vec<ResourceClaim>,
}

/// `contains(claims, claim)`。
fn contains(claims: &[ResourceClaim], claim: &ResourceClaim) -> bool {
    claims.iter().any(|parent| claim_contains(parent, claim))
}

/// iterator 版 `contains`(bucket concat の一時 Vec 確保を避ける)。
fn contains_iter<'a>(
    claims: impl Iterator<Item = &'a ResourceClaim>,
    claim: &ResourceClaim,
) -> bool {
    claims
        .into_iter()
        .any(|parent| claim_contains(parent, claim))
}

/// `allClaims(manifest)`。
fn all_claims(manifest: &ResourceManifest) -> Vec<ResourceClaim> {
    manifest
        .reads
        .iter()
        .chain(&manifest.writes)
        .chain(&manifest.produces)
        .cloned()
        .collect()
}

/// `MAX_ACCESS_OBSERVATIONS`。
pub const MAX_ACCESS_OBSERVATIONS: usize = 1024;

/// `invalidResource`(resources.ts の canonical 実装)を `ToolError` に包む。
fn invalid_resource(message: &str) -> ToolError {
    Box::new(crate::resources::invalid_resource(message))
}

/// `ResourceError` を TS の生の throw 値に戻す。TS では
/// `normalizeResourceManifest` が `CheaperError`/`WorkspaceEscapeError` を
/// そのまま throw するため、enum wrapper を剥がして伝播する
/// (`WorkspaceEscapeError` は `rethrow_as_denied` の downcast が効く形)。
fn manifest_error(e: ResourceError) -> ToolError {
    match e {
        ResourceError::Invalid(e) => Box::new(e),
        ResourceError::Escape(e) => Box::new(e),
    }
}

/// `new Set(writeScope)` の insertion order + dedup。
fn dedup_write_scope(items: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    items
        .into_iter()
        .filter(|s| seen.insert(s.clone()))
        .collect()
}

/// `ResourceManifestDeclaration`(typed)を `normalize_resource_manifest`
/// が受け取る untyped JSON に戻す。`None` bucket は key 省略(TS `undefined`)
/// とする — `produces` の `null` は normalize が error にするため。
fn manifest_declaration_value(decl: &ResourceManifestDeclaration) -> Value {
    fn bucket(items: &Option<Vec<ScopeDeclaration>>) -> Option<Value> {
        items.as_ref().map(|list| {
            Value::Array(
                list.iter()
                    .map(|d| json!({"path": d.path, "scope": d.scope.as_str()}))
                    .collect(),
            )
        })
    }
    let mut obj = Map::new();
    if let Some(v) = bucket(&decl.reads) {
        obj.insert("reads".to_string(), v);
    }
    if let Some(v) = bucket(&decl.writes) {
        obj.insert("writes".to_string(), v);
    }
    if let Some(v) = bucket(&decl.produces) {
        obj.insert("produces".to_string(), v);
    }
    Value::Object(obj)
}

struct ObservedRows {
    rows: Vec<AccessObservation>,
    total: usize,
}

/// writeScope の canonical identity と display、manifest/scope 認可の唯一の実装。
pub struct ResourceAccess {
    options: ScopeAccessOptions,
    /// TS は `??=` で settled promise(結果・拒否とも)を cache する。
    /// Rust では成功のみ cache し、error は `prepare_scope` を再実行して
    /// 返す — `prepare_scope` は決定的なため同一 error が再生成され、
    /// `WorkspaceEscapeError` の concrete type(downcast)も保持できる。
    preparation: OnceLock<PreparedResourceScope>,
    observed: Mutex<ObservedRows>,
    /// `readonly readStrictness`。
    pub read_strictness: ReadStrictness,
    /// `readonly explicit`。
    pub explicit: bool,
}

impl ResourceAccess {
    /// `new ResourceAccess(options, inherited?)`。TS の `throw
    /// invalidResource` は `Err` に写す。`readStrictness` の値検証は
    /// `Option<ReadStrictness>` が closed union のため不要(到達不能)。
    pub fn new(
        options: ScopeAccessOptions,
        inherited: Option<ReadStrictness>,
    ) -> Result<Self, ToolError> {
        if inherited.is_some()
            && options.scope.read_strictness.is_some()
            && inherited != options.scope.read_strictness
        {
            return Err(invalid_resource("parent read strictness mismatch"));
        }
        let read_strictness = inherited
            .or(options.scope.read_strictness)
            .unwrap_or(ReadStrictness::Record);
        let explicit =
            options.scope.resource_manifest.is_some() || options.scope.write_scope.is_some();
        let options = ScopeAccessOptions {
            scope: ResourceScopeOptions {
                write_scope: options.scope.write_scope.map(dedup_write_scope),
                ..options.scope
            },
            ..options
        };
        Ok(Self {
            options,
            preparation: OnceLock::new(),
            observed: Mutex::new(ObservedRows {
                rows: Vec::new(),
                total: 0,
            }),
            read_strictness,
            explicit,
        })
    }

    /// `prepare()`(memoized)。成功値を共有参照で返す(TS は同一 object を
    /// 共有する)。失敗時は再計算で同等の error を返す(上記フィールド注記)。
    pub fn prepare(&self) -> Result<&PreparedResourceScope, ToolError> {
        if let Some(prepared) = self.preparation.get() {
            return Ok(prepared);
        }
        let prepared = self.prepare_scope()?;
        // 競合した場合は先に set された値を返す(決定的なため等価)。
        Ok(self.preparation.get_or_init(|| prepared))
    }

    /// `prepareWorkerManifest`: resourceManifest 無しで準備した literal
    /// writeClaims を safety の各 bucket に narrow する。
    pub fn prepare_worker_manifest(
        &self,
        safety_manifest: Option<&Value>,
    ) -> Result<ResourceManifest, ToolError> {
        let prepared = self.prepare()?;
        let safety = match safety_manifest {
            None => prepared.manifest.clone(),
            Some(input) => normalize_resource_manifest(
                &prepared.root,
                Some(input),
                self.options.symlink_policy,
            )
            .map_err(manifest_error)?,
        };
        let Some(write_claims) = &prepared.write_claims else {
            return Ok(safety);
        };
        Ok(ResourceManifest {
            reads: safety.reads,
            writes: write_claims
                .iter()
                .filter(|claim| contains(&safety.writes, claim))
                .cloned()
                .collect(),
            produces: write_claims
                .iter()
                .filter(|claim| contains(&safety.produces, claim))
                .cloned()
                .collect(),
        })
    }

    fn prepare_scope(&self) -> Result<PreparedResourceScope, ToolError> {
        let opts = &self.options;
        let root = canonicalize_workspace_root(&opts.root).map_err(|e| Box::new(e) as ToolError)?;
        let mut write_claims: Option<Vec<ResourceClaim>> =
            opts.scope.write_scope.as_ref().map(|_| Vec::new());
        if let (Some(claims), Some(scope)) = (write_claims.as_mut(), &opts.scope.write_scope) {
            for target in scope {
                let resolved = resolve_workspace_path(&root, target, opts.symlink_policy)
                    .map_err(|e| Box::new(e) as ToolError)?;
                claims.push(ResourceClaim {
                    resource: Resource::workspace(resolved.real, ResourceScope::Exact),
                    mode: ClaimMode::Exclusive,
                });
            }
        }
        let root_claim = |mode: ClaimMode| ResourceClaim {
            resource: Resource::workspace(root.clone(), ResourceScope::Subtree),
            mode,
        };
        let manifest = match &opts.scope.resource_manifest {
            Some(input) => normalize_resource_manifest(&root, Some(input), opts.symlink_policy)
                .map_err(manifest_error)?,
            None => ResourceManifest {
                reads: if opts.names.is_empty() {
                    Vec::new()
                } else {
                    vec![root_claim(ClaimMode::Shared)]
                },
                writes: if opts.names.iter().any(|name| is_mutating_tool(*name)) {
                    write_claims
                        .clone()
                        .unwrap_or_else(|| vec![root_claim(ClaimMode::Exclusive)])
                } else {
                    Vec::new()
                },
                produces: Vec::new(),
            },
        };
        if let Some(writes) = &write_claims {
            if manifest
                .writes
                .iter()
                .chain(&manifest.produces)
                .any(|claim| !contains(writes, claim))
            {
                return Err(invalid_resource("manifest writes exceed writeScope"));
            }
        }
        let reads = if self.read_strictness == ReadStrictness::Record && !opts.names.is_empty() {
            vec![root_claim(ClaimMode::Shared)]
        } else {
            manifest.reads.clone()
        };
        Ok(PreparedResourceScope {
            claims: reads
                .iter()
                .chain(&manifest.writes)
                .chain(&manifest.produces)
                .cloned()
                .collect(),
            root,
            manifest,
            write_claims,
            read_strictness: self.read_strictness,
        })
    }

    /// `catalog.get(name)?.resources?.(args)`(T2-4 の `catalog.rs` が宣言
    /// 導出の所有者 — T2-7 で本 module の二重実装を除去し委譲へ統合)。
    /// 返る typed declaration を `normalize_resource_manifest` へ渡す
    /// untyped JSON へ写す。全 tool が `resources` を持つため `get`
    /// 失敗経路は存在しない(`ToolName` は closed enum)。
    fn catalog_resource_declaration(
        &self,
        name: ToolName,
        input: &Value,
    ) -> Result<Value, ToolError> {
        let metadata = self
            .options
            .catalog
            .get(name.as_str())
            .expect("ToolName は closed enum — 全 tool に catalog metadata がある");
        let declaration = (metadata.resources)(input)?;
        Ok(manifest_declaration_value(&declaration))
    }

    /// `operation(name, input)`。
    pub fn operation(&self, name: ToolName, input: &Value) -> Result<LeasePreparation, ToolError> {
        self.operation_inner(name, input).map_err(rethrow_as_denied)
    }

    fn operation_inner(
        &self,
        name: ToolName,
        input: &Value,
    ) -> Result<LeasePreparation, ToolError> {
        let scope = self.prepare()?;
        let mut args = input.clone();
        if let Some(path) = input.get("path").and_then(Value::as_str) {
            let resolved = resolve_workspace_path(&scope.root, path, self.options.symlink_policy)
                .map_err(|e| Box::new(e) as ToolError)?;
            // `{ ...input, path: resolved.display }`(path は display に差替)。
            let mut map = input
                .as_object()
                .expect("path を持つ input は object")
                .clone();
            map.insert("path".to_string(), Value::String(resolved.display.clone()));
            args = Value::Object(map);
            if self
                .options
                .denylist
                .as_ref()
                .is_some_and(|d| d.is_denied(&resolved.display))
            {
                return Err(Box::new(DeniedError::policy()));
            }
        }
        let declaration = self.catalog_resource_declaration(name, &args)?;
        let mut manifest = normalize_resource_manifest(
            &scope.root,
            Some(&declaration),
            self.options.symlink_policy,
        )
        .map_err(manifest_error)?;
        // subtree search が symlink を辿る時は start directory 外へ読み得る。
        if self.options.symlink_policy == SymlinkPolicy::WithinWorkspace
            && matches!(name, ToolName::Find | ToolName::Grep)
        {
            manifest.reads = vec![ResourceClaim {
                resource: Resource::workspace(scope.root.clone(), ResourceScope::Subtree),
                mode: ClaimMode::Shared,
            }];
        }
        if manifest
            .writes
            .iter()
            .chain(&manifest.produces)
            .any(|claim| {
                !contains_iter(
                    scope
                        .manifest
                        .writes
                        .iter()
                        .chain(scope.manifest.produces.iter()),
                    claim,
                ) || scope
                    .write_claims
                    .as_ref()
                    .is_some_and(|writes| !contains(writes, claim))
            })
        {
            return Err(Box::new(DeniedError::scope()));
        }
        Ok(LeasePreparation {
            claims: all_claims(&manifest),
            read_strictness: self.read_strictness,
        })
    }

    /// `observe(inputPath, mode, resourceScope = "exact", active?, support = false)`。
    pub fn observe(
        &self,
        input_path: &str,
        mode: ClaimMode,
        resource_scope: Option<ResourceScope>,
        active: Option<&ActiveResourceAccess>,
        support: bool,
    ) -> Result<(), ToolError> {
        let prepared = self.prepare()?;
        let resolved =
            resolve_workspace_path(&prepared.root, input_path, self.options.symlink_policy)
                .map_err(|e| rethrow_as_denied(Box::new(e)))?;
        if self
            .options
            .denylist
            .as_ref()
            .is_some_and(|d| d.is_denied(&resolved.display))
        {
            return Err(Box::new(DeniedError::policy()));
        }
        let claim = ResourceClaim {
            resource: Resource::workspace(
                resolved.real.clone(),
                resource_scope.unwrap_or(ResourceScope::Exact),
            ),
            mode,
        };
        let declared = match mode {
            ClaimMode::Exclusive => contains_iter(
                prepared
                    .manifest
                    .writes
                    .iter()
                    .chain(prepared.manifest.produces.iter()),
                &claim,
            ),
            ClaimMode::Shared => contains(&prepared.manifest.reads, &claim),
        };
        let support_in =
            |claims: &[ResourceClaim]| is_support_directory(&prepared.root, &claim, claims);
        let authorized_writes: Vec<ResourceClaim> = prepared
            .manifest
            .writes
            .iter()
            .chain(&prepared.manifest.produces)
            .cloned()
            .collect();
        let job_allowed = match mode {
            ClaimMode::Shared => declared || self.read_strictness == ReadStrictness::Record,
            ClaimMode::Exclusive => {
                (declared
                    && prepared
                        .write_claims
                        .as_ref()
                        .is_none_or(|writes| contains(writes, &claim)))
                    || (support
                        && support_in(&authorized_writes)
                        && prepared
                            .write_claims
                            .as_ref()
                            .is_none_or(|writes| support_in(writes)))
            }
        };
        let operation_allowed = active.is_none_or(|active| {
            contains(&active.claims, &claim) || (support && support_in(&active.claims))
        });
        let allowed = job_allowed && operation_allowed;
        {
            let mut observed = self.observed.lock().expect("access rows poisoned");
            observed.total += 1;
            if observed.rows.len() < MAX_ACCESS_OBSERVATIONS {
                observed.rows.push(AccessObservation {
                    operation_id: active.map(|a| a.id.clone()),
                    path: resolved.display,
                    claim,
                    declared,
                    allowed,
                    support,
                });
            }
        }
        if !allowed {
            return Err(Box::new(DeniedError::scope()));
        }
        Ok(())
    }

    /// `snapshot()`(row の deep copy 込み)。
    pub fn snapshot(&self) -> AccessSnapshot {
        let observed = self.observed.lock().expect("access rows poisoned");
        AccessSnapshot {
            total: observed.total,
            dropped: observed.total - observed.rows.len(),
            observations: observed.rows.clone(),
        }
    }
}

/// mkdir は認可済み target の canonical 祖先と既存 root だけを support する。
pub fn is_support_directory(
    root: &str,
    directory: &ResourceClaim,
    claims: &[ResourceClaim],
) -> bool {
    if directory.resource.path() == root {
        return true;
    }
    let subtree = Resource::workspace(directory.resource.path(), ResourceScope::Subtree);
    claims.iter().any(|claim| {
        claim.mode == ClaimMode::Exclusive
            && claim_contains(
                &ResourceClaim {
                    resource: subtree.clone(),
                    mode: ClaimMode::Exclusive,
                },
                claim,
            )
    })
}

#[cfg(test)]
mod tests {
    use cheaper_core::errors::{CheaperError, ErrorCode};
    use cheaper_core::workspace::deny_paths::create_path_denylist;
    use serde_json::json;

    use super::*;
    use crate::walker::{ACCESS_DENIED_MESSAGE, POLICY_DENIED_MESSAGE, SCOPE_DENIED_MESSAGE};

    /// `mkdtemp` 相当: `temp_dir()/cheaper-scope-<rand>` を作成し、
    /// canonical root を返す。
    fn tmpdir() -> String {
        let dir = std::env::temp_dir().join(format!("cheaper-scope-{}", fastrand::u64(..)));
        std::fs::create_dir_all(&dir).expect("tmpdir");
        canonicalize_workspace_root(dir.to_str().unwrap()).unwrap()
    }

    fn real_of(root: &str, rel: &str) -> String {
        resolve_workspace_path(root, rel, SymlinkPolicy::Deny)
            .unwrap()
            .real
    }

    fn opts(root: &str, names: Vec<ToolName>) -> ScopeAccessOptions {
        ScopeAccessOptions {
            scope: ResourceScopeOptions::default(),
            root: root.to_string(),
            symlink_policy: SymlinkPolicy::Deny,
            names,
            denylist: None,
            catalog: crate::catalog::create_tool_catalog(None).unwrap(),
        }
    }

    /// default: readStrictness="record"、names=[read] で root subtree の
    /// shared read claim が manifest に乗る。explicit=false。
    #[test]
    fn scope_prepare_defaults() {
        let root = tmpdir();
        let access = ResourceAccess::new(opts(&root, vec![ToolName::Read]), None).unwrap();
        assert_eq!(access.read_strictness, ReadStrictness::Record);
        assert!(!access.explicit);
        let prepared = access.prepare().unwrap();
        assert_eq!(
            prepared.manifest.reads,
            vec![ResourceClaim {
                resource: Resource::workspace(root.clone(), ResourceScope::Subtree),
                mode: ClaimMode::Shared
            }]
        );
        assert!(prepared.manifest.writes.is_empty());
        assert!(prepared.manifest.produces.is_empty());
        assert!(prepared.write_claims.is_none());
        // claims = reads(override)+writes+produces。
        assert_eq!(prepared.claims, prepared.manifest.reads);
        // prepare() は memoized — 同一 object を返す。
        assert!(std::ptr::eq(prepared, access.prepare().unwrap()));
    }

    /// mutating tool を含む names の default manifest は root subtree の
    /// exclusive write claim(writeScope 未指定時)。
    #[test]
    fn scope_prepare_mutating_default_write_claim() {
        let root = tmpdir();
        let access =
            ResourceAccess::new(opts(&root, vec![ToolName::Read, ToolName::Write]), None).unwrap();
        let prepared = access.prepare().unwrap();
        assert_eq!(
            prepared.manifest.writes,
            vec![ResourceClaim {
                resource: Resource::workspace(root, ResourceScope::Subtree),
                mode: ClaimMode::Exclusive
            }]
        );
    }

    /// writeScope → literal writeClaims(dedup + exact exclusive)。manifest
    /// 未指定 + mutating tool 時は manifest.writes が writeClaims に
    /// narrow される。explicit=true。
    #[test]
    fn scope_prepare_write_scope_derivation() {
        let root = tmpdir();
        let mut o = opts(&root, vec![ToolName::Write]);
        o.scope.write_scope = Some(vec!["out.txt".into(), "out.txt".into()]);
        let access = ResourceAccess::new(o, None).unwrap();
        assert!(access.explicit);
        let prepared = access.prepare().unwrap();
        let expected = vec![ResourceClaim {
            resource: Resource::workspace(real_of(&root, "out.txt"), ResourceScope::Exact),
            mode: ClaimMode::Exclusive,
        }];
        assert_eq!(prepared.write_claims.as_deref(), Some(expected.as_slice()));
        assert_eq!(prepared.manifest.writes, expected);
        assert_eq!(
            prepared.claims.len(),
            prepared.manifest.reads.len() + 1 // reads + writes + produces
        );
    }

    /// manifest.writes/produces が writeClaims を超えると invalid resource。
    #[test]
    fn scope_prepare_manifest_writes_exceeding_scope_rejected() {
        let root = tmpdir();
        let mut o = opts(&root, vec![ToolName::Write]);
        o.scope.write_scope = Some(vec!["a.txt".into()]);
        o.scope.resource_manifest = Some(json!({
            "writes": [{"path": "b.txt", "scope": "exact"}]
        }));
        let access = ResourceAccess::new(o, None).unwrap();
        let e = access.prepare().unwrap_err();
        let err = e.downcast_ref::<CheaperError>().expect("CheaperError");
        assert_eq!(err.code(), ErrorCode::InvalidRequest);
        assert_eq!(
            e.to_string(),
            "invalid resource lease: manifest writes exceed writeScope"
        );
    }

    /// parent read strictness mismatch は ctor で invalid resource。
    /// inherited 指定時に option 側が未指定なら inherited を継承する。
    #[test]
    fn scope_ctor_read_strictness_inheritance() {
        let root = tmpdir();
        let mut o = opts(&root, vec![ToolName::Read]);
        o.scope.read_strictness = Some(ReadStrictness::Record);
        let e = ResourceAccess::new(o, Some(ReadStrictness::Reject))
            .err()
            .unwrap();
        assert_eq!(
            e.to_string(),
            "invalid resource lease: parent read strictness mismatch"
        );
        let access = ResourceAccess::new(
            opts(&root, vec![ToolName::Read]),
            Some(ReadStrictness::Reject),
        )
        .unwrap();
        assert_eq!(access.read_strictness, ReadStrictness::Reject);
    }

    /// record: 未宣言 read は allowed=true で観測される(declared=false)。
    /// reject: 未宣言 read は scope denied で、denied 行も観測される。
    #[test]
    fn scope_observe_read_strictness_record_vs_reject() {
        let root = tmpdir();
        std::fs::write(real_of(&root, "f.txt"), "x").unwrap();
        let manifest = json!({"reads": [], "writes": [{"path": "w.txt", "scope": "exact"}]});
        let scope_writes = vec!["w.txt".to_string()];

        for (strictness, denied) in [
            (ReadStrictness::Record, false),
            (ReadStrictness::Reject, true),
        ] {
            let mut o = opts(&root, vec![ToolName::Write]);
            o.scope.read_strictness = Some(strictness);
            o.scope.write_scope = Some(scope_writes.clone());
            o.scope.resource_manifest = Some(manifest.clone());
            let access = ResourceAccess::new(o, None).unwrap();
            let result = access.observe("f.txt", ClaimMode::Shared, None, None, false);
            let rows = access.snapshot();
            if denied {
                let e = result.unwrap_err();
                assert_eq!(e.to_string(), SCOPE_DENIED_MESSAGE);
                assert!(!rows.observations[0].allowed);
            } else {
                result.unwrap();
                assert!(rows.observations[0].allowed);
            }
            assert_eq!(rows.observations[0].path, "f.txt");
            assert!(!rows.observations[0].declared);
            assert!(!rows.observations[0].support);
            assert!(rows.observations[0].operation_id.is_none());
        }
    }

    /// writeScope 内の write は declared+allowed、外は scope denied。
    /// denied 行も error 返却前に観測される。
    #[test]
    fn scope_observe_write_within_and_outside_scope() {
        let root = tmpdir();
        let mut o = opts(&root, vec![ToolName::Write]);
        o.scope.write_scope = Some(vec!["out.txt".into()]);
        let access = ResourceAccess::new(o, None).unwrap();

        access
            .observe("out.txt", ClaimMode::Exclusive, None, None, false)
            .unwrap();
        let e = access
            .observe("other.txt", ClaimMode::Exclusive, None, None, false)
            .unwrap_err();
        assert_eq!(e.to_string(), SCOPE_DENIED_MESSAGE);
        let rows = access.snapshot().observations;
        assert_eq!(rows.len(), 2);
        assert!(rows[0].declared && rows[0].allowed);
        assert!(!rows[1].declared && !rows[1].allowed);
    }

    /// mkdir support: 認可済み write target の canonical 祖先 directory は
    /// support=true で allowed。root 自身も常に support。
    #[test]
    fn scope_observe_support_directories() {
        let root = tmpdir();
        let mut o = opts(&root, vec![ToolName::Write]);
        o.scope.write_scope = Some(vec!["sub/new.txt".into()]);
        let access = ResourceAccess::new(o, None).unwrap();

        // write target の親 dir → support mkdir allowed。
        access
            .observe("sub", ClaimMode::Exclusive, None, None, true)
            .unwrap();
        // support=false なら同じ dir でも denied。
        let e = access
            .observe("sub", ClaimMode::Exclusive, None, None, false)
            .unwrap_err();
        assert_eq!(e.to_string(), SCOPE_DENIED_MESSAGE);
        // root 自身は writeClaim と無関係に常に support。
        access
            .observe(".", ClaimMode::Exclusive, None, None, true)
            .unwrap();
        // 関係ない dir は denied。
        let e = access
            .observe("elsewhere", ClaimMode::Exclusive, None, None, true)
            .unwrap_err();
        assert_eq!(e.to_string(), SCOPE_DENIED_MESSAGE);
        let rows = access.snapshot().observations;
        assert!(rows[0].support);
        assert!(rows[0].allowed);
    }

    /// denylist path → POLICY_DENIED、workspace escape → ACCESS_DENIED。
    #[test]
    fn scope_observe_denied_and_escape() {
        let root = tmpdir();
        let mut o = opts(&root, vec![ToolName::Read]);
        o.denylist = Some(create_path_denylist(&["secret.txt"]));
        let access = ResourceAccess::new(o, None).unwrap();
        let e = access
            .observe("secret.txt", ClaimMode::Shared, None, None, false)
            .unwrap_err();
        assert_eq!(e.to_string(), POLICY_DENIED_MESSAGE);
        let e = access
            .observe("../escape.txt", ClaimMode::Shared, None, None, false)
            .unwrap_err();
        assert_eq!(e.to_string(), ACCESS_DENIED_MESSAGE);
        // denylist/escape は観測行を積まない(error が先に伝播)。
        assert_eq!(access.snapshot().total, 0);
    }

    /// active operation claims: jobAllowed でも operation claims 外は
    /// denied。operationId が観測行に載る。
    #[test]
    fn scope_observe_active_operation_claims() {
        let root = tmpdir();
        let access = ResourceAccess::new(opts(&root, vec![ToolName::Read]), None).unwrap();
        let narrow = ActiveResourceAccess {
            id: "op-1".into(),
            claims: vec![ResourceClaim {
                resource: Resource::workspace(real_of(&root, "a.txt"), ResourceScope::Exact),
                mode: ClaimMode::Exclusive,
            }],
        };
        // jobAllowed(record)でも active claims 外 → denied + operationId。
        let e = access
            .observe("b.txt", ClaimMode::Shared, None, Some(&narrow), false)
            .unwrap_err();
        assert_eq!(e.to_string(), SCOPE_DENIED_MESSAGE);
        assert_eq!(
            access.snapshot().observations[0].operation_id.as_deref(),
            Some("op-1")
        );
        // root subtree exclusive claim は任意の read を包含 → allowed。
        let broad = ActiveResourceAccess {
            id: "op-2".into(),
            claims: vec![ResourceClaim {
                resource: Resource::workspace(root.clone(), ResourceScope::Subtree),
                mode: ClaimMode::Exclusive,
            }],
        };
        access
            .observe("b.txt", ClaimMode::Shared, None, Some(&broad), false)
            .unwrap();
    }

    /// `MAX_ACCESS_OBSERVATIONS` まで行を保持、超過分は dropped だけ増える。
    #[test]
    fn scope_snapshot_observation_cap() {
        let root = tmpdir();
        let access = ResourceAccess::new(opts(&root, vec![ToolName::Read]), None).unwrap();
        for _ in 0..MAX_ACCESS_OBSERVATIONS + 7 {
            access
                .observe("f.txt", ClaimMode::Shared, None, None, false)
                .unwrap();
        }
        let snap = access.snapshot();
        assert_eq!(snap.total, MAX_ACCESS_OBSERVATIONS + 7);
        assert_eq!(snap.observations.len(), MAX_ACCESS_OBSERVATIONS);
        assert_eq!(snap.dropped, 7);
    }

    /// `operation()` は catalog 宣言を normalized claims に変える。
    /// write op が writeScope を超えると scope denied。
    #[test]
    fn scope_operation_claims_and_scope_denial() {
        let root = tmpdir();
        let mut o = opts(&root, vec![ToolName::Write]);
        o.scope.write_scope = Some(vec!["out.txt".into()]);
        let access = ResourceAccess::new(o, None).unwrap();

        let prep = access
            .operation(ToolName::Write, &json!({"path": "out.txt"}))
            .unwrap();
        assert_eq!(
            prep.claims,
            vec![ResourceClaim {
                resource: Resource::workspace(real_of(&root, "out.txt"), ResourceScope::Exact),
                mode: ClaimMode::Exclusive
            }]
        );
        let e = access
            .operation(ToolName::Write, &json!({"path": "other.txt"}))
            .unwrap_err();
        assert_eq!(e.to_string(), SCOPE_DENIED_MESSAGE);
    }

    /// `operation()` の path 境界: denylist → POLICY_DENIED、escape →
    /// ACCESS_DENIED。path 不明(未知)の read は空 claims。
    #[test]
    fn scope_operation_path_boundary() {
        let root = tmpdir();
        let mut o = opts(&root, vec![ToolName::Read]);
        o.denylist = Some(create_path_denylist(&["secret.txt"]));
        let access = ResourceAccess::new(o, None).unwrap();
        let e = access
            .operation(ToolName::Read, &json!({"path": "secret.txt"}))
            .unwrap_err();
        assert_eq!(e.to_string(), POLICY_DENIED_MESSAGE);
        let e = access
            .operation(ToolName::Read, &json!({"path": "../outside"}))
            .unwrap_err();
        assert_eq!(e.to_string(), ACCESS_DENIED_MESSAGE);
        // path 未指定の read → declaration reads=null(未知)→ normalize が
        // root subtree shared claim に broaden する(D5: 未知は空ではなく
        // workspace 全体の read として扱う)。
        let prep = access.operation(ToolName::Read, &json!({})).unwrap();
        assert_eq!(
            prep.claims,
            vec![ResourceClaim {
                resource: Resource::workspace(root.clone(), ResourceScope::Subtree),
                mode: ClaimMode::Shared
            }]
        );
    }

    /// within-workspace で find/grep の reads は root subtree に widen
    /// される(symlink traversal 用)。deny では宣言 path のまま。
    #[test]
    fn scope_operation_find_root_widening() {
        let root = tmpdir();
        let mut o = opts(&root, vec![ToolName::Find]);
        o.symlink_policy = SymlinkPolicy::WithinWorkspace;
        let access = ResourceAccess::new(o, None).unwrap();
        let prep = access
            .operation(ToolName::Find, &json!({"path": "sub"}))
            .unwrap();
        assert_eq!(
            prep.claims,
            vec![ResourceClaim {
                resource: Resource::workspace(root.clone(), ResourceScope::Subtree),
                mode: ClaimMode::Shared
            }]
        );
        let access = ResourceAccess::new(opts(&root, vec![ToolName::Find]), None).unwrap();
        let prep = access
            .operation(ToolName::Find, &json!({"path": "sub"}))
            .unwrap();
        assert_eq!(
            prep.claims,
            vec![ResourceClaim {
                resource: Resource::workspace(real_of(&root, "sub"), ResourceScope::Subtree),
                mode: ClaimMode::Shared
            }]
        );
    }

    /// `prepareWorkerManifest` は writeClaims を safety manifest の
    /// writes/produces bucket に narrow する。
    #[test]
    fn scope_prepare_worker_manifest_narrowing() {
        let root = tmpdir();
        let mut o = opts(&root, vec![ToolName::Write]);
        o.scope.write_scope = Some(vec!["a.txt".into(), "b.txt".into()]);
        let access = ResourceAccess::new(o, None).unwrap();
        let safety = json!({
            "reads": [],
            "writes": [{"path": "a.txt", "scope": "exact"}],
            "produces": [{"path": "b.txt", "scope": "exact"}]
        });
        let manifest = access.prepare_worker_manifest(Some(&safety)).unwrap();
        assert_eq!(
            manifest.writes,
            vec![ResourceClaim {
                resource: Resource::workspace(real_of(&root, "a.txt"), ResourceScope::Exact),
                mode: ClaimMode::Exclusive
            }]
        );
        assert_eq!(
            manifest.produces,
            vec![ResourceClaim {
                resource: Resource::workspace(real_of(&root, "b.txt"), ResourceScope::Exact),
                mode: ClaimMode::Exclusive
            }]
        );
        // writeScope 未指定時は safety をそのまま返す。
        let access = ResourceAccess::new(opts(&root, vec![ToolName::Read]), None).unwrap();
        let manifest = access.prepare_worker_manifest(Some(&safety)).unwrap();
        assert_eq!(manifest.writes.len(), 1);
        assert_eq!(manifest.produces.len(), 1);
    }

    /// `isSupportDirectory` の直接 pin。
    #[test]
    fn scope_is_support_directory() {
        let root = tmpdir();
        let dir = ResourceClaim {
            resource: Resource::workspace(real_of(&root, "sub"), ResourceScope::Exact),
            mode: ClaimMode::Exclusive,
        };
        // root 自身 → true。
        let root_dir = ResourceClaim {
            resource: Resource::workspace(root.clone(), ResourceScope::Exact),
            mode: ClaimMode::Exclusive,
        };
        assert!(is_support_directory(&root, &root_dir, &[]));
        // claim 無し → false。
        assert!(!is_support_directory(&root, &dir, &[]));
        // dir 内への exclusive claim → true。
        let inner = ResourceClaim {
            resource: Resource::workspace(real_of(&root, "sub/f.txt"), ResourceScope::Exact),
            mode: ClaimMode::Exclusive,
        };
        assert!(is_support_directory(
            &root,
            &dir,
            std::slice::from_ref(&inner)
        ));
        // shared claim は support 根拠にならない。
        let shared_inner = ResourceClaim {
            mode: ClaimMode::Shared,
            ..inner
        };
        assert!(!is_support_directory(&root, &dir, &[shared_inner]));
        // 無関係な exclusive → false。
        let other = ResourceClaim {
            resource: Resource::workspace(real_of(&root, "other/f.txt"), ResourceScope::Exact),
            mode: ClaimMode::Exclusive,
        };
        assert!(!is_support_directory(&root, &dir, &[other]));
    }
}
