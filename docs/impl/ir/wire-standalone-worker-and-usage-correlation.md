---
id: "IMPL-034MIWwhe522nQ1s8Qz4UQ"
type: "impl"
status: "completed"
title: "Wire standalone Worker and Usage correlation"
created: "2026-10-26"
updated: "2026-10-27"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified:
      - { type: file, path: crates/cheaper-cli/src/cli.rs }
      - { type: file, path: crates/cheaper-cli/src/run.rs }
      - { type: file, path: crates/cheaper-cli/src/usage_log.rs }
      - { type: file, path: crates/cheaper-cli/tests/integration/history.rs }
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/execution-history-worker-usage.md"
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

# Wire standalone Worker and Usage correlation

## Summary

Implemented best-effort History lifecycle recording for standalone Worker runs
and additive execution correlation on the existing Worker Usage row. The
effective policy echo remains independent from identity context.

## Context

Followed `docs/tasks/execution-history-worker-usage.md`, the approved Issue 192
plan/design, and `TSPEC-034MGCxlMYuOemMCKIx9RZ`. The standalone boundary starts
after config-dir selection and before worker resolution. Utility and parser
paths remain outside the lifecycle.

## Implementation

### Changes

`run.rs` now writes a Worker `started` event before resolution and attempts a
terminal event for every returned result. Both writes are best-effort. The
same local execution and trace IDs are passed to managed children as explicit
correlation hints. `cli.rs` carries an independent typed `UsageCorrelation`
beside `ExecutionEcho`, and `usage_log.rs` projects its optional IDs only onto
existing `kind:"run"` rows. Count rows are unchanged. Added integration
coverage for pre-resolution failure lifecycle, inherited unverified hints,
and a successful run whose History and Usage IDs match.

### Decisions During Implementation

Kept History identity separate from the effective execution-policy echo so
Usage correlation cannot change the existing policy projection. Preserved the
old `record_usage` call for count-tokens and added a context-aware path for
Worker success and error recording. Error status is derived from the public
`CheaperError` source chain; no private Worker error type is exposed.
Inherited parent values remain `parentExecutionHintId` in History and are not
projected as a verified Usage `parentExecutionId`.

## Related Experiments

### Adopted

No experiment log was needed; the approved boundary and implementation path
were clear.

### Rejected

None.

## Validation

Passed:

- `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration history::` — 17 passed, including 2 new standalone Worker cases.
- `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration standalone_run_and_usage_share_one_execution_identity` — 1 passed after final inherited-hint ownership assertion.
- `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --lib execution_correlation_tests` — 2 passed.
- `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration run::` — 40 passed.
- `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test run_dev_seam` — 39 passed.
- `mise exec -- cargo fmt --check` — passed.
- `mise exec -- cargo clippy -p cheaper-cli --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam -- -D warnings` — passed.

## Risks

History and Usage remain independent best-effort writes; a failed write can
leave missing linkage while command output and accounting cardinality remain
unchanged.

## Follow-ups

No follow-up is required for this task slice.
