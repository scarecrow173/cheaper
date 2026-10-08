//! Port of `packages/workflow/src/resource-inference.ts`.

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::js::code_unit_compare;
use cheaper_core::resources::normalization::{
    normalize_resource_claim, normalize_resource_manifest, ResourceManifestDeclaration,
    ScopeDeclaration,
};
use cheaper_core::resources::predicates::{
    claim_contains, claims_conflict, resource_key, resources_overlap,
};
use cheaper_core::resources::types::{ClaimMode, ResourceClaim, ResourceManifest, ResourceScope};
use cheaper_core::worker::validate::is_mutating_tool;
use cheaper_core::workspace::paths::SymlinkPolicy;
use cheaper_tools::ToolError;
use serde_json::{json, Map, Value};

use crate::types::{
    DependencyEdge, JobKind, ResourceConflict, ResourceInference, ResourceInferenceDiagnostic,
    ResourceInferenceJob, ResourceInferenceResult, ScopePath, ToolCatalog, WorkflowResources,
};

fn invalid(message: impl Into<String>) -> ToolError {
    Box::new(CheaperError::new(ErrorCode::InvalidRequest, message.into()))
}

/// `claimKey(claim)` = `JSON.stringify([resourceKey, mode])`。
fn claim_key(claim: &ResourceClaim) -> String {
    json!([resource_key(&claim.resource), claim.mode.as_str()]).to_string()
}

/// `claims(manifest)` — reads+writes+produces を claimKey code-unit 順で
/// sort + adjacent dedupe。
fn claims(manifest: &ResourceManifest) -> Vec<ResourceClaim> {
    let mut all: Vec<ResourceClaim> = manifest
        .reads
        .iter()
        .chain(manifest.writes.iter())
        .chain(manifest.produces.iter())
        .cloned()
        .collect();
    all.sort_by(|a, b| code_unit_compare(&claim_key(a), &claim_key(b)));
    all.dedup_by(|a, b| claim_key(a) == claim_key(b));
    all
}

/// `object(value)`。
fn object(value: &Value) -> bool {
    value.is_object()
}

/// `deferred(value)`: object で `$from` own key を持つ。
fn deferred(value: Option<&Value>) -> bool {
    value.is_some_and(|v| v.as_object().is_some_and(|o| o.contains_key("$from")))
}

/// `ResourceManifestDeclaration` → normalize 入力 JSON(`undefined`
/// bucket は key 自体を省略)。
fn declaration_value(declaration: &ResourceManifestDeclaration) -> Value {
    let mut out = Map::new();
    for (key, bucket) in [
        ("reads", &declaration.reads),
        ("writes", &declaration.writes),
        ("produces", &declaration.produces),
    ] {
        if let Some(bucket) = bucket {
            out.insert(
                key.to_string(),
                Value::Array(
                    bucket
                        .iter()
                        .map(|s| json!({ "path": s.path, "scope": s.scope }))
                        .collect(),
                ),
            );
        }
    }
    Value::Object(out)
}

/// `ResourceManifest` → `{...manifest}` spread 相当の JSON。
fn manifest_value(manifest: &ResourceManifest) -> Value {
    serde_json::to_value(manifest).unwrap_or(Value::Null)
}

/// `Source`。
struct Source {
    known_process: bool,
    declaration: ResourceManifestDeclaration,
    authority: Option<Vec<ResourceClaim>>,
    unknown_authority: bool,
    ancestor_mkdir: bool,
}

/// `WorkspaceResourceInference`。safety claim と既知 read を分離し、
/// 未知 fallback から順序を発明しない。
pub struct WorkspaceResourceInference<'a> {
    /// `Options.workspaceRoot` — canonical 化は normalize 側が担う。
    pub workspace_root: String,
    pub symlink_policy: SymlinkPolicy,
    /// `Options.tools`(`ToolCatalog` port — `resources` callback を持つ
    /// metadata を返す)。
    pub tools: &'a dyn ToolCatalog,
}

