//! Port of `packages/workflow/test/resource-inference.test.ts`。
//!
//! `expect.arrayContaining` は per-claim contains 検査、
//! `expect.objectContaining` on diagnostics は variant-aware helper で
//! 写す。`it.runIf(win32)` は `#[cfg]`/runtime split で移植する。

mod common;

use std::sync::Arc;

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::nodepath;
use cheaper_core::resources::normalization::ScopeDeclaration as CoreScopeDeclaration;
use cheaper_core::resources::types::{
    ClaimMode, Resource, ResourceClaim, ResourceManifest, ResourceScope,
};
use cheaper_core::tools::vocabulary::ToolName;
use cheaper_core::worker::schema::{compile_schema, SchemaKind};
use cheaper_core::workspace::paths::{realpath, SymlinkPolicy};
use cheaper_tools::ToolError;
use serde_json::{json, Map, Value};

use cheaper_workflow::{
    JobKind, ProcessResourceDeclaration, ResourceInference, ResourceInferenceDiagnostic,
    ResourceInferenceJob, ResourceInferenceResult, ScopeDeclaration, ScopePath, WorkflowJob,
    WorkflowResources, WorkspaceResourceInference,
};

use common::{fixture_worker, FixtureTools};

struct Workspace {
    _dir: tempfile::TempDir,
    root: String,
}

fn workspace() -> Workspace {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = realpath(&dir.path().to_string_lossy()).expect("realpath");
    Workspace { _dir: dir, root }
}

/// `symlink(target, link, "junction"|"dir")` 相当。
fn symlink_dir(target: &str, link: &str) {
    #[cfg(windows)]
    {
        let status = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J", link, target])
            .output()
            .expect("cmd mklink");
        assert!(status.status.success(), "mklink /J failed");
    }
    #[cfg(not(windows))]
    std::os::unix::fs::symlink(target, link).expect("symlink");
}

fn exact(path: &str) -> ScopeDeclaration {
    ScopeDeclaration {
        path: ScopePath::Literal(path.to_string()),
        scope: ResourceScope::Exact,
    }
}

fn subtree(path: &str) -> ScopeDeclaration {
    ScopeDeclaration {
        path: ScopePath::Literal(path.to_string()),
        scope: ResourceScope::Subtree,
    }
}

/// `process` declaration の `{path, scope}`(core 側 scope 型)。
fn core_exact(path: &str) -> CoreScopeDeclaration {
    CoreScopeDeclaration {
        path: path.to_string(),
        scope: ResourceScope::Exact,
    }
}

fn core_subtree(path: &str) -> CoreScopeDeclaration {
    CoreScopeDeclaration {
        path: path.to_string(),
        scope: ResourceScope::Subtree,
    }
}

/// `claim(resourcePath, mode, scope)` — `path.join(root, rel)`。
fn claim(root: &str, rel: &str, mode: ClaimMode, scope: ResourceScope) -> ResourceClaim {
    ResourceClaim {
        resource: Resource::workspace(nodepath::join(&[root, rel]), scope),
        mode,
    }
}

fn tool_job(
    name: &str,
    input: Value,
    resources: Option<WorkflowResources>,
) -> ResourceInferenceJob {
    ResourceInferenceJob {
        process: None,
        job: WorkflowJob {
            kind: JobKind::Tool,
            name: name.to_string(),
            input,
            needs: None,
            resources,
        },
        worker: None,
    }
}

fn declared(resources: WorkflowResources) -> ResourceInferenceJob {
    tool_job("grep", json!({}), Some(resources))
}

fn manifest<'a>(result: &'a ResourceInferenceResult, id: &str) -> &'a ResourceManifest {
    result
        .manifests
        .iter()
        .find(|(key, _)| key == id)
        .map(|(_, m)| m)
        .unwrap_or_else(|| panic!("missing manifest: {id}"))
}

fn manifest_keys(result: &ResourceInferenceResult) -> Vec<&str> {
    result.manifests.iter().map(|(k, _)| k.as_str()).collect()
}

fn err_details(e: &ToolError) -> Option<&Map<String, Value>> {
    e.downcast_ref::<CheaperError>().and_then(|c| c.details())
}

fn has_fallback(diagnostics: &[ResourceInferenceDiagnostic], job: &str, reason: &str) -> bool {
    diagnostics.iter().any(|d| {
        matches!(d, ResourceInferenceDiagnostic::Fallback { job: j, reason: r }
            if j == job && r == reason)
    })
}

fn has_dependency(
    diagnostics: &[ResourceInferenceDiagnostic],
    jobs: [&str; 2],
    claims: [ResourceClaim; 2],
) -> bool {
    diagnostics.iter().any(|d| {
        matches!(d, ResourceInferenceDiagnostic::Dependency(c)
            if c.jobs == [jobs[0].to_string(), jobs[1].to_string()]
                && c.claims == [claims[0].clone(), claims[1].clone()])
    })
}

