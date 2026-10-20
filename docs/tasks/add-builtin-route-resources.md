---
id: "TASK-034Jg6HoEn3yiLTP7PG0qz"
type: "task"
status: "todo"
title: "Add builtin route workflow condense worker and embedded resources"
created: "2026-10-20"
updated: "2026-10-20"
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
    - "docs/tasks/add-workflow-when.md"
    - "docs/tasks/add-route-declaration-loader.md"
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
    - "docs/test-specs/builtin-route-resources.md"
  references: []
  defers: []
  deferred-by: []
---

# Add builtin route workflow condense worker and embedded resources

## Work

`resources/routes.yaml`(enabled route 宣言)+
`resources/workflows/routed-exec.workflow.json`(exec + analyze
`when` condense)+ `resources/workers/condense.md` +
input/output schema を新設し、embedded.rs materialize と
`embedded_fingerprint` の resource 列挙を routes/workflows/worker
へ対称拡張する。analyze の pointer は `/details/outputTokens`。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-pretooluse-route-resolver-and-delegated-workflow.md (Phase D2)

## Dependencies

- docs/tasks/add-workflow-when.md (analyze `when` 前提)
- docs/tasks/add-route-declaration-loader.md (routes.yaml schema)

## Affected Files

- crates/cheaper-cli/resources/routes.yaml
- crates/cheaper-cli/resources/workflows/routed-exec.workflow.json
- crates/cheaper-cli/resources/workers/condense.{md,input.schema.json,output.schema.json}
- crates/cheaper-cli/src/embedded.rs

## Done When

- [ ] builtin routes.yaml が loader の schema を通る
- [ ] routed-exec workflow が compile を pass する
- [ ] materialize 後 builtin dir に routes/workflows/condense が存在
- [ ] fingerprint 列挙へ新規 resource が含まれる

## Verification

- [ ] embedded materialize contract test + workflow compile test +
      builtin_workers assertion 更新
