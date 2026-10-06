---
id: "IDEA-034zSTSs8wPfzUDNrGraOY"
type: "idea"
status: "promoted"
title: "Workflow resource manifest と producer consumer 推論"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/65"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../discovery/workflow-resource-inference.md"

  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Workflow resource manifest と producer consumer 推論

## Summary
Workflow が output ownership と既存 state への mutation を区別し、consumer が読む出力から依存を推論できるようにする。

## Problem and Motivation
#64 は ResourceInference port のみで、resource 宣言から実際の依存や conflict を構築できない。

## Expected Value
親 Coding Agent と後続 Workflow engine が安全性を保ちながら手書き needs を減らせる。

## Open Questions
tool metadata、Worker writeScope、将来 process task の宣言をどう port に接続するか。

## Next Action
discovery の責務調査と spec/design の coordinator review へ進める。