fn default_manifest() -> ResourceManifest {
    ResourceManifest {
        reads: Vec::new(),
        writes: Vec::new(),
        produces: Vec::new(),
    }
}

/// `fixtures().worker` を TS test と同じく mutation して使う。
fn mutating_worker() -> cheaper_core::worker::validate::WorkerDefinition {
    let mut worker = fixture_worker();
    worker.tools = vec![ToolName::Read, ToolName::Write];
    worker.cheaper.write_scope = Some("targets".to_string());
    worker.input_schema = Some(Arc::new(
        compile_schema(Map::new(), SchemaKind::Input).expect("input schema compiles"),
    ));
    worker
}

fn worker_job(
    input: Value,
    resources: WorkflowResources,
    worker: cheaper_core::worker::validate::WorkerDefinition,
) -> ResourceInferenceJob {
    ResourceInferenceJob {
        process: None,
        job: WorkflowJob {
            kind: JobKind::Worker,
            name: worker.name.clone(),
            input,
            needs: None,
            resources: Some(resources),
        },
        worker: Some(Arc::new(worker)),
    }
}

/// `new WorkspaceResourceInference({workspaceRoot, symlinkPolicy:
/// "within-workspace", tools: createToolCatalog()})`。
fn inference<'a>(
    root: &str,
    catalog: &'a cheaper_tools::catalog::ToolCatalog,
) -> WorkspaceResourceInference<'a> {
    WorkspaceResourceInference {
        workspace_root: root.to_string(),
        symlink_policy: SymlinkPolicy::WithinWorkspace,
        tools: catalog,
    }
}

/// fixture tools(`resources` callback 無し metadata)+ policy "deny"。
fn fixture_inference<'a>(
    root: &str,
    tools: &'a FixtureTools,
    policy: SymlinkPolicy,
) -> WorkspaceResourceInference<'a> {
    WorkspaceResourceInference {
        workspace_root: root.to_string(),
        symlink_policy: policy,
        tools,
    }
}

// ---------------------------------------------------------------------------
// catalog resources and fallback
// ---------------------------------------------------------------------------

#[tokio::test]
async fn catalog_read_write_edit_scopes() {
    let ws = workspace();
    let catalog = cheaper_tools::catalog::create_tool_catalog(None).expect("catalog");
    for name in ["read", "count_tokens", "write", "edit"] {
        let result = inference(&ws.root, &catalog)
            .infer(&[(
                "a".to_string(),
                tool_job(
                    name,
                    json!({ "path": "src/a.ts" }),
                    Some(WorkflowResources {
                        produces: Some(Vec::new()),
                        ..Default::default()
                    }),
                ),
            )])
            .await
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let m = manifest(&result, "a");
        let expected_reads = if name == "write" {
            Vec::new()
        } else {
            vec![claim(
                &ws.root,
                "src/a.ts",
                ClaimMode::Shared,
                ResourceScope::Exact,
            )]
        };
        assert_eq!(m.reads, expected_reads, "{name} reads");
        let expected_writes = if name == "write" || name == "edit" {
            vec![claim(
                &ws.root,
                "src/a.ts",
                ClaimMode::Exclusive,
                ResourceScope::Exact,
            )]
        } else {
            Vec::new()
        };
        assert_eq!(m.writes, expected_writes, "{name} writes");
        assert!(m.produces.is_empty(), "{name} produces");
    }
}

#[tokio::test]
async fn catalog_subtree_and_default() {
    let ws = workspace();
    let catalog = cheaper_tools::catalog::create_tool_catalog(None).expect("catalog");
    for name in ["grep", "find", "ls"] {
        let result = inference(&ws.root, &catalog)
            .infer(&[
                (
                    "a".to_string(),
                    tool_job(
                        name,
                        json!({ "path": "src" }),
                        Some(WorkflowResources {
                            produces: Some(Vec::new()),
                            ..Default::default()
                        }),
                    ),
                ),
                (
                    "b".to_string(),
                    tool_job(
                        name,
                        json!({}),
                        Some(WorkflowResources {
                            produces: Some(Vec::new()),
                            ..Default::default()
                        }),
                    ),
                ),
            ])
            .await
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(
            manifest(&result, "a").reads,
            vec![claim(
                &ws.root,
                "src",
                ClaimMode::Shared,
                ResourceScope::Subtree
            )],
            "{name} a.reads"
        );
        assert_eq!(
            manifest(&result, "b").reads,
            vec![claim(
                &ws.root,
                ".",
                ClaimMode::Shared,
                ResourceScope::Subtree
            )],
            "{name} b.reads"
        );
        assert!(manifest(&result, "a").writes.is_empty());
    }
}

