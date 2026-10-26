---
id: "IMPL-034MHsQwuzT8Ysc0cdXxZW"
type: "impl"
status: "completed"
title: "Add read-only History CLI and bounded queries"
created: "2026-10-27"
updated: "2026-10-27"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified:
      - { type: file, path: crates/cheaper-cli/src/args.rs }
      - { type: file, path: crates/cheaper-cli/src/cli.rs }
      - { type: file, path: crates/cheaper-cli/src/text_format.rs }
      - { type: file, path: crates/cheaper-cli/tests/integration/main.rs }
      - { type: file, path: crates/cheaper-cli/tests/integration/history.rs }
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/execution-history-query-cli.md"
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

# Add read-only History CLI and bounded queries

## Summary

Starting task `TASK-034MGHz9sAMkZkvZW0WFgZ` from the approved implementation plan. The bounded History read API is provided by the completed core task; this slice adds only the CLI surface and its contract tests.

## Context

Follow the approved plan, the execution-history design, and bounded query TSPEC. Keep list/show/trace read-only, return no cursor for incomplete scans, and fail filtered verification if no named tests are registered.

## Implementation

### Changes

Added History list/show/trace grammar, validated filters/limit/cursor, read-only dispatch, and JSON/text projections through the core query API. Cursor decoding occurs before file access; incomplete query responses omit continuation cursors. Registered four integration tests.

### Decisions During Implementation

CLI task ownership excludes `src/history/query.rs`; that file is owned by the completed core task. Integration registration is owned by this task.

## Related Experiments

### Adopted

No experiment log; the existing bounded History API supported the approved design directly.

### Rejected

None.

## Validation

Initial RED confirmed `history` was an unknown command. The required `history:: -- --list` listed 4 named tests. The final focused History integration run passed 4 tests, 0 failed, exit 0, including missing-store `show` no-write. Owned five files passed rustfmt and `git diff --check`. Whole-repo `cargo fmt --all --check` also reported changes in unowned Workflow files; those were left untouched.

## Risks

This slice was verified on Windows; full canonical check remains for final integration.

## Follow-ups

Task complete. The final independent branch review will cover cross-surface behavior and the full canonical verification.
