---
id: "TASK-034EDQIHsWPnLsqwvFQiFS"
type: "task"
status: "done"
title: "T08: cheaper-runtime catalog module split"
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
    - "docs/tasks/p0-runtime-provider-adapter-helpers.md"
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

# T08: cheaper-runtime catalog module split

## Work

`catalog/mod.rs`（1,372 行）を `catalog/{types,compose,store}.rs` へ分割
し、re-export で後方互換を保持する（spec Phase 1）。

- 型定義 → `types.rs`
- merge helper 9 本・compose 系 → `compose.rs`
- 永続化/読み込み → `store.rs`
- 公開 API（module path）は re-export で不変

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/execute-post-feature-structural-refactoring.md`
- `docs/designs/post-feature-structural-refactoring.md`

## Dependencies

- `docs/tasks/p0-runtime-provider-adapter-helpers.md`（同一ファイルを
  触る共通 ctor 化・コメント修正に先行依存）

## Affected Files

- `crates/cheaper-runtime/src/catalog/{mod.rs→types.rs,compose.rs,store.rs}`

## Done When

- [ ] catalog が責務単位に分割され re-export で公開 path が不変
- [ ] merge/store の挙動差分なし

## Verification

- [ ] `mise exec -- cargo test -p cheaper-runtime -p cheaper-cli`（dev-seam 付き）が pass