#[tokio::test]
async fn metadata_absent_fallback_and_declared_empty_read_array() {
    let ws = workspace();
    let tools = FixtureTools::new();
    let adapter = fixture_inference(&ws.root, &tools, SymlinkPolicy::Deny);
    let result = adapter
        .infer(&[
            ("unknown".to_string(), tool_job("grep", json!({}), None)),
            (
                "noRead".to_string(),
                tool_job(
                    "grep",
                    json!({}),
                    Some(WorkflowResources {
                        reads: Some(Vec::new()),
                        ..Default::default()
                    }),
                ),
            ),
        ])
        .await
        .expect("infer");
    assert_eq!(
        manifest(&result, "unknown"),
        &ResourceManifest {
            reads: vec![claim(
                &ws.root,
                ".",
                ClaimMode::Shared,
                ResourceScope::Subtree
            )],
            writes: vec![claim(
                &ws.root,
                ".",
                ClaimMode::Exclusive,
                ResourceScope::Subtree
            )],
            produces: Vec::new(),
        }
    );
    assert!(manifest(&result, "noRead").reads.is_empty());
    assert!(result.edges.is_empty());
}

#[tokio::test]
async fn omitted_empty_object_fallback_retains_known_consumer_dependencies() {
    let ws = workspace();
    let catalog = cheaper_tools::catalog::create_tool_catalog(None).expect("catalog");
    let result = inference(&ws.root, &catalog)
        .infer(&[
            (
                "producer".to_string(),
                tool_job(
                    "write",
                    json!({ "path": "generated/a.json" }),
                    Some(WorkflowResources {
                        produces: Some(vec![exact("generated/a.json")]),
                        ..Default::default()
                    }),
                ),
            ),
            (
                "consumer".to_string(),
                tool_job("read", json!({ "path": "generated/a.json" }), None),
            ),
            (
                "empty".to_string(),
                tool_job(
                    "read",
                    json!({ "path": "unrelated.json" }),
                    Some(WorkflowResources::default()),
                ),
            ),
        ])
        .await
        .expect("infer");
    assert_eq!(
        manifest(&result, "consumer").reads,
        vec![claim(
            &ws.root,
            ".",
            ClaimMode::Shared,
            ResourceScope::Subtree
        )]
    );
    assert_eq!(
        manifest(&result, "empty").writes,
        vec![claim(
            &ws.root,
            ".",
            ClaimMode::Exclusive,
            ResourceScope::Subtree
        )]
    );
    assert_eq!(
        result.edges,
        vec![cheaper_workflow::DependencyEdge {
            from: "producer".to_string(),
            to: "consumer".to_string()
        }]
    );
}

#[tokio::test]
async fn known_access_underclaim_rejected_and_fallback_paths() {
    let ws = workspace();
    let catalog = cheaper_tools::catalog::create_tool_catalog(None).expect("catalog");
    let err = inference(&ws.root, &catalog)
        .infer(&[(
            "a".to_string(),
            tool_job(
                "read",
                json!({ "path": "a" }),
                Some(WorkflowResources {
                    reads: Some(Vec::new()),
                    ..Default::default()
                }),
            ),
        )])
        .await
        .expect_err("underclaim rejected");
    assert_eq!(common::err_code(&err), Some(ErrorCode::InvalidRequest));

    let result = inference(&ws.root, &catalog)
        .infer(&[
            (
                "absolute".to_string(),
                tool_job(
                    "read",
                    json!({ "path": nodepath::join(&[&ws.root, "a"]) }),
                    Some(WorkflowResources {
                        produces: Some(Vec::new()),
                        ..Default::default()
                    }),
                ),
            ),
            (
                "deferred".to_string(),
                tool_job(
                    "read",
                    json!({ "path": { "$from": "source", "path": "/details/path" } }),
                    Some(WorkflowResources {
                        produces: Some(Vec::new()),
                        ..Default::default()
                    }),
                ),
            ),
        ])
        .await
        .expect("fallback");
    for id in ["absolute", "deferred"] {
        assert_eq!(
            manifest(&result, id).reads,
            vec![claim(
                &ws.root,
                ".",
                ClaimMode::Shared,
                ResourceScope::Subtree
            )],
            "{id} reads"
        );
    }
    assert!(result.edges.is_empty());
}

