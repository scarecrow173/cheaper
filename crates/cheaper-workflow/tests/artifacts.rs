//! Port of `packages/workflow/test/artifacts.test.ts`。
//!
//! `Object.isFrozen`/`Object.getOwnPropertyDescriptors`/`structuredClone`
//! 相当の表層検査は Rust 型システムでは不変・owned value として自明なため、
//! 対応する不変条件(identity/provenance/plain-JSON shape)を検査する。
//! `removeEventListener` spy は Rust port に観測 seam がないため、
//! abort 後の read 失敗と stats で同一 observable 状態を検査する。

mod common;

use std::sync::Arc;

use cheaper_core::errors::ErrorCode;
use cheaper_tools::artifact::{
    ArtifactData, ArtifactPorts, ArtifactProducer, ArtifactPublisher, ArtifactReadRequest,
    ArtifactReference, ArtifactSource,
};
use cheaper_tools::invocation::{
    create_invocation_tool_runtime, InvocationToolRuntimeOptions, WorkerScopeOptions,
};
use cheaper_tools::operation::AbortController;
use cheaper_tools::registry::ToolRuntimeOptions;
use serde_json::{json, Value};

use cheaper_workflow::{
    project_artifact_metadata, with_artifact_store, ArtifactProjectionOptions,
    ArtifactStoreOptions, ArtifactStoreStats, RunArtifactStore,
};

fn limits() -> ArtifactStoreOptions {
    ArtifactStoreOptions {
        max_artifact_bytes: 32,
        max_total_bytes: 48,
        max_preview_bytes: 5,
        max_read_bytes: 8,
        signal: None,
    }
}

fn producer() -> ArtifactProducer {
    ArtifactProducer {
        job_id: "A1".to_string(),
        name: "stdout".to_string(),
    }
}

fn source(reference: &ArtifactReference) -> ArtifactSource {
    ArtifactSource {
        reference: reference.clone(),
    }
}

fn put(store: &RunArtifactStore, data: impl Into<ArtifactData>) -> ArtifactReference {
    store
        .put(cheaper_tools::artifact::ArtifactPut {
            producer: producer(),
            data: data.into(),
        })
        .expect("put succeeds")
}

fn read_req(offset: u64, max_bytes: u64) -> ArtifactReadRequest {
    ArtifactReadRequest { offset, max_bytes }
}

fn scope_options(owner: &str, root: &str) -> WorkerScopeOptions {
    WorkerScopeOptions {
        owner_id: owner.to_string(),
        runtime: ToolRuntimeOptions {
            resource_manifest: None,
            write_scope: None,
            read_strictness: None,
            artifacts: None,
            process: None,
            command: None,
            root: root.to_string(),
            symlink_policy: cheaper_core::workspace::paths::SymlinkPolicy::Deny,
            names: vec![cheaper_core::tools::vocabulary::ToolName::Read],
            degradable: Vec::new(),
            max_tool_calls: 100,
            on_limit: Box::new(|| {}),
            journal: None,
            denylist: None,
            parent_lease: None,
            signal: None,
            abort_grace_ms: None,
            concurrency: None,
            eval_spawn: None,
        },
    }
}

