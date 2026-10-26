---
id: "TASK-034MGHDFIrWUJA0skPjUkQ"
type: "task"
status: "todo"
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
- `crates/cheaper-cli/tests/integration/hook_history.rs`, `crates/cheaper-cli/benches/history_hook.rs`

## Done When

- [ ] Hook Deny remains advice and Rewrite is not an actual Route run; evaluator/fallback and dedup provenance are distinguishable.
- [ ] Native Hook response bytes and exit behavior remain unchanged when History succeeds or fails.
- [ ] Local benchmark measures 1,000 representative decisions and meets plan-local p50 ≤5 ms/p95 ≤25 ms added latency.

## Verification

- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook_history:: -- --list` lists tests; then run the same filter without `-- --list` and `mise exec -- cargo bench -p cheaper-cli --bench history_hook`.