/// `command` の catalog manifest(design §(f))— generic tool 分岐から
/// `command_grant_resources()`(保守的 root subtree reads+writes、
/// `produces` 非宣言)が適用される。`resources` omitted 時の generic
/// fallback も同一形に収束し、明示 declared が known access を
/// underclaim すると `declared {bucket} underclaims known access`
/// (`INVALID_REQUEST`)で fail-closed する。
#[tokio::test]
async fn catalog_command_conservative_manifest_and_underclaim() {
    let ws = workspace();
    let catalog = cheaper_tools::catalog::create_tool_catalog(None).expect("catalog");
    let result = inference(&ws.root, &catalog)
        .infer(&[(
            "a".to_string(),
            tool_job(
                "command",
                json!({ "name": "build", "args": ["--release"] }),
                Some(WorkflowResources {
                    produces: Some(Vec::new()),
                    ..Default::default()
                }),
            ),
        )])
        .await
        .expect("command manifest");
    let m = manifest(&result, "a");
    assert_eq!(
        m.reads,
        vec![claim(
            &ws.root,
            ".",
            ClaimMode::Shared,
            ResourceScope::Subtree
        )]
    );
    assert_eq!(
        m.writes,
        vec![claim(
            &ws.root,
            ".",
            ClaimMode::Exclusive,
            ResourceScope::Subtree
        )]
    );
    // `produces` は宣言しない(duplicate-producer 検出との衝突回避)。
    assert!(m.produces.is_empty());

    // `resources` omitted — non-process tool は generic fallback に落ちる
    // が、command の保守的 manifest と同一形へ収束する。
    let omitted = inference(&ws.root, &catalog)
        .infer(&[(
            "a".to_string(),
            tool_job("command", json!({ "name": "build", "args": [] }), None),
        )])
        .await
        .expect("omitted fallback");
    assert_eq!(manifest(&omitted, "a").reads, m.reads);
    assert_eq!(manifest(&omitted, "a").writes, m.writes);
    let diagnostics = omitted.diagnostics.expect("diagnostics");
    assert!(has_fallback(&diagnostics, "a", "resources-omitted"));

    // 明示 declared の underclaim — known access(root subtree)を含まない
    // declared は bucket 毎に `INVALID_REQUEST` で reject される。
    for (resources, bucket) in [
        (
            WorkflowResources {
                reads: Some(Vec::new()),
                ..Default::default()
            },
            "reads",
        ),
        (
            WorkflowResources {
                writes: Some(vec![exact("out/only")]),
                ..Default::default()
            },
            "writes",
        ),
    ] {
        let err = inference(&ws.root, &catalog)
            .infer(&[(
                "a".to_string(),
                tool_job(
                    "command",
                    json!({ "name": "build", "args": [] }),
                    Some(resources),
                ),
            )])
            .await
            .expect_err("command underclaim");
        assert_eq!(common::err_code(&err), Some(ErrorCode::InvalidRequest));
        let message = err
            .downcast_ref::<CheaperError>()
            .map(|e| e.message().to_string())
            .unwrap_or_else(|| err.to_string());
        assert!(
            message.contains(&format!("declared {bucket} underclaims known access")),
            "{message}"
        );
    }
}

// ---------------------------------------------------------------------------
// Worker authoritative scope
// ---------------------------------------------------------------------------

#[tokio::test]
async fn worker_write_scope_targets_and_ancestor_mkdir_safety() {
    let ws = workspace();
    let catalog = cheaper_tools::catalog::create_tool_catalog(None).expect("catalog");
    let result = inference(&ws.root, &catalog)
        .infer(&[(
            "a".to_string(),
            worker_job(
                json!({ "targets": ["src/a.ts", { "path": "src/b.ts" }] }),
                WorkflowResources {
                    reads: Some(vec![exact("input.json")]),
                    ..Default::default()
                },
                mutating_worker(),
            ),
        )])
        .await
        .expect("infer");
    let m = manifest(&result, "a");
    assert_eq!(
        m.reads,
        vec![claim(
            &ws.root,
            "input.json",
            ClaimMode::Shared,
            ResourceScope::Exact
        )]
    );
    for expected in [
        claim(
            &ws.root,
            "src/a.ts",
            ClaimMode::Exclusive,
            ResourceScope::Exact,
        ),
        claim(
            &ws.root,
            "src/b.ts",
            ClaimMode::Exclusive,
            ResourceScope::Exact,
        ),
        claim(&ws.root, ".", ClaimMode::Exclusive, ResourceScope::Subtree),
    ] {
        assert!(m.writes.contains(&expected), "missing write {expected:?}");
    }
    let diagnostics = result.diagnostics.expect("diagnostics");
    assert!(has_fallback(&diagnostics, "a", "worker-ancestor-mkdir"));
}

#[tokio::test]
async fn worker_underclaim_rejection() {
    let ws = workspace();
    let catalog = cheaper_tools::catalog::create_tool_catalog(None).expect("catalog");
    for resources in [
        WorkflowResources {
            reads: Some(Vec::new()),
            writes: Some(vec![exact("outside.ts")]),
            ..Default::default()
        },
        WorkflowResources {
            reads: Some(Vec::new()),
            writes: Some(Vec::new()),
            ..Default::default()
        },
        WorkflowResources {
            reads: Some(Vec::new()),
            produces: Some(vec![subtree("src")]),
            ..Default::default()
        },
    ] {
        let err = inference(&ws.root, &catalog)
            .infer(&[(
                "a".to_string(),
                worker_job(
                    json!({ "targets": ["src/a.ts"] }),
                    resources,
                    mutating_worker(),
                ),
            )])
            .await
            .expect_err("underclaim");
        assert_eq!(common::err_code(&err), Some(ErrorCode::InvalidRequest));
    }
    for targets in [json!(null), json!(1), json!([2])] {
        let err = inference(&ws.root, &catalog)
            .infer(&[(
                "a".to_string(),
                worker_job(
                    json!({ "targets": targets }),
                    WorkflowResources {
                        reads: Some(Vec::new()),
                        ..Default::default()
                    },
                    mutating_worker(),
                ),
            )])
            .await
            .expect_err("invalid targets");
        assert_eq!(common::err_code(&err), Some(ErrorCode::InvalidInput));
    }
}

