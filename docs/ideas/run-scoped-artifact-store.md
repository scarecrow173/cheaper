---
id: "IDEA-034zSPC3SGPGIHidq3x7sp"
type: "idea"
status: "promoted"
title: "Run-scoped artifact store"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/66"]
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
  derived-by: ["DISC-034zSQKiU63uXPcMSjjodn"]
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Run-scoped artifact store

## Idea

大きな workflow 中間出力を prompt に全文挿入せず、invocation 内の reference で受け渡す。

## Problem and Beneficiaries

Workflow Engine と read 系 consumer が、大きな tool/process 出力を metadata と bounded preview で扱えるようにする。

## Open Questions

run 間分離、storage bounds、borrowed reader の cleanup を discovery で整理する。

## Next Action

run-scoped-artifact-boundaries discovery へ昇格する。
