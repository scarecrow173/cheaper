---
id: "IMPL-034MHTtoBvDNoO1W2QnjdC"
type: "impl"
status: "completed"
title: "Record typed Hook decision evidence"
created: "2026-10-27"
updated: "2026-10-27"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/cheaper-cli/benches/history_hook.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/hook_history.rs"
    modified:
      - type: "file"
        path: "crates/cheaper-cli/Cargo.toml"
      - type: "file"
        path: "crates/cheaper-cli/src/hook/eval_cache.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/hook/routing_policy.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/hook/targets.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/history/schema.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/history/store.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/main.rs"
      - type: "file"
        path: "docs/tasks/hook-history-evidence.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/hook-history-evidence.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Record typed Hook decision evidence

## Summary

Starting task `TASK-034MGHDFIrWUJA0skPjUkQ` from the approved implementation plan. The History API is available from the completed core task.

## Context

Follow the approved plan and Hook/Route decision-evidence TSPEC. Persist only authoritative Hook evidence while preserving native response bytes and advice-versus-execution semantics.

## Implementation

### Changes

- `hook::routing_policy` now returns a typed `HookEvaluation` alongside the existing `Verdict` compatibility API. It records finite read/search/route reasons, bounded measurements, validated route identity/viability, and route fallback evidence at the source branch.
- `hook::eval_cache` carries typed evidence through leader/follower results and records cache provenance. Cache hits do not write duplicate History decisions; Rewrite payloads remain unshared.
- `hook::targets` is the authoritative History writer. It records paired evaluator/native verdicts, supported fallback reason, measurements, validated route facts, and inherited carrier hints before native rendering; persistence remains best-effort.
- Added 11 named Hook History integration tests and a local 1,000-decision benchmark. `Cargo.toml` registers the benchmark with `harness = false` so its timing entrypoint executes.
- Updated the benchmark's post-run directory assertion to account for the shared `.sweep-lease` marker introduced by bounded cross-process retention throttling; it still verifies exactly 1,000 execution directories and one decision file in each.

### Decisions During Implementation

Use the typed evidence owner boundary; do not record Hook receipts as worker executions. Preserve Hook `Deny` as advice and Rewrite as emitted routing advice, not an executed Route run. Search reason codes come only from observed finite evaluator branches; query text and raw errors are not persisted. Route evidence remains meaningful when a supported resolver result cannot produce a rewrite, without mislabeling a runnable route as not runnable.

## Related Experiments

### Adopted

- The real 1,000-decision benchmark exposed per-write full History retention scans (p50 97.148 ms, p95 185.660 ms). The HistoryStore uses a stable OS-locked shared sweep lease with a 2-second interval. A successful bounded sweep records remaining directory/byte credits; each incoming start/decision consumes its actual event size under the lease. If the current incoming event exceeds remaining credits, the writer performs a bounded re-sweep under the same lease; failed, incomplete, pending, corrupt, or unknown capacity skips only the History publication, preserving command/Usage behavior. This preserves the ADR's best-effort targets rather than introducing a hard quota. Terminal writes remain limited to existing started directories and do not trigger retention; their bounded bytes may temporarily exceed the target until a later start/decision sweep removes excess. The OS releases the lease after process exit.

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

Task graph confirms Hook evidence is dependency-ready after History core.

- `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook_history:: -- --list` — exit 0; lists 11 tests.
- `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook_history::` — exit 0; 11 passed, 0 failed, 0 ignored, 343 filtered out (0.82 s).
- Scoped `rustfmt` and `git diff --check` passed.
- `doc-maintenance apply --type task` refreshed the managed task index; `doc-status lint --type task --rule index-metadata-mismatch --json` reported 0 findings across 416 documents. `audit_impl_record.js --json` reported 0 findings across 199 records.
- Final `mise exec -- cargo bench -p cheaper-cli --bench history_hook` on the reservation-credit HistoryStore exited 0 after 1,000 decisions with added p50 1.263 ms and p95 1.559 ms; both remain under the 5 ms/25 ms budgets. The prior 1.131/1.371 ms measurement was before the capacity-credit revalidation change and is retained only as a diagnostic.
- Earlier measurements are retained as diagnostic history: the default harness initially ran 0 tests; the first real loop measured p50 97.148 ms/p95 185.660 ms and exited 1; the first post-store-fix run completed its writes but exited 1 because the assertion counted `.sweep-lease` as an execution directory and printed no percentiles; an interim lease run measured p50 1.079 ms/p95 1.269 ms before the empty-marker initialization fix; the next source measured 1.218 ms/1.477 ms before the ADR-conformant publish gate. The workload and thresholds were not reduced.
- Store-focused tests on the reservation-credit state machine passed 17/17, including changed-incoming credit revalidation, no publish when bounded capacity cannot be established, shared cooldown, failed-capacity write suppression for started and decision events, bounded malformed marker rejection/retry, contention, no-create reads, and child-process holder-exit release. The child process smoke passed on Windows; `git diff --check` passed for the store implementation.
- One full workspace run stopped at `history_store_fresh_success_marker_rechecks_changed_incoming_capacity`: the fixture compared a marker timestamp written from wall-clock time to an earlier synthetic `now + 1ms`, incorrectly requiring it to be fresh. The assertion now checks the deterministic Failed marker state at the injected time and does not make a later write assertion whose outcome depends on the 2-second cooldown.
- A parallel Store17 run exposed that `history_store_allows_independent_concurrent_execution_publications` expected all 8 concurrent best-effort writes to succeed, despite accepted lock-contention skips. The test now verifies unique execution IDs, complete readable records for admitted writes, no partial event for skipped writes, and at least one admission. The revised test passed 3/3 exact sequential repetitions; the full Store17 suite passed afterward.

## Risks

The final benchmark meets the latency budget with the accepted HistoryStore write-suppression contract. The process-level lock smoke ran on Windows; POSIX behavior will be exercised by Ubuntu CI.

## Follow-ups

The Hook task is complete; continue with final integration verification.