#[tokio::test]
async fn deferred_write_scope_is_unknown_and_cannot_certify_produces() {
    let ws = workspace();
    let catalog = cheaper_tools::catalog::create_tool_catalog(None).expect("catalog");
    let item = json!({ "$from": "source", "path": "/details/targets" });
    let result = inference(&ws.root, &catalog)
        .infer(&[(
            "a".to_string(),
            worker_job(
                json!({ "targets": item }),
                WorkflowResources {
                    reads: Some(Vec::new()),
                    ..Default::default()
                },
                mutating_worker(),
            ),
        )])
        .await
        .expect("deferred fallback");
    assert_eq!(
        manifest(&result, "a").writes,
        vec![claim(
            &ws.root,
            ".",
            ClaimMode::Exclusive,
            ResourceScope::Subtree
        )]
    );
    let err = inference(&ws.root, &catalog)
        .infer(&[(
            "a".to_string(),
            worker_job(
                json!({ "targets": item }),
                WorkflowResources {
                    reads: Some(Vec::new()),
                    produces: Some(vec![exact("src/a.ts")]),
                    ..Default::default()
                },
                mutating_worker(),
            ),
        )])
        .await
        .expect_err("uncertified produces");
    assert_eq!(common::err_code(&err), Some(ErrorCode::InvalidRequest));
}

#[tokio::test]
async fn worker_read_only_and_unknown_mutation() {
    let ws = workspace();
    let catalog = cheaper_tools::catalog::create_tool_catalog(None).expect("catalog");
    let make_job = |resources: WorkflowResources| {
        // `WorkerDefinition` は Clone を持たない — fixture を毎回再構築
        // する(TS は同一 object を共有するが推論側は入力を mutate
        // しないため等価)。
        let worker = fixture_worker();
        ResourceInferenceJob {
            process: None,
            job: WorkflowJob {
                kind: JobKind::Worker,
                name: worker.name.clone(),
                input: json!({ "task": "x" }),
                needs: None,
                resources: Some(resources),
            },
            worker: Some(Arc::new(worker)),
        }
    };
    let result = inference(&ws.root, &catalog)
        .infer(&[(
            "a".to_string(),
            make_job(WorkflowResources {
                reads: Some(Vec::new()),
                ..Default::default()
            }),
        )])
        .await
        .expect("read-only");
    assert_eq!(manifest(&result, "a"), &default_manifest());
    let err = inference(&ws.root, &catalog)
        .infer(&[(
            "a".to_string(),
            make_job(WorkflowResources {
                reads: Some(Vec::new()),
                produces: Some(vec![exact("a")]),
                ..Default::default()
            }),
        )])
        .await
        .expect_err("produces rejected");
    assert_eq!(common::err_code(&err), Some(ErrorCode::InvalidRequest));

    let mut mutated = fixture_worker();
    mutated.tools.push(ToolName::Write);
    let result = inference(&ws.root, &catalog)
        .infer(&[(
            "a".to_string(),
            ResourceInferenceJob {
                process: None,
                job: WorkflowJob {
                    kind: JobKind::Worker,
                    name: mutated.name.clone(),
                    input: json!({ "task": "x" }),
                    needs: None,
                    resources: Some(WorkflowResources {
                        reads: Some(Vec::new()),
                        ..Default::default()
                    }),
                },
                worker: Some(Arc::new(mutated)),
            },
        )])
        .await
        .expect("mutable worker");
    assert_eq!(
        manifest(&result, "a").writes,
        vec![claim(
            &ws.root,
            ".",
            ClaimMode::Exclusive,
            ResourceScope::Subtree
        )]
    );
}

// ---------------------------------------------------------------------------
// dependency, ownership and conflict
// ---------------------------------------------------------------------------

