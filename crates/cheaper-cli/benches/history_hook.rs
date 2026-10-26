//! Local Hook History overhead check. This uses the public History API and a
//! temporary local directory; each decision is an immutable per-execution
//! file and the measured path makes no network calls or shared append lock.

use std::fs;
use std::hint::black_box;
use std::time::{Duration, Instant};

use cheaper_cli::history::identity::ExecutionIdentity;
use cheaper_cli::history::schema::{
    DecisionVerdict, ExecutionEvent, ExecutionKind, ExecutionStatus, HistoryEvent,
    HookCacheProvenance, HookNativeVerdict, ReasonCode,
};
use cheaper_cli::history::store::HistoryStore;

const DECISIONS: usize = 1_000;
const P50_BUDGET: Duration = Duration::from_millis(5);
const P95_BUDGET: Duration = Duration::from_millis(25);

fn percentile(sorted: &[Duration], numerator: usize, denominator: usize) -> Duration {
    let rank = sorted.len().saturating_mul(numerator).div_ceil(denominator);
    sorted[rank.saturating_sub(1).min(sorted.len() - 1)]
}

fn decision(index: usize) -> HistoryEvent {
    let identity = ExecutionIdentity::root();
    let (reason, evaluator, native, provenance, file_bytes, measured_tokens, fallback) =
        match index % 4 {
            0 => (
                ReasonCode::ReadByteFastPath,
                DecisionVerdict::Allow,
                HookNativeVerdict::Allow,
                HookCacheProvenance::Bypass,
                128.0,
                None,
                None,
            ),
            1 => (
                ReasonCode::ReadBelowThreshold,
                DecisionVerdict::Allow,
                HookNativeVerdict::Allow,
                HookCacheProvenance::LeaderEvaluated,
                24_000.0,
                Some(Some(1_024.0)),
                None,
            ),
            2 => (
                ReasonCode::ReadThresholdExceededDelegateAdvised,
                DecisionVerdict::DenyDelegationAdvised,
                HookNativeVerdict::Deny,
                HookCacheProvenance::LeaderEvaluated,
                108_000.0,
                Some(Some(9_216.0)),
                None,
            ),
            _ => (
                ReasonCode::RouteRewriteEmitted,
                DecisionVerdict::RewriteEmitted,
                HookNativeVerdict::Deny,
                HookCacheProvenance::LeaderEvaluated,
                108_000.0,
                Some(Some(9_216.0)),
                Some(ReasonCode::RouteRewriteUnsupportedFallbackDeny),
            ),
        };
    HistoryEvent::new(
        &identity,
        ExecutionKind::HookDecision,
        ExecutionEvent::Decision,
        ExecutionStatus::Completed,
    )
    .with_reason_code(Some(reason))
    .with_hook_evidence(Some(evaluator), Some(native), fallback, Some(provenance))
    .with_file_bytes(Some(file_bytes))
    .with_threshold(Some(8_192.0))
    .with_measured_tokens(measured_tokens)
}

fn main() {
    let temp = tempfile::tempdir().expect("create local temp directory");
    let store = HistoryStore::new(temp.path());
    let mut added_latency = Vec::with_capacity(DECISIONS);

    for index in 0..DECISIONS {
        // Model the no-History baseline as a typed result already produced by
        // the evaluator. The measured increment is History publication only.
        let event = decision(index);
        black_box(&event);

        let history_started = Instant::now();
        assert!(
            black_box(store.write_decision(&event)),
            "decision {index} write"
        );
        added_latency.push(history_started.elapsed());
    }

    let directories = fs::read_dir(store.root_dir())
        .expect("read local History root")
        .collect::<Result<Vec<_>, _>>()
        .expect("enumerate local History root");
    assert_eq!(
        directories.len(),
        DECISIONS + 1,
        "one directory per decision plus the shared sweep lease"
    );
    let mut decision_directories = 0;
    for entry in &directories {
        if entry.file_name() == std::ffi::OsStr::new(".sweep-lease") {
            assert!(
                entry.file_type().expect("inspect sweep lease").is_file(),
                "sweep lease is a file"
            );
            continue;
        }
        assert!(entry
            .file_type()
            .expect("inspect decision directory")
            .is_dir());
        decision_directories += 1;
        let children = fs::read_dir(entry.path())
            .expect("read decision directory")
            .collect::<Result<Vec<_>, _>>()
            .expect("enumerate decision directory");
        assert_eq!(children.len(), 1, "one file per decision directory");
        assert_eq!(
            children[0].file_name(),
            std::ffi::OsStr::new("decision.json"),
            "no per-execution append log or lock files are created"
        );
    }
    assert_eq!(decision_directories, DECISIONS);

    added_latency.sort_unstable();
    let p50 = percentile(&added_latency, 50, 100);
    let p95 = percentile(&added_latency, 95, 100);
    println!(
        "Hook History: decisions={DECISIONS}, added p50={:.3} ms, added p95={:.3} ms; local filesystem, one decision file per execution, no shared append lock",
        p50.as_secs_f64() * 1_000.0,
        p95.as_secs_f64() * 1_000.0,
    );
    assert!(
        p50 <= P50_BUDGET,
        "added p50 {p50:?} exceeds {P50_BUDGET:?}"
    );
    assert!(
        p95 <= P95_BUDGET,
        "added p95 {p95:?} exceeds {P95_BUDGET:?}"
    );
}
