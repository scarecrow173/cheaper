---
id: "TASK-Qv0y8UiqVlBjROUfcjHGA6"
type: "task"
status: "done"
title: "worker prose の maxFindings null semantics 明記"
created: "2026-10-17"
updated: "2026-10-17"
owners:
  - scarecrow173
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/113"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/resources/workers/bulk-reader.md"
      - type: "file"
        path: "crates/cheaper-cli/resources/workers/tool-operator.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/worker-schema-structure-not-magnitude.md"
  implemented-by: []
  depends-on:
    - "docs/plans/worker-schema-structure-not-magnitude.md"
    - "docs/tasks/worker-schema-magnitude-removal.md"
  blocks:
    - "docs/tasks/regression-and-final-verification-113-114.md"
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
    - "docs/designs/worker-schema-structure-not-magnitude.md"
  defers: []
  deferred-by: []
---

# worker prose の maxFindings null semantics 明記

## Work

1. `crates/cheaper-cli/resources/workers/bulk-reader.md` と
   `tool-operator.md` の "Respect `maxFindings`/`output.maxFindings`
   when given" に、省略・`null` の場合は件数を model が判断する旨を
   明記する。
2. その他 worker `.md` の prose に schema 数値上限を前提とした記述が
   残っていないか確認し、あれば品質ガイダンス表現へ修正する。

## Done

- prose が schema 変更後の contract(null/省略 = 指示なし)と整合する。