#[tokio::test]
async fn borrows_canonical_store_across_scopes_identity_caps_cleanup() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root =
        cheaper_core::workspace::paths::realpath(&dir.path().to_string_lossy()).expect("realpath");
    let store = RunArtifactStore::new(limits()).expect("store");
    let publisher: Arc<dyn ArtifactPublisher> = Arc::clone(&store) as Arc<dyn ArtifactPublisher>;
    assert_eq!(publisher.max_artifact_bytes(), 32);
    let runtime = create_invocation_tool_runtime(InvocationToolRuntimeOptions {
        artifacts: Some(ArtifactPorts {
            reader: Some(store.reader()),
            publisher: Some(Arc::clone(&publisher)),
        }),
        process: None,
        command: None,
        root: root.clone(),
        symlink_policy: cheaper_core::workspace::paths::SymlinkPolicy::Deny,
        signal: None,
        abort_grace_ms: None,
        concurrency: None,
        clock: None,
    })
    .expect("runtime");
    let first = runtime
        .create_worker_scope(scope_options("producer", &root))
        .expect("producer scope");
    let second = runtime
        .create_worker_scope(scope_options("consumer", &root))
        .expect("consumer scope");

    tokio::fs::write(
        cheaper_core::nodepath::join(&[&root, "a.txt"]),
        "hello world",
    )
    .await
    .expect("write a.txt");
    let published = first
        .execute(
            "read",
            &json!({ "path": "a.txt", "artifact": "text" }),
            None,
        )
        .await
        .expect("publish read");
    assert!(published.content.is_empty());
    let artifact_ref: ArtifactReference =
        serde_json::from_value(published.details.expect("details")["artifact"].clone())
            .expect("reference deserializes");
    assert_eq!(
        artifact_ref.producer,
        ArtifactProducer {
            job_id: "producer".to_string(),
            name: "text".to_string()
        }
    );
    let stats = store.stats();
    let err = first
        .execute(
            "read",
            &json!({ "path": "a.txt", "artifact": "text" }),
            None,
        )
        .await
        .expect_err("duplicate producer rejected");
    assert_eq!(common::err_code(&err), Some(ErrorCode::InvalidInput));
    assert_eq!(store.stats(), stats);
    first.dispose().await;
    assert!(!store.stats().closed);

    let read = |reference: ArtifactReference, max_bytes: u64| {
        let second = second.clone();
        async move {
            second
                .execute(
                    "read",
                    &json!({
                        "source": { "kind": "artifact", "reference": reference },
                        "offset": 0,
                        "maxBytes": max_bytes,
                    }),
                    None,
                )
                .await
        }
    };
    let result = read(artifact_ref.clone(), 3).await.expect("artifact read");
    assert_eq!(result.content[0].text, "hel");
    assert_eq!(
        result.details.expect("details"),
        json!({ "offset": 0, "nextOffset": 3, "eof": false })
    );

    let tampered = ArtifactReference {
        preview: "tampered".to_string(),
        ..artifact_ref.clone()
    };
    let err = read(tampered, 3).await.expect_err("tamper rejected");
    assert_eq!(common::err_code(&err), Some(ErrorCode::InvalidInput));

    let foreign = RunArtifactStore::new(limits()).expect("foreign store");
    let foreign_ref = foreign
        .put(cheaper_tools::artifact::ArtifactPut {
            producer: producer(),
            data: "foreign".into(),
        })
        .expect("foreign put");
    let err = read(foreign_ref, 3)
        .await
        .expect_err("foreign ref rejected");
    assert_eq!(common::err_code(&err), Some(ErrorCode::InvalidInput));
    foreign.dispose();

    let err = read(artifact_ref.clone(), 9)
        .await
        .expect_err("over-limit read rejected");
    assert_eq!(common::err_code(&err), Some(ErrorCode::PolicyViolation));

    runtime.dispose().await;
    assert!(!store.stats().closed);
    assert_eq!(
        project_artifact_metadata(&artifact_ref, ArtifactProjectionOptions::default()).availability,
        cheaper_tools::artifact::ArtifactAvailability::Unavailable
    );
    store.dispose();
    let err = store
        .read(&source(&artifact_ref), &read_req(0, 3))
        .expect_err("closed read rejected");
    assert_eq!(err.code(), ErrorCode::InvalidRequest);
}

#[test]
fn retains_provenance_and_plain_json_metadata_with_opaque_uri() {
    let store = RunArtifactStore::new(limits()).expect("store");
    let reference = put(&store, "日本🙂ab");
    assert_eq!(reference.producer, producer());
    assert_eq!(reference.bytes, 12);
    assert_eq!(reference.preview, "日");
    assert!(reference.truncated);
    assert_eq!(reference.run_id, store.run_id());
    let uri_pattern = regex_lite_uri(&reference.artifact);
    assert!(uri_pattern, "opaque artifact uri: {}", reference.artifact);
    assert!(!reference.artifact.contains("stdout"));
    // plain JSON round-trip(materializeBindings 経由の clone 相当)。
    let json = serde_json::to_value(&reference).expect("reference serializes");
    let inputs = serde_json::Map::new();
    let results = std::collections::HashMap::new();
    let cloned = cheaper_workflow::materialize_bindings(
        &json!({ "$literal": json }),
        &inputs,
        &results,
        cheaper_workflow::MaterializeOptions::default(),
    )
    .expect("literal materializes");
    assert_eq!(
        serde_json::from_value::<ArtifactReference>(cloned).expect("round-trip"),
        reference
    );
    store.dispose();
}