#[tokio::test]
async fn produces_to_read_edge_and_no_self_edge() {
    let ws = workspace();
    let tools = FixtureTools::new();
    let adapter = fixture_inference(&ws.root, &tools, SymlinkPolicy::WithinWorkspace);
    let result = adapter
        .infer(&[
            (
                "a".to_string(),
                declared(WorkflowResources {
                    reads: Some(vec![exact("out/a")]),
                    writes: Some(Vec::new()),
                    produces: Some(vec![exact("out/a")]),
                }),
            ),
            (
                "b".to_string(),
                declared(WorkflowResources {
                    reads: Some(vec![subtree("out")]),
                    writes: Some(Vec::new()),
                    ..Default::default()
                }),
            ),
        ])
        .await
        .expect("infer");
    assert_eq!(
        result.edges,
        vec![cheaper_workflow::DependencyEdge {
            from: "a".to_string(),
            to: "b".to_string()
        }]
    );
    let diagnostics = result.diagnostics.expect("diagnostics");
    assert!(has_dependency(
        &diagnostics,
        ["a", "b"],
        [
            claim(
                &ws.root,
                "out/a",
                ClaimMode::Exclusive,
                ResourceScope::Exact
            ),
            claim(&ws.root, "out", ClaimMode::Shared, ResourceScope::Subtree),
        ]
    ));
}

#[tokio::test]
async fn writers_conflict_without_edge_and_component_boundary() {
    let ws = workspace();
    let tools = FixtureTools::new();
    let adapter = fixture_inference(&ws.root, &tools, SymlinkPolicy::WithinWorkspace);
    let write = |path: &str| {
        declared(WorkflowResources {
            reads: Some(Vec::new()),
            writes: Some(vec![exact(path)]),
            ..Default::default()
        })
    };
    let result = adapter
        .infer(&[
            ("a".to_string(), write("src/a")),
            ("b".to_string(), write("src/a")),
            ("c".to_string(), write("src/ab")),
        ])
        .await
        .expect("infer");
    assert!(result.edges.is_empty());
    assert_eq!(
        result.conflicts.expect("conflicts"),
        vec![cheaper_workflow::ResourceConflict {
            jobs: ["a".to_string(), "b".to_string()],
            claims: [
                claim(
                    &ws.root,
                    "src/a",
                    ClaimMode::Exclusive,
                    ResourceScope::Exact
                ),
                claim(
                    &ws.root,
                    "src/a",
                    ClaimMode::Exclusive,
                    ResourceScope::Exact
                ),
            ],
        }]
    );
}

#[tokio::test]
async fn unknown_read_does_not_order_but_conflicts() {
    let ws = workspace();
    let tools = FixtureTools::new();
    let adapter = fixture_inference(&ws.root, &tools, SymlinkPolicy::WithinWorkspace);
    let result = adapter
        .infer(&[
            (
                "a".to_string(),
                declared(WorkflowResources {
                    reads: Some(Vec::new()),
                    writes: Some(Vec::new()),
                    produces: Some(vec![exact("out/a")]),
                }),
            ),
            (
                "b".to_string(),
                tool_job(
                    "read",
                    json!({ "path": { "$from": "a", "path": "/details/path" } }),
                    None,
                ),
            ),
        ])
        .await
        .expect("infer");
    assert!(result.edges.is_empty());
    assert!(!result.conflicts.expect("conflicts").is_empty());
}

#[tokio::test]
async fn duplicate_exact_subtree_ownership_is_deterministic() {
    let ws = workspace();
    let tools = FixtureTools::new();
    let adapter = fixture_inference(&ws.root, &tools, SymlinkPolicy::WithinWorkspace);
    for (left, right) in [
        (exact("out/a"), exact("out/./a")),
        (subtree("out"), exact("out/a")),
        (subtree("out"), subtree("out/sub")),
    ] {
        let produces = |d: ScopeDeclaration| WorkflowResources {
            reads: Some(Vec::new()),
            writes: Some(Vec::new()),
            produces: Some(vec![d]),
        };
        let jobs = [
            ("a".to_string(), declared(produces(left.clone()))),
            ("b".to_string(), declared(produces(right.clone()))),
        ];
        let first = adapter.infer(&jobs).await.expect_err("duplicate");
        assert_eq!(common::err_code(&first), Some(ErrorCode::InvalidRequest));
        let details = err_details(&first).expect("details");
        assert_eq!(details["kind"], json!("duplicate-producer"));
        assert_eq!(details["jobs"], json!(["a", "b"]));
        // reversed job 順でも同一 error。
        let reversed = [jobs[1].clone(), jobs[0].clone()];
        let second = adapter.infer(&reversed).await.expect_err("duplicate");
        let (first, second) = (
            first.downcast_ref::<CheaperError>().unwrap(),
            second.downcast_ref::<CheaperError>().unwrap(),
        );
        assert_eq!(first.code(), second.code());
        assert_eq!(first.message(), second.message());
        assert_eq!(first.details(), second.details());
    }
}

