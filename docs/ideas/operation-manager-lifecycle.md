---
id: "IDEA-034zSLUoRMHrYpLGc8wKPj"
type: "idea"
status: "promoted"
title: "OperationManager の invocation lifecycle"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/56"
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
  derived-by: ["../discovery/operation-manager-lifecycle.md"]
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# OperationManager の invocation lifecycle

## Summary

tool の lifecycle を Pi adapter の下に置き、invocation 内で追跡する。

## Problem and Motivation

- #56 は run abort と operation の停止観測を分離することを要求する。

## Expected Value

- Pi Worker と後続 Workflow の runtime consumer が同じ追跡契約を利用できる。

## Open Questions

- bounded drain 超過時に停止を偽装せず追跡する方法を discovery で比較する。

## Next Action

- [x] discovery に引き継ぐ。
