---
id: "TASK-034EDQ1S6FcEJLJQze19Ey"
type: "task"
status: "done"
title: "T01: cheaper-core shared helpers consolidation"
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

# T01: cheaper-core shared helpers consolidation

## Work

core crate の重複知識を一権威化する（spec Phase 0 core 項目）。

- `NAME_PATTERN`/`name_re`（`resolve.rs`/`validate.rs` の正規表現重複）を単一定義へ
- `schema_error_entries`（3 箇所 verbatim 重複）を共用 helper 化
- absent-file 判定（`policy/load.rs`・`model/config.rs` の重複）を helper 化
- `MAX_SAFE_INTEGER` を `js.rs` 正源から導出（named 定数 5 箇所:
  `policy/{process,schema,workflow}.rs`・`tools/ls.rs` + `catalog.rs`・
  `engine.rs` の inline literal `9007199254740991` 系も含む）

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/execute-post-feature-structural-refactoring.md` — 実装対象 plan
- `docs/specs/post-feature-structural-refactoring-scope.md` — Phase 0 core 項目
- `docs/designs/post-feature-structural-refactoring.md` — 配置指針

## Dependencies

- なし（Phase 0 起点 task）

## Affected Files

- `crates/cheaper-core/src/{js.rs,resolve.rs,worker/validate.rs,
  policy/{load.rs,process.rs,schema.rs,workflow.rs},model/config.rs}`
- caller 側: `crates/cheaper-tools/src/tools/{ls.rs,catalog.rs}`・
  `crates/cheaper-workflow/src/engine.rs`（inline literal の正源化）

## Done When

- [ ] 重複する helper/定数が core の一権威実装を参照している
- [ ] 公開 API 名・error reason 文字列・挙動が変化していない

## Verification

- [ ] `mise exec -- cargo test -p cheaper-core -p cheaper-tools -p cheaper-workflow --features cheaper-runtime/dev-seam` が pass