#[tokio::test]
async fn canonical_alias_and_case_output() {
    let ws = workspace();
    let tools = FixtureTools::new();
    let adapter = fixture_inference(&ws.root, &tools, SymlinkPolicy::WithinWorkspace);
    let produces = |p: &str| WorkflowResources {
        reads: Some(Vec::new()),
        writes: Some(Vec::new()),
        produces: Some(vec![exact(p)]),
    };
    std::fs::create_dir(nodepath::join(&[&ws.root, "real"])).expect("mkdir real");
    symlink_dir(
        &nodepath::join(&[&ws.root, "real"]),
        &nodepath::join(&[&ws.root, "alias"]),
    );
    let err = adapter
        .infer(&[
            ("a".to_string(), declared(produces("real/out"))),
            ("b".to_string(), declared(produces("alias/out"))),
        ])
        .await
        .expect_err("canonical alias");
    assert_eq!(common::err_code(&err), Some(ErrorCode::InvalidRequest));

    let result = adapter
        .infer(&[
            ("a".to_string(), declared(produces("Case"))),
            ("b".to_string(), declared(produces("case"))),
        ])
        .await;
    if cfg!(windows) {
        assert_eq!(
            common::err_code(&result.expect_err("case alias")),
            Some(ErrorCode::InvalidRequest)
        );
    } else {
        result.expect("case-sensitive FS allows distinct names");
    }
}

#[tokio::test]
async fn core_boundary_escape_and_glob_safety_and_ownership() {
    let ws = workspace();
    let tools = FixtureTools::new();
    let adapter = fixture_inference(&ws.root, &tools, SymlinkPolicy::WithinWorkspace);
    let err = adapter
        .infer(&[(
            "a".to_string(),
            declared(WorkflowResources {
                produces: Some(vec![exact("out/*.json")]),
                ..Default::default()
            }),
        )])
        .await
        .expect_err("glob produces rejected");
    assert_eq!(common::err_code(&err), Some(ErrorCode::InvalidRequest));
    assert!(adapter
        .infer(&[(
            "a".to_string(),
            declared(WorkflowResources {
                reads: Some(vec![exact("../escape")]),
                ..Default::default()
            })
        )])
        .await
        .is_err());
    let result = adapter
        .infer(&[(
            "a".to_string(),
            declared(WorkflowResources {
                reads: Some(vec![exact("src/*.ts")]),
                writes: Some(Vec::new()),
                produces: Some(vec![subtree("out/**")]),
            }),
        )])
        .await
        .expect("glob fallback");
    let m = manifest(&result, "a");
    assert_eq!(
        m.reads,
        vec![claim(
            &ws.root,
            ".",
            ClaimMode::Shared,
            ResourceScope::Subtree
        )]
    );
    assert_eq!(
        m.produces,
        vec![claim(
            &ws.root,
            "out",
            ClaimMode::Exclusive,
            ResourceScope::Subtree
        )]
    );
}

#[tokio::test]
async fn reversed_job_and_claim_byte_equality_and_no_input_mutation() {
    let ws = workspace();
    let tools = FixtureTools::new();
    let adapter = fixture_inference(&ws.root, &tools, SymlinkPolicy::WithinWorkspace);
    let jobs = [
        (
            "b".to_string(),
            declared(WorkflowResources {
                reads: Some(vec![exact("z"), exact("out/a")]),
                writes: Some(vec![exact("state")]),
                ..Default::default()
            }),
        ),
        (
            "a".to_string(),
            declared(WorkflowResources {
                reads: Some(Vec::new()),
                writes: Some(vec![exact("state")]),
                produces: Some(vec![exact("out/b"), exact("out/a"), exact("out/a")]),
            }),
        ),
    ];
    let snapshot: Vec<(String, ResourceInferenceJob)> = jobs.to_vec();
    // reversed claim 順 — a の produces/read 順を反転する。
    let mut reversed: Vec<(String, ResourceInferenceJob)> = jobs.to_vec();
    for (_, job) in &mut reversed {
        if let Some(resources) = &mut job.job.resources {
            for list in [
                &mut resources.reads,
                &mut resources.writes,
                &mut resources.produces,
            ]
            .into_iter()
            .flatten()
            {
                list.reverse();
            }
        }
    }
    reversed.reverse();
    let first = adapter.infer(&jobs).await.expect("first");
    let second = adapter.infer(&reversed).await.expect("second");
    assert_eq!(first.manifests, second.manifests);
    assert_eq!(first.edges, second.edges);
    assert_eq!(first.conflicts, second.conflicts);
    assert_eq!(first.diagnostics, second.diagnostics);
    // caller input は mutation されない。
    assert_eq!(jobs[0].1.job.resources, snapshot[0].1.job.resources);
    assert_eq!(jobs[1].1.job.resources, snapshot[1].1.job.resources);
    assert_eq!(manifest_keys(&first), ["a", "b"]);
}

// ---------------------------------------------------------------------------
// process declarations
// ---------------------------------------------------------------------------

