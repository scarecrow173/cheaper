---
id: "IMPL-034MphdOSZ5smrEZgDgPfh"
type: "impl"
status: "completed"
title: "Implement usage byError aggregation issue 184 T3"
created: "2026-10-26"
updated: "2026-10-26"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/usage_report.rs"
      - type: "file"
        path: "docs/tasks/implement-usage-by-error-aggregation.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/implement-usage-by-error-aggregation.md"
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
  references:
    - "docs/plans/implement-usage-route-error-analysis-issue-184.md"
    - "docs/designs/usage-grouped-route-error-analysis-issue-184.md"
    - "docs/specs/usage-analysis-scope-issue-184.md"
    - "docs/test-specs/usage-route-error-analysis-contract-issue-184.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Implement usage byError aggregation issue 184 T3

## Summary

Implemented and independently reviewed `byError` aggregation for filtered run
records. T3 follows T2 commit `c7141c9`; T3 changes are ready for commit.

## Context

Follow the approved Issue #184 plan/design/spec and T3 task. The aggregation
must consume the same post-filter run records as the parent totals and must
ignore route scope, History files, raw error details, and execution IDs.

## Implementation

### Changes

- Added `ErrorAgg`/`build_by_error` with exact string-code grouping, missing or
  non-string code folding to `unknown`, failure/run shares, and deterministic
  count/code ordering.
- Added only allowlisted worker/model/reason/execution dimensions. Execution
  numbers use `cheaper_core::json::number::format_number`; array/object values
  are skipped. `byError` is emitted only for `UsageGroupBy::Error`.
- Updated module overview. No CLI flags, text rendering, History access,
  execution ID logic, or record writer changes.

### Decisions During Implementation

- No exploratory alternatives; implement the approved design directly.

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

- Nine focused tests passed: eight T3 pins plus
  `usage_same_trace_distinct_runs_counted`.
- `usage_report::`: 60 passed / 5 failed. All remaining failures are T4 CLI/text
  pins: `invalid_group_by_value_rejected_at_dispatch`,
  `route_filter_leaves_by_error_unscoped`,
  `route_flag_scopes_route_at_dispatch`,
  `group_by_last_wins_emits_by_error_only`, `format_text_parity`.
- `cargo fmt --all -- --check` and strict CLI clippy passed.
- Regression golden: 61/61 passed; regression directory has no diff.
- Independent cooperative and adversarial reviews: NO CANDIDATES.

## Risks

- CLI/text support is T4 and remains intentionally incomplete until that task.

## Follow-ups

- T3 review is complete. Commit this slice and route the graph to T4.
