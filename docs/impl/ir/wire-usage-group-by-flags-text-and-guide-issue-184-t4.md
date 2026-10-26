---
id: "IMPL-034MpqYmnpqh3br76LERKo"
type: "impl"
status: "completed"
title: "Wire usage group by flags text and guide issue 184 T4"
created: "2026-10-26"
updated: "2026-10-26"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/args.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/cli.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/usage_report.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/text_format.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/args.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/usage_report.rs"
      - type: "file"
        path: "docs/guides/usage.md"
      - type: "file"
        path: "docs/tasks/wire-usage-group-by-flags-text-and-guide.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/wire-usage-group-by-flags-text-and-guide.md"
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
    - "docs/guides/usage.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Wire usage group by flags text and guide issue 184 T4

## Summary

Wired `--group-by`/`--route` through args and usage dispatch, added text
sections and updated the user guide. T4 tests and required checks passed.

## Context

Implements approved Issue #184 T4 after T2/T3 aggregation commits. New keys are
flag-gated in JSON, while route summary lines are always present in text.

## Implementation

### Changes

- Registered options with last-wins behavior, usage-only guards, route action
  allowlist rejection, and dispatch validation for `route|error`.
- Passed route/group options into the report builder and added the two help
  lines required by design.
- Added route summary, window diagnostics, `byRoute`, and `byError` text
  sections, including empty section headers and non-empty detail lines.
- Updated `usage_command_format_text_writes_report` and synchronized the usage
  guide with route windows, grouping, filter scopes, periods, and denominator
  meanings.

### Decisions During Implementation

- Kept the report JSON regression golden unchanged; the text format receives
  the specified additive route lines.
- Independent review found a P2 text rendering edge case: `count_detail()`
  replaces `=` throughout an execution key with `:`. An allowed string key
  containing `=` is therefore altered in text output. Both reviewers confirmed
  the scenario; it is non-blocking and left for follow-up under review-protocol.

## Related Experiments

### Adopted

<!-- Record adopted experiment logs and why they were adopted. -->

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

- `usage_report::`: 65 passed / 0 failed; `args::`: 47 passed / 0 failed;
  `usage_log::`: 21 passed / 0 failed.
- strict CLI clippy and `cargo fmt --all -- --check` passed.
- regression golden 61/61 passed; regression directory has no diff.
- `format_text_parity` passed for specified numeric/worker/model/reason and
  execution echo examples.
- Independent cooperative/adversarial review converged on the same P2 edge
  case above; no blocking findings.

## Risks

- Text output modifies `=` within allowed execution string keys, as described
  above. Current contract tests pass; this affects only those uncommon keys.

## Follow-ups

- Run T5 bounded JSONL peak-memory measurement, then T6 full verification and
  document-graph closeout.
