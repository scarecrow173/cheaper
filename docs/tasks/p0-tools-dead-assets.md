---
id: "TASK-034EDQ2nAKA8yGvk57aniQ"
type: "task"
status: "todo"
title: "T02: cheaper-tools js16 shim and glob_probe removal"
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

# T02: cheaper-tools js16 shim and glob_probe removal

## Work

tools crate の dead 資産を解消する（spec Phase 0 tools 項目）。

- `src/js16.rs` shim を削除し caller（`grep.rs`・`find.rs`・`edit.rs`）を
  `cheaper_core` 直接参照へ更新
- `src/bin/glob_probe.rs` を削除（参照元の `probe-glob.cjs` は不在）
- differential fixture の frozen corpus である旨を明文化

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/execute-post-feature-structural-refactoring.md`
- `docs/specs/post-feature-structural-refactoring-scope.md`

## Dependencies

- なし（T01 の core helper とは独立 — js16 caller は既存の core export を参照）

## Affected Files

- `crates/cheaper-tools/src/js16.rs`（削除）
- `crates/cheaper-tools/src/bin/glob_probe.rs`（削除）
- `crates/cheaper-tools/src/tools/{grep.rs,find.rs,edit.rs}`
- differential fixture コメント/文書

## Done When

- [ ] js16 shim・glob_probe が削除され caller が core 直接参照
- [ ] fixture が frozen corpus と明記されている

## Verification

- [ ] `mise exec -- cargo test -p cheaper-tools`（dev-seam 付き）が pass