impl WorkspaceResourceInference<'_> {
    /// `this.normalize(declaration)`。
    fn normalize(
        &self,
        declaration: Option<&ResourceManifestDeclaration>,
    ) -> Result<ResourceManifest, ToolError> {
        let value = declaration.map(declaration_value);
        normalize_resource_manifest(&self.workspace_root, value.as_ref(), self.symlink_policy)
            .map_err(|e| Box::new(e) as ToolError)
    }

    /// `source(item)`。
    async fn source(&self, item: &ResourceInferenceJob) -> Result<Source, ToolError> {
        let process_job = item.job.kind == JobKind::Tool && item.job.name == "process";
        if item.process.is_some() && !process_job {
            return Err(invalid("process declaration requires a process job"));
        }
        if process_job {
            let input = &item.job.input;
            // `!object(input) || typeof input.task !== "string" ||
            //  Object.keys(input).some(key => key !== "task")`
            let named_task = object(input)
                && matches!(input.get("task"), Some(Value::String(_)))
                && input
                    .as_object()
                    .is_some_and(|o| o.keys().all(|k| k == "task"));
            if !named_task {
                return Err(invalid("process requires a named task input"));
            }
            let task = input["task"].as_str().expect("checked").to_string();
            match &item.process {
                None => {
                    let resources = self.tools.get("process").and_then(|meta| meta.resources);
                    let Some(resources) = resources else {
                        return Err(invalid("process requires a policy-aware tool catalog"));
                    };
                    // `resources(structuredClone(input))`
                    let resolved = resources(&input.clone())?;
                    if resolved.reads.is_none()
                        || resolved.writes.is_none()
                        || resolved.produces.is_none()
                    {
                        return Err(invalid("process requires known task inputs and outputs"));
                    }
                    let authority = self.normalize(Some(&resolved))?.produces;
                    Ok(Source {
                        known_process: true,
                        declaration: resolved,
                        authority: Some(authority),
                        unknown_authority: false,
                        ancestor_mkdir: false,
                    })
                }
                Some(declaration) => {
                    // `declaration.task !== input.task ||
                    //  typeof declaration.task !== "string" ||
                    //  declaration.task.length === 0 ||
                    //  !Array.isArray(declaration.inputs) ||
                    //  !Array.isArray(declaration.outputs)`
                    // (Rust 型では inputs/outputs は常に配列)。
                    if declaration.task != task || declaration.task.is_empty() {
                        return Err(invalid("process requires declared task inputs and outputs"));
                    }
                    let merged = ResourceManifestDeclaration {
                        reads: Some(declaration.inputs.clone()),
                        writes: Some(declaration.outputs.clone()),
                        produces: Some(declaration.outputs.clone()),
                    };
                    let authority = self.normalize(Some(&merged))?.produces;
                    Ok(Source {
                        known_process: true,
                        declaration: merged,
                        authority: Some(authority),
                        unknown_authority: false,
                        ancestor_mkdir: false,
                    })
                }
            }
        } else if item.job.kind == JobKind::Tool {
            // `metadata?.resources?.(structuredClone(item.job.input)) ?? {}`
            let declaration = match self.tools.get(&item.job.name) {
                Some(meta) => match meta.resources {
                    Some(resources) => resources(&item.job.input.clone())?,
                    None => ResourceManifestDeclaration::default(),
                },
                None => ResourceManifestDeclaration::default(),
            };
            Ok(Source {
                known_process: false,
                declaration,
                authority: None,
                unknown_authority: false,
                ancestor_mkdir: false,
            })
        } else {
            let Some(worker) = &item.worker else {
                return Err(invalid(
                    "worker resource inference requires worker metadata",
                ));
            };
            let mutation = worker.tools.iter().any(|t| is_mutating_tool(*t));
            if !mutation {
                return Ok(Source {
                    known_process: false,
                    declaration: ResourceManifestDeclaration {
                        reads: None,
                        writes: Some(Vec::new()),
                        produces: Some(Vec::new()),
                    },
                    authority: Some(Vec::new()),
                    unknown_authority: false,
                    ancestor_mkdir: false,
                });
            }
            let Some(field) = worker.cheaper.write_scope.clone() else {
                return Ok(Source {
                    known_process: false,
                    declaration: ResourceManifestDeclaration::default(),
                    authority: None,
                    unknown_authority: true,
                    ancestor_mkdir: false,
                });
            };
            let value = item.job.input.as_object().and_then(|o| o.get(&field));
            if deferred(value) {
                return Ok(Source {
                    known_process: false,
                    declaration: ResourceManifestDeclaration::default(),
                    authority: None,
                    unknown_authority: true,
                    ancestor_mkdir: false,
                });
            }
            let Some(values) = value.and_then(Value::as_array) else {
                return Err(Box::new(CheaperError::new(
                    ErrorCode::InvalidInput,
                    "writeScope must contain targets",
                )));
            };
            let mut targets: Vec<ScopeDeclaration> = Vec::new();
            let mut unknown = false;
            for target in values {
                // `deferred(target) || (object(target) && deferred(target.path))`
                if deferred(Some(target)) || (object(target) && deferred(target.get("path"))) {
                    unknown = true;
                    continue;
                }
                let target_path = match target {
                    Value::String(s) => Some(s.as_str()),
                    Value::Object(_) => target.get("path").and_then(Value::as_str),
                    _ => None,
                };
                let Some(path) = target_path.filter(|p| !p.is_empty()) else {
                    return Err(Box::new(CheaperError::new(
                        ErrorCode::InvalidInput,
                        "writeScope target must be a path",
                    )));
                };
                targets.push(ScopeDeclaration {
                    path: path.to_string(),
                    scope: ResourceScope::Exact,
                });
            }
            // authority は ownership 正規化で exact literal を検査する。
            let authority = self
                .normalize(Some(&ResourceManifestDeclaration {
                    reads: Some(Vec::new()),
                    writes: Some(Vec::new()),
                    produces: Some(targets.clone()),
                }))?
                .produces;
            if unknown {
                return Ok(Source {
                    known_process: false,
                    declaration: ResourceManifestDeclaration::default(),
                    authority: None,
                    unknown_authority: true,
                    ancestor_mkdir: false,
                });
            }
            Ok(Source {
                known_process: false,
                declaration: ResourceManifestDeclaration {
                    reads: None,
                    writes: Some(targets.clone()),
                    produces: None,
                },
                authority: Some(authority),
                unknown_authority: false,
                ancestor_mkdir: !targets.is_empty(),
            })
        }
    }
}

