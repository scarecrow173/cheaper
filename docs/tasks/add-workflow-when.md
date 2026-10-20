---
id: "TASK-034Jg6koI5aIWEPUXfhobu"
type: "task"
status: "done"
title: "Add workflow job when condition evaluation"
created: "2026-10-20"
updated: "2026-10-21"
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
    - "docs/plans/implement-pretooluse-route-resolver-and-delegated-workflow.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-pretooluse-route-resolver-and-delegated-workflow.md"
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
    - "docs/test-specs/workflow-when-condition.md"
  references: []
  defers: []
  deferred-by: []
---

# Add workflow job when condition evaluation

## Work

job `when` predicate を schema → compiler(dep edge 推論・`$from`
path 検証)→ engine(pump 評価・CONDITION_SKIPPED・skip 非伝播・
`$from`→null materialization・error 集約除外)まで貫通させる。
演算子意味論: exists=非 null、eq/ne=JSON 等価、gt/lt=数値のみ。
runtime binding 失敗は job failure。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-pretooluse-route-resolver-and-delegated-workflow.md (Phase A4)

## Dependencies

- なし(Phase A の独立項目)

## Affected Files

- crates/cheaper-workflow/src/schema.rs
- crates/cheaper-workflow/src/compiler.rs
- crates/cheaper-workflow/src/types.rs
- crates/cheaper-workflow/src/engine.rs
- crates/cheaper-workflow/tests/compile_contracts/

## Done When

- [ ] when=false の job が CONDITION_SKIPPED で skip される
- [ ] skip が dependent・workflow error 集約へ伝播しない
- [ ] condition-skipped producer への `$from` が null materialize
- [ ] binding 失敗が silent skip でなく job failure になる

## Verification

- [ ] compile_contracts: when schema・dep edge・`$from` path
- [ ] engine test: skip 伝播・null materialization・operator 意味論