fn regex_lite_uri(uri: &str) -> bool {
    // `^artifact://[a-f0-9-]{36}/[a-f0-9-]{36}$` 相当の手書き検査
    // (uuid::Uuid::to_string の lowercase hex format)。
    let Some(rest) = uri.strip_prefix("artifact://") else {
        return false;
    };
    let Some((run, id)) = rest.split_once('/') else {
        return false;
    };
    let is_uuid = |s: &str| {
        s.len() == 36
            && s.bytes().enumerate().all(|(i, b)| {
                if [8, 13, 18, 23].contains(&i) {
                    b == b'-'
                } else {
                    b.is_ascii_hexdigit() && !b.is_ascii_uppercase()
                }
            })
    };
    is_uuid(run) && is_uuid(id)
}

#[test]
fn rejects_per_artifact_and_total_overflow_atomically() {
    let store = RunArtifactStore::new(ArtifactStoreOptions {
        max_artifact_bytes: 8,
        max_total_bytes: 12,
        ..limits()
    })
    .expect("store");
    let a = put(&store, "12345678");
    let err = store
        .put(cheaper_tools::artifact::ArtifactPut {
            producer: ArtifactProducer {
                job_id: "A2".to_string(),
                name: "x".to_string(),
            },
            data: "123456789".into(),
        })
        .expect_err("per-artifact overflow");
    assert_eq!(err.code(), ErrorCode::PolicyViolation);
    assert_eq!(
        store.stats(),
        ArtifactStoreStats {
            bytes: 8,
            artifacts: 1,
            closed: false
        }
    );
    store
        .put(cheaper_tools::artifact::ArtifactPut {
            producer: ArtifactProducer {
                job_id: "A2".to_string(),
                name: "x".to_string(),
            },
            data: "1234".into(),
        })
        .expect("fits");
    let err = store
        .put(cheaper_tools::artifact::ArtifactPut {
            producer: ArtifactProducer {
                job_id: "A3".to_string(),
                name: "x".to_string(),
            },
            data: "1".into(),
        })
        .expect_err("total overflow");
    assert_eq!(err.code(), ErrorCode::PolicyViolation);
    assert_eq!(
        store.stats(),
        ArtifactStoreStats {
            bytes: 12,
            artifacts: 2,
            closed: false
        }
    );
    let read = store.read(&source(&a), &read_req(0, 8)).expect("read");
    assert_eq!(String::from_utf8(read.data).unwrap(), "12345678");
    store.dispose();
}

#[test]
fn bounds_utf8_and_invalid_byte_previews() {
    for (data, cap, expected) in [
        (ArtifactData::Text("日🙂x".to_string()), 3u64, "日"),
        (ArtifactData::Text("日🙂x".to_string()), 7, "日🙂"),
        (ArtifactData::Bytes(vec![0xff, 0xff, 0xff]), 4, "\u{FFFD}"),
        (ArtifactData::Text("abc".to_string()), 0, ""),
    ] {
        let store = RunArtifactStore::new(ArtifactStoreOptions {
            max_preview_bytes: cap,
            ..limits()
        })
        .expect("store");
        let reference = put(&store, data);
        assert_eq!(reference.preview, expected);
        assert!(reference.preview.len() <= cap as usize);
        assert!(reference.truncated);
        store.dispose();
    }
    let store = RunArtifactStore::new(limits()).expect("store");
    let reference = put(&store, "abc");
    assert_eq!(reference.preview, "abc");
    assert!(!reference.truncated);
    store.dispose();
}

#[test]
fn preserves_utf8_bom_in_bounded_preview_metadata() {
    let store = RunArtifactStore::new(ArtifactStoreOptions {
        max_preview_bytes: 4,
        ..limits()
    })
    .expect("store");
    let reference = put(&store, "\u{FEFF}x");
    assert_eq!(reference.preview, "\u{FEFF}x");
    assert!(!reference.truncated);
    assert_eq!(reference.bytes, 4);
    store.dispose();
}