#[tokio::test]
async fn process_config_input_output_edge_and_undeclared_rejection() {
    let ws = workspace();
    let catalog = cheaper_tools::catalog::create_tool_catalog(None).expect("catalog");
    let adapter = inference(&ws.root, &catalog);
    let process_job = |task: &str| tool_job("process", json!({ "task": task }), None);
    let err = adapter
        .infer(&[("a".to_string(), process_job("generate"))])
        .await
        .expect_err("unknown task rejected");
    assert_eq!(common::err_code(&err), Some(ErrorCode::PolicyViolation));

    let with_process =
        |task: &str, inputs: Vec<CoreScopeDeclaration>, outputs: Vec<CoreScopeDeclaration>| {
            let mut job = process_job(task);
            job.process = Some(ProcessResourceDeclaration {
                task: task.to_string(),
                inputs,
                outputs,
            });
            job
        };
    let mut empty = process_job("no-io");
    empty.job.resources = Some(WorkflowResources {
        reads: Some(Vec::new()),
        writes: Some(Vec::new()),
        ..Default::default()
    });
    empty.process = Some(ProcessResourceDeclaration {
        task: "no-io".to_string(),
        inputs: Vec::new(),
        outputs: Vec::new(),
    });
    let result = adapter
        .infer(&[
            (
                "a".to_string(),
                with_process("generate", Vec::new(), vec![core_exact("out/a")]),
            ),
            (
                "b".to_string(),
                with_process("consume", vec![core_exact("out/a")], Vec::new()),
            ),
            ("empty".to_string(), empty),
        ])
        .await
        .expect("declared process");
    assert_eq!(
        result.edges,
        vec![cheaper_workflow::DependencyEdge {
            from: "a".to_string(),
            to: "b".to_string()
        }]
    );
    assert_eq!(
        manifest(&result, "a").produces,
        vec![claim(
            &ws.root,
            "out/a",
            ClaimMode::Exclusive,
            ResourceScope::Exact
        )]
    );
    assert_eq!(manifest(&result, "empty"), &default_manifest());
}

#[tokio::test]
async fn job_ids_including_prototype_keys_remain_own_manifests() {
    let ws = workspace();
    let catalog = cheaper_tools::catalog::create_tool_catalog(None).expect("catalog");
    let jobs: Vec<(String, ResourceInferenceJob)> = ["__proto__", "constructor", "toString"]
        .iter()
        .map(|id| {
            (
                (*id).to_string(),
                tool_job("read", json!({ "path": "a" }), None),
            )
        })
        .collect();
    let result = inference(&ws.root, &catalog)
        .infer(&jobs)
        .await
        .expect("infer");
    assert_eq!(
        manifest_keys(&result),
        ["__proto__", "constructor", "toString"]
    );
    assert!(manifest_keys(&result).contains(&"__proto__"));
}

#[tokio::test]
async fn worker_wildcard_targets_cannot_expand_authoritative_ownership() {
    let ws = workspace();
    let catalog = cheaper_tools::catalog::create_tool_catalog(None).expect("catalog");
    let err = inference(&ws.root, &catalog)
        .infer(&[(
            "a".to_string(),
            worker_job(
                json!({ "targets": ["src/*.ts"] }),
                WorkflowResources {
                    reads: Some(Vec::new()),
                    produces: Some(vec![exact("unrelated.json")]),
                    ..Default::default()
                },
                mutating_worker(),
            ),
        )])
        .await
        .expect_err("wildcard ownership rejected");
    assert_eq!(common::err_code(&err), Some(ErrorCode::InvalidRequest));
}

#[tokio::test]
async fn process_source_cannot_replace_another_job_kind_authority() {
    let ws = workspace();
    let catalog = cheaper_tools::catalog::create_tool_catalog(None).expect("catalog");
    let mut job = tool_job("read", json!({ "path": "a" }), None);
    job.process = Some(ProcessResourceDeclaration {
        task: "generate".to_string(),
        inputs: Vec::new(),
        outputs: vec![core_exact("b")],
    });
    let err = inference(&ws.root, &catalog)
        .infer(&[("a".to_string(), job)])
        .await
        .expect_err("process on read rejected");
    assert_eq!(common::err_code(&err), Some(ErrorCode::InvalidRequest));
}

#[tokio::test]
async fn process_subtree_glob_ownership_cannot_authorize_outside_outputs() {
    let ws = workspace();
    let catalog = cheaper_tools::catalog::create_tool_catalog(None).expect("catalog");
    let mut job = tool_job(
        "process",
        json!({ "task": "generate" }),
        Some(WorkflowResources {
            produces: Some(vec![subtree("out/**"), exact("outside.json")]),
            ..Default::default()
        }),
    );
    job.process = Some(ProcessResourceDeclaration {
        task: "generate".to_string(),
        inputs: Vec::new(),
        outputs: vec![core_subtree("out/**")],
    });
    let err = inference(&ws.root, &catalog)
        .infer(&[("a".to_string(), job)])
        .await
        .expect_err("outside output rejected");
    assert_eq!(common::err_code(&err), Some(ErrorCode::InvalidRequest));
}