/// workflow `resources`(materialize 済み前提)→ manifest declaration。
/// `path` がまだ `{$input}` のままなら `invalid("resource path must be
/// materialized")` を caller が先に返す。
fn declared_of(resources: &WorkflowResources) -> ResourceManifestDeclaration {
    let bucket = |b: &Option<Vec<crate::types::ScopeDeclaration>>| {
        b.as_ref().map(|decls| {
            decls
                .iter()
                .map(|d| ScopeDeclaration {
                    path: match &d.path {
                        ScopePath::Literal(p) => p.clone(),
                        // caller が事前に拒否済み。安全側として空文字は
                        // `validate_path` が拒否する形で流さない。
                        ScopePath::Input(_) => String::new(),
                    },
                    scope: d.scope,
                })
                .collect()
        })
    };
    ResourceManifestDeclaration {
        reads: bucket(&resources.reads),
        writes: bucket(&resources.writes),
        produces: bucket(&resources.produces),
    }
}

impl ResourceInference for WorkspaceResourceInference<'_> {
    fn infer<'a>(
        &'a self,
        jobs: &'a [(String, ResourceInferenceJob)],
    ) -> Pin<Box<dyn Future<Output = Result<ResourceInferenceResult, ToolError>> + Send + 'a>> {
        Box::pin(async move {
            let mut ids: Vec<&String> = jobs.iter().map(|(id, _)| id).collect();
            ids.sort_by(|a, b| code_unit_compare(a, b));
            let mut manifests: HashMap<&str, ResourceManifest> = HashMap::new();
            let mut dependency_reads: HashMap<&str, Vec<ResourceClaim>> = HashMap::new();
            let mut diagnostics: Vec<ResourceInferenceDiagnostic> = Vec::new();
            for id in &ids {
                let item = jobs
                    .iter()
                    .find(|(k, _)| k == *id)
                    .map(|(_, v)| v)
                    .expect("id from keys");
                let source = self.source(item).await?;
                // `known` — source.declaration は常に normalize する。
                let known = self.normalize(Some(&source.declaration))?;
                let explicit = &item.job.resources;
                // core に渡す前に未 materialize binding を拒否する。
                if let Some(explicit) = explicit {
                    for bucket in [
                        explicit.reads.as_deref(),
                        explicit.writes.as_deref(),
                        explicit.produces.as_deref(),
                    ]
                    .into_iter()
                    .flatten()
                    {
                        for declaration in bucket {
                            if matches!(declaration.path, ScopePath::Input(_)) {
                                return Err(invalid("resource path must be materialized"));
                            }
                        }
                    }
                }
                let declared = explicit.as_ref().map(declared_of);
                let omitted = declared.as_ref().is_none_or(|d| {
                    d.reads.is_none() && d.writes.is_none() && d.produces.is_none()
                });
                let merged;
                let normalize_input: Option<&ResourceManifestDeclaration> = if omitted {
                    if source.known_process {
                        Some(&source.declaration)
                    } else {
                        None
                    }
                } else {
                    // `{...source.declaration, ...declared}` — declared の
                    // present key が source を上書きする。
                    let d = declared.as_ref().expect("not omitted");
                    merged = ResourceManifestDeclaration {
                        reads: d.reads.clone().or_else(|| source.declaration.reads.clone()),
                        writes: d
                            .writes
                            .clone()
                            .or_else(|| source.declaration.writes.clone()),
                        produces: d
                            .produces
                            .clone()
                            .or_else(|| source.declaration.produces.clone()),
                    };
                    Some(&merged)
                };
                let mut manifest = self.normalize(normalize_input)?;
                for (bucket, known_claims, manifest_claims, declared_present) in [
                    (
                        "reads",
                        &known.reads,
                        &manifest.reads,
                        declared.as_ref().and_then(|d| d.reads.as_ref()).is_some(),
                    ),
                    (
                        "writes",
                        &known.writes,
                        &manifest.writes,
                        declared.as_ref().and_then(|d| d.writes.as_ref()).is_some(),
                    ),
                    (
                        "produces",
                        &known.produces,
                        &manifest.produces,
                        declared
                            .as_ref()
                            .and_then(|d| d.produces.as_ref())
                            .is_some(),
                    ),
                ] {
                    // `declared?.[bucket] !== undefined &&
                    //  source.declaration[bucket] != null && ...`
                    let source_bucket_present = match bucket {
                        "reads" => source.declaration.reads.is_some(),
                        "writes" => source.declaration.writes.is_some(),
                        _ => source.declaration.produces.is_some(),
                    };
                    if declared_present
                        && source_bucket_present
                        && known_claims.iter().any(|access| {
                            !manifest_claims
                                .iter()
                                .any(|scope| claim_contains(scope, access))
                        })
                    {
                        return Err(invalid(format!(
                            "declared {bucket} underclaims known access: {id}"
                        )));
                    }
                }
                if let Some(authority) = &source.authority {
                    let declared_writes_present =
                        declared.as_ref().and_then(|d| d.writes.as_ref()).is_some();
                    if declared_writes_present
                        && manifest.writes.iter().any(|access| {
                            !authority.iter().any(|scope| claim_contains(scope, access))
                        })
                    {
                        return Err(invalid(format!("writes exceed authoritative scope: {id}")));
                    }
                    if manifest
                        .produces
                        .iter()
                        .any(|access| !authority.iter().any(|scope| claim_contains(scope, access)))
                    {
                        return Err(invalid(format!(
                            "produces exceed authoritative scope: {id}"
                        )));
                    }
                }
                let declared_writes = declared.as_ref().and_then(|d| d.writes.as_ref());
                if source.unknown_authority
                    && ((declared_writes.is_some() && !manifest.writes.is_empty())
                        || !manifest.produces.is_empty()
                        || declared_writes.is_some_and(|w| w.is_empty()))
                {
                    return Err(invalid(format!(
                        "unknown writeScope cannot certify declared mutation: {id}"
                    )));
                }
                if source.ancestor_mkdir {
                    let safety = normalize_resource_claim(
                        &self.workspace_root,
                        &ScopeDeclaration {
                            path: ".".to_string(),
                            scope: ResourceScope::Subtree,
                        },
                        ClaimMode::Exclusive,
                        self.symlink_policy,
                    )
                    .map_err(|e| Box::new(e) as ToolError)?;
                    let mut value = manifest_value(&manifest);
                    let mut writes: Vec<Value> = manifest
                        .writes
                        .iter()
                        .map(|c| serde_json::to_value(c).unwrap_or(Value::Null))
                        .collect();
                    writes.push(serde_json::to_value(safety).unwrap_or(Value::Null));
                    value["writes"] = Value::Array(writes);
                    manifest.writes = normalize_resource_manifest(
                        &self.workspace_root,
                        Some(&value),
                        self.symlink_policy,
                    )
                    .map_err(|e| Box::new(e) as ToolError)?
                    .writes;
                    diagnostics.push(ResourceInferenceDiagnostic::Fallback {
                        job: (*id).clone(),
                        reason: "worker-ancestor-mkdir".to_string(),
                    });
                }
                manifests.insert(id.as_str(), manifest.clone());
                dependency_reads.insert(
                    id.as_str(),
                    if declared_writes_or(declared.as_ref(), "reads") {
                        manifest.reads.clone()
                    } else if source.declaration.reads.is_some() {
                        known.reads.clone()
                    } else {
                        Vec::new()
                    },
                );
                let declared_reads_absent =
                    declared.as_ref().and_then(|d| d.reads.as_ref()).is_none();
                let declared_writes_absent =
                    declared.as_ref().and_then(|d| d.writes.as_ref()).is_none();
                if (omitted && !source.known_process)
                    || (source.declaration.reads.is_none() && declared_reads_absent)
                    || (source.declaration.writes.is_none() && declared_writes_absent)
                {
                    diagnostics.push(ResourceInferenceDiagnostic::Fallback {
                        job: (*id).clone(),
                        reason: if omitted {
                            "resources-omitted"
                        } else {
                            "unknown-access"
                        }
                        .to_string(),
                    });
                }
            }
            let mut edge_seen: HashSet<(String, String)> = HashSet::new();
            let mut edges: Vec<DependencyEdge> = Vec::new();
            let mut conflicts: Vec<ResourceConflict> = Vec::new();
            for i in 0..ids.len() {
                for j in (i + 1)..ids.len() {
                    let a = ids[i].as_str();
                    let b = ids[j].as_str();
                    for left in &manifests[a].produces {
                        for right in &manifests[b].produces {
                            if resources_overlap(&left.resource, &right.resource) {
                                return Err(Box::new(
                                    CheaperError::new(
                                        ErrorCode::InvalidRequest,
                                        "duplicate resource producer",
                                    )
                                    .with_details({
                                        let mut details = Map::new();
                                        details.insert(
                                            "kind".to_string(),
                                            json!("duplicate-producer"),
                                        );
                                        details.insert("jobs".to_string(), json!([a, b]));
                                        details.insert(
                                            "claims".to_string(),
                                            serde_json::to_value([left, right])
                                                .unwrap_or(Value::Null),
                                        );
                                        details
                                    }),
                                ));
                            }
                        }
                    }
                    for left in claims(&manifests[a]) {
                        for right in claims(&manifests[b]) {
                            if claims_conflict(&left, &right) {
                                let conflict = ResourceConflict {
                                    jobs: [a.to_string(), b.to_string()],
                                    claims: [left.clone(), right.clone()],
                                };
                                conflicts.push(conflict.clone());
                                diagnostics.push(ResourceInferenceDiagnostic::Conflict(conflict));
                            }
                        }
                    }
                }
            }
            for from in &ids {
                for to in &ids {
                    if from == to {
                        continue;
                    }
                    for producer in &manifests[from.as_str()].produces {
                        for read in &dependency_reads[to.as_str()] {
                            if resources_overlap(&producer.resource, &read.resource) {
                                if edge_seen.insert(((*from).clone(), (*to).clone())) {
                                    edges.push(DependencyEdge {
                                        from: (*from).clone(),
                                        to: (*to).clone(),
                                    });
                                }
                                diagnostics.push(ResourceInferenceDiagnostic::Dependency(
                                    ResourceConflict {
                                        jobs: [(*from).clone(), (*to).clone()],
                                        claims: [producer.clone(), read.clone()],
                                    },
                                ));
                            }
                        }
                    }
                }
            }
            Ok(ResourceInferenceResult {
                manifests: ids
                    .iter()
                    .map(|id| ((*id).clone(), manifests[id.as_str()].clone()))
                    .collect(),
                edges,
                conflicts: Some(conflicts),
                diagnostics: Some(diagnostics),
            })
        })
    }
}

/// `declared?.reads !== undefined` 相当の bucket 参照 helper。
fn declared_writes_or(declared: Option<&ResourceManifestDeclaration>, bucket: &str) -> bool {
    match bucket {
        "reads" => declared.and_then(|d| d.reads.as_ref()).is_some(),
        "writes" => declared.and_then(|d| d.writes.as_ref()).is_some(),
        _ => declared.and_then(|d| d.produces.as_ref()).is_some(),
    }
}
