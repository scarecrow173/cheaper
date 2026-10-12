---
id: "TASK-034EDQJGStTXkFTSXPBshK"
type: "task"
status: "todo"
title: "T10: cheaper-cli compose module separation"
created: "2026-10-12"
updated: "2026-10-12"
owners: []
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
    - "docs/plans/execute-post-feature-structural-refactoring.md"
  implemented-by: []
  depends-on:
    - "docs/plans/execute-post-feature-structural-refactoring.md"
    - "docs/tasks/p0-cli-shared-helpers.md"
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
    - "docs/test-specs/post-feature-structural-refactoring-parity.md"
  references: []
  defers: []
  deferred-by: []
---

# T10: cheaper-cli compose module separation

## Work

cli の composition root を `compose.rs` へ分離する（spec Phase 1）。

- `compose.rs`（新規）: `create_runtime`・devin plugin 登録・session
  factory 配線・`resolve_named_worker`・workspace preamble（5 箇所重複の
  helper 化）を保持
- run.rs は command 本体のみ。models.rs/workflow_run.rs/auth.rs/
  usage_report.rs は `crate::compose` から参照
- dev-seam plumbing を `fake_stream.rs` 境界へ集約
- signal 系は T03 で `signal.rs` へ集約済み — compose からは参照のみ

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/execute-post-feature-structural-refactoring.md`
- `docs/designs/post-feature-structural-refactoring.md`

## Dependencies

- `docs/tasks/p0-cli-shared-helpers.md`（signal/errors helper が先行）

## Affected Files

- `crates/cheaper-cli/src/{compose.rs,run.rs,workflow_run.rs,models.rs,
  auth.rs,usage_report.rs,fake_stream.rs}`

## Done When

- [ ] composition root が compose.rs に集約され command 実装と分離
- [ ] envelope/error/exit code 差分なし

## Verification

- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` が pass
