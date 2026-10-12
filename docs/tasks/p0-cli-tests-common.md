---
id: "TASK-034EDQ3rigtcVChwaRKAWD"
type: "task"
status: "done"
title: "T04: cheaper-cli tests common module"
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

# T04: cheaper-cli tests common module

## Work

cli の integration test 間で重複する helper を `tests/common/mod.rs` へ
集約する（spec Phase 0 cli 項目）。

- VecSink/PipeStdin/ambient_env/Fixture を 10 テストファイルの重複から集約

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/execute-post-feature-structural-refactoring.md`
- `docs/specs/post-feature-structural-refactoring-scope.md`

## Dependencies

- なし（tests/ 配下のみの変更で T03 とは独立）

## Affected Files

- `crates/cheaper-cli/tests/common/mod.rs`（新規）
- 重複 helper を持つ 10 個の `crates/cheaper-cli/tests/*.rs`

## Done When

- [ ] 4 helper が `tests/common` からのみ参照される
- [ ] 全テストが変更前と同じ挙動で pass

## Verification

- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` が pass