#[tokio::test]
async fn keeps_invocation_signal_fixed_when_caller_options_change() {
    let controller = AbortController::new();
    let detached = AbortController::new();
    let err = with_artifact_store(
        ArtifactStoreOptions {
            signal: Some(controller.signal()),
            ..limits()
        },
        |store| {
            let detached = detached.clone();
            let controller = controller.clone();
            async move {
                store
                    .put(cheaper_tools::artifact::ArtifactPut {
                        producer: producer(),
                        data: "hello".into(),
                    })
                    .expect("put");
                // caller が options.signal を差し替えても store が保持する
                // signal は生成時のもの。detached signal の abort は無関係。
                detached.abort(cheaper_tools::operation::AbortReason::aborted());
                assert!(!store.stats().closed);
                controller.abort(cheaper_tools::operation::AbortReason::aborted());
                Ok("completed")
            }
        },
    )
    .await
    .expect_err("aborted store rejects the run result");
    assert_eq!(common::err_code(&err), Some(ErrorCode::Aborted));
}

#[test]
fn returns_bounded_copies_without_retaining_caller_mutations() {
    let store = RunArtifactStore::new(limits()).expect("store");
    let mut input = vec![1u8, 2, 3, 4, 5, 6, 7, 8, 9];
    let reference = put(&store, ArtifactData::Bytes(input.clone()));
    input.fill(0);
    let reader = store.reader();
    let first = reader
        .read(&source(&reference), &read_req(0, 4))
        .expect("read");
    assert_eq!(first.data, vec![1, 2, 3, 4]);
    assert_eq!(first.offset, 0);
    assert_eq!(first.next_offset, 4);
    assert!(!first.eof);
    // reader の返す data は copy — 後続 read に影響しない。
    let mut consumed = first.data;
    consumed.fill(0);
    assert_eq!(
        reader
            .read(&source(&reference), &read_req(4, 8))
            .expect("read")
            .data,
        vec![5, 6, 7, 8, 9]
    );
    assert_eq!(
        reader
            .read(&source(&reference), &read_req(0, 4))
            .expect("read")
            .data,
        vec![1, 2, 3, 4]
    );
    let tail = reader
        .read(&source(&reference), &read_req(9, 4))
        .expect("eof read");
    assert_eq!(tail.next_offset, 9);
    assert!(tail.eof);
    let err = reader
        .read(&source(&reference), &read_req(0, 9))
        .expect_err("over-limit");
    assert_eq!(common::err_code(&err), Some(ErrorCode::PolicyViolation));
    store.dispose();
    store.dispose();
    assert_eq!(
        store.stats(),
        ArtifactStoreStats {
            bytes: 0,
            artifacts: 0,
            closed: true
        }
    );
}

#[test]
fn rejects_duplicate_producers_without_overwriting() {
    let store = RunArtifactStore::new(limits()).expect("store");
    let reference = put(&store, "first");
    assert!(store
        .put(cheaper_tools::artifact::ArtifactPut {
            producer: producer(),
            data: "next".into(),
        })
        .is_err());
    let read = store
        .read(&source(&reference), &read_req(0, 8))
        .expect("read");
    assert_eq!(String::from_utf8(read.data).unwrap(), "first");
    store.dispose();
}

