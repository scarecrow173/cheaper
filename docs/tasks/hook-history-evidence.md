---
id: "TASK-034MGHDFIrWUJA0skPjUkQ"
type: "task"
status: "done"
title: "Record typed Hook decision evidence"
created: "2026-10-27"
updated: "2026-10-27"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-execution-history-for-issue-192.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-execution-history-for-issue-192.md"
    - "TASK-034MGH8FHwescRINQne4sz"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "TSPEC-034MGCCC9F0R4NwYCGFOlk"
  references: []
  defers: []
  deferred-by: []
---

# Record typed Hook decision evidence

## Work

Record typed Hook decision evidence at the authoritative evaluator/decision owner before native response rendering. Preserve advice-versus-execution meaning, dedup provenance, native responses, and best-effort behavior.

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-execution-history-for-issue-192.md`

## Dependencies

- `docs/tasks/execution-history-core.md` (`TASK-034MGH8FHwescRINQne4sz`)

## Affected Files

- `crates/cheaper-cli/src/hook/routing_policy.rs`, `hook/targets.rs`, `hook/eval_cache.rs`
- `crates/cheaper-cli/src/history/schema.rs` (coordinator-owned typed-field extension), `crates/cheaper-cli/Cargo.toml` (benchmark harness registration)
- `crates/cheaper-cli/src/history/store.rs` (coordinator-owned bounded retention sweep amortization found by the required Hook hot-path benchmark)
- `crates/cheaper-cli/tests/integration/main.rs`, `tests/integration/hook_history.rs`, `crates/cheaper-cli/benches/history_hook.rs`

## Done When

- [x] Hook Deny remains advice and Rewrite is not an actual Route run; evaluator/fallback and dedup provenance are distinguishable.
- [x] Hook events distinguish evaluator verdict, target-native verdict, supported finite fallback reason, and cache provenance without duplicating generic verdict; source-backed Search reasons never persist free-form text.
- [x] Native Hook response bytes and exit behavior remain unchanged when History succeeds or fails.
- [x] Local benchmark measures 1,000 representative decisions and meets plan-local p50 ≤5 ms/p95 ≤25 ms added latency on the final ADR-conformant HistoryStore source.

## Verification

- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook_history:: -- --list` lists 11 Hook History tests.
- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook_history::` — 11 passed, 0 failed, 343 filtered out.
- [x] Final `mise exec -- cargo bench -p cheaper-cli --bench history_hook` run on the final HistoryStore publication gate exited 0: 1,000 decisions, p50 1.131 ms, p95 1.371 ms.
- [x] The first harness attempt ran 0 tests. After switching the target to `harness = false`, the first real 1,000-write measurement exited 1 at p50 97.148 ms/p95 185.660 ms. A subsequent run exited 1 before percentile output because the new shared `.sweep-lease` file was counted as an execution directory; the assertion was corrected to count 1,000 decision directories plus the shared lease marker. The workload and timing thresholds were unchanged.
- [x] Managed task index regenerated; `doc_status.js lint --type task --rule index-metadata-mismatch --json` reports 0 findings across 416 documents. `audit_impl_record.js --json` reports 0 findings across 199 records.