#[test]
fn rejects_foreign_runs_malformed_uris_and_altered_provenance() {
    let a = RunArtifactStore::new(limits()).expect("store a");
    let b = RunArtifactStore::new(limits()).expect("store b");
    let reference = put(&a, "secret");
    assert!(b.read(&source(&reference), &read_req(0, 8)).is_err());
    for artifact in [
        "../secret".to_string(),
        "file:///etc/passwd".to_string(),
        "C:\\private.txt".to_string(),
        "/etc/passwd".to_string(),
        format!("artifact://{}/../../secret", a.run_id()),
        format!("artifact://{}/%2e%2e", a.run_id()),
        format!("{}?path=/etc/passwd", reference.artifact),
        format!("{}/extra", reference.artifact),
        format!(
            "artifact://{}/00000000-0000-0000-0000-000000000000",
            a.run_id()
        ),
        format!("{}\n", reference.artifact),
    ] {
        let tampered = ArtifactReference {
            artifact,
            ..reference.clone()
        };
        assert!(
            a.read(&source(&tampered), &read_req(0, 8)).is_err(),
            "malformed uri accepted"
        );
    }
    for altered in [
        ArtifactReference {
            run_id: b.run_id().to_string(),
            ..reference.clone()
        },
        ArtifactReference {
            producer: ArtifactProducer {
                job_id: "A2".to_string(),
                name: "stdout".to_string(),
            },
            ..reference.clone()
        },
        ArtifactReference {
            bytes: 0,
            ..reference.clone()
        },
        ArtifactReference {
            preview: "forged".to_string(),
            ..reference.clone()
        },
    ] {
        assert!(a.read(&source(&altered), &read_req(0, 8)).is_err());
    }
    a.dispose();
    b.dispose();
}

#[test]
fn serializes_metadata_only_and_supports_include_preview() {
    let store = RunArtifactStore::new(ArtifactStoreOptions {
        max_artifact_bytes: 4096,
        max_total_bytes: 4096,
        ..limits()
    })
    .expect("store");
    let reference = put(&store, format!("head{}", "PRIVATE BODY".repeat(200)));
    let output = serde_json::to_string(&store.to_json()).expect("serialize");
    assert!(!output.contains("PRIVATE BODY"));
    let parsed: Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed["artifacts"][0]["preview"], json!("headP"));
    let metadata = store.metadata(ArtifactProjectionOptions {
        include_preview: Some(false),
    });
    assert_eq!(metadata.len(), 1);
    assert!(metadata[0].preview.is_none());
    store.dispose();
    let final_meta = project_artifact_metadata(&reference, ArtifactProjectionOptions::default());
    assert_eq!(
        final_meta.availability,
        cheaper_tools::artifact::ArtifactAvailability::Unavailable
    );
    assert_eq!(final_meta.preview.as_deref(), Some("headP"));
    assert_eq!(final_meta.bytes, 2404);
    assert!(project_artifact_metadata(
        &reference,
        ArtifactProjectionOptions {
            include_preview: Some(false)
        }
    )
    .preview
    .is_none());
    // `data` key は metadata shape に存在しない。
    let serialized = serde_json::to_value(&final_meta).unwrap();
    assert!(serialized.get("data").is_none());
    assert!(store
        .metadata(ArtifactProjectionOptions::default())
        .is_empty());
    let after = serde_json::to_string(&store.to_json()).unwrap();
    assert!(!after.contains("headP"));
}

#[tokio::test]
async fn with_artifact_store_disposes_on_completion_and_failure() {
    let mut borrowed: Option<Arc<dyn cheaper_tools::artifact::ArtifactReader>> = None;
    let mut owned: Option<Arc<RunArtifactStore>> = None;
    let mut captured: Option<ArtifactReference> = None;
    let value = with_artifact_store(limits(), |store| {
        owned = Some(Arc::clone(&store));
        borrowed = Some(store.reader());
        let reference = store
            .put(cheaper_tools::artifact::ArtifactPut {
                producer: producer(),
                data: "hello".into(),
            })
            .expect("put");
        captured = Some(reference.clone());
        async move { Ok(reference) }
    })
    .await
    .expect("success path");
    assert_eq!(value, captured.expect("captured"));
    assert_eq!(
        owned.as_ref().expect("owned").stats(),
        ArtifactStoreStats {
            bytes: 0,
            artifacts: 0,
            closed: true
        }
    );
    assert!(borrowed
        .expect("borrowed")
        .read(&source(&value), &read_req(0, 8))
        .is_err());

    let err = with_artifact_store(limits(), |store| {
        owned = Some(Arc::clone(&store));
        async move {
            store
                .put(cheaper_tools::artifact::ArtifactPut {
                    producer: producer(),
                    data: "hello".into(),
                })
                .expect("put");
            Err::<Value, _>(Box::<dyn std::error::Error + Send + Sync>::from(
                "producer failed",
            ))
        }
    })
    .await
    .expect_err("failure path");
    assert_eq!(err.to_string(), "producer failed");
    assert_eq!(
        owned.as_ref().expect("owned").stats(),
        ArtifactStoreStats {
            bytes: 0,
            artifacts: 0,
            closed: true
        }
    );
}

#[tokio::test]
async fn rejects_pre_aborted_signals_without_invoking_callback() {
    let controller = AbortController::new();
    controller.abort(cheaper_tools::operation::AbortReason::aborted());
    let err = common::unwrap_err(RunArtifactStore::new(ArtifactStoreOptions {
        signal: Some(controller.signal()),
        ..limits()
    }));
    assert_eq!(err.code(), ErrorCode::Aborted);
    let mut invoked = false;
    let err = with_artifact_store(
        ArtifactStoreOptions {
            signal: Some(controller.signal()),
            ..limits()
        },
        |_store| {
            invoked = true;
            async move { Ok(()) }
        },
    )
    .await
    .expect_err("pre-aborted run rejected");
    assert_eq!(common::err_code(&err), Some(ErrorCode::Aborted));
    assert!(!invoked);
}

#[tokio::test]
async fn clears_immediately_on_inflight_abort() {
    let controller = AbortController::new();
    let mut borrowed: Option<Arc<dyn cheaper_tools::artifact::ArtifactReader>> = None;
    let mut owned: Option<Arc<RunArtifactStore>> = None;
    let mut captured: Option<ArtifactReference> = None;
    let err = with_artifact_store(
        ArtifactStoreOptions {
            signal: Some(controller.signal()),
            ..limits()
        },
        |store| {
            owned = Some(Arc::clone(&store));
            borrowed = Some(store.reader());
            let reference = store
                .put(cheaper_tools::artifact::ArtifactPut {
                    producer: producer(),
                    data: "hello".into(),
                })
                .expect("put");
            captured = Some(reference.clone());
            async move {
                tokio::task::yield_now().await;
                controller.abort(cheaper_tools::operation::AbortReason::aborted());
                assert_eq!(
                    store.stats(),
                    ArtifactStoreStats {
                        bytes: 0,
                        artifacts: 0,
                        closed: true
                    }
                );
                let read_err = store
                    .read(&source(&reference), &read_req(0, 8))
                    .expect_err("aborted read");
                assert_eq!(read_err.code(), ErrorCode::Aborted);
                Ok(reference)
            }
        },
    )
    .await
    .expect_err("inflight abort propagates");
    assert_eq!(common::err_code(&err), Some(ErrorCode::Aborted));
    assert!(owned.expect("owned").stats().closed);
    let reference = captured.expect("captured");
    assert!(borrowed
        .expect("borrowed")
        .read(&source(&reference), &read_req(0, 8))
        .is_err());
}

#[test]
fn validates_injected_caps_and_read_ranges() {
    // u64 表現できる cap のみ -1/NaN/Infinity は型で排除される。
    // `Number.MAX_SAFE_INTEGER + 1` 相当は u64::MAX で代用する。
    for cap in [u64::MAX] {
        for options in [
            ArtifactStoreOptions {
                max_artifact_bytes: cap,
                ..limits()
            },
            ArtifactStoreOptions {
                max_total_bytes: cap,
                ..limits()
            },
            ArtifactStoreOptions {
                max_preview_bytes: cap,
                ..limits()
            },
            ArtifactStoreOptions {
                max_read_bytes: cap,
                ..limits()
            },
        ] {
            assert!(RunArtifactStore::new(options).is_err());
        }
    }
    assert!(RunArtifactStore::new(ArtifactStoreOptions {
        max_read_bytes: 0,
        ..limits()
    })
    .is_err());
    let store = RunArtifactStore::new(limits()).expect("store");
    let reference = put(&store, "x");
    for (offset, max_bytes) in [
        (2u64, 1u64),
        (u64::MAX, 1u64),
        (0u64, 0u64),
        (0u64, u64::MAX),
    ] {
        assert!(
            store
                .read(&source(&reference), &read_req(offset, max_bytes))
                .is_err(),
            "range {offset}/{max_bytes}"
        );
    }
    store.dispose();
    assert!(store
        .put(cheaper_tools::artifact::ArtifactPut {
            producer: producer(),
            data: "x".into(),
        })
        .is_err());
}
