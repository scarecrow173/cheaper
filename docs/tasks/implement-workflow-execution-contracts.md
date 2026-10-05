---
id: "TASK-034AJcLMvVtCF2DauUO4It"
type: "task"
status: "blocked"
title: "Workflow policy/helper/compiler の正本を実装する"
created: "2026-10-05"
updated: "2026-10-06"
owners: ["scarecrow173"]
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
    - "../plans/implement-one-shot-workflow-execution-and-cli-integration.md"
  implemented-by: ["../impl/ir/implement-workflow-execution-contracts.md"]
  depends-on:
    - "../plans/implement-one-shot-workflow-execution-and-cli-integration.md"
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
    - "../test-specs/workflow-execution-contract-boundaries.md"
  references: []
  defers: []
  deferred-by: []
---

# Workflow policy/helper/compiler の正本を実装する

## Work

S1。WorkflowPolicy defaults/schema/min restriction、core literal writeScope helper、CompiledWorkflow.validateJobOutputを導入する。plan S1の順にcontract tests作成→RED実行→実装→GREEN→IR記録→論理commitを行う。

## Classification

- `normal-plan-task`

## Upstream Documents

- [承認済みplan](../plans/implement-one-shot-workflow-execution-and-cli-integration.md) と task verified-by の test-spec。
- [spec](../specs/one-shot-workflow-engine-and-run-cli.md)、[design](../designs/one-shot-workflow-execution-and-cli-composition.md)、[local ADR](../adr/compose-one-shot-workflows-with-injected-executors-and-invocation-owned-resource.md)。

## Dependencies

front matter depends-onのtask全てがdoneになってから開始する。source/contractsがdesignを満たせない場合は原因とconcreterevisionをcoordinatorへ渡す。

## Affected Files

core/src/policy/workflow.ts (new), schema.ts, load.ts, merge.ts, core/src/index.ts, core/src/worker/write-scope.ts (new); workflow/src/types.ts, compiler.ts; core/test/policy/workflow.test.ts (new), policy/schema.test.ts, policy/load.test.ts, policy/merge.test.ts, policy/restrict.test.ts, write-scope.test.ts (new); workflow/test/compiler.test.ts（各パス先頭 packages/）。

## Done When

- [ ] defaults/min/safe-integer/timeout narrowingとschema snapshotがcanonical boundaryで成立する。移設はsingle-worker既存挙動を維持する。
- [ ] IRにRED/GREEN/変更/reviewevidenceを記録し論理commitを作る。

## Verification

- [ ] `mise exec -- pnpm --filter @cheaper/core exec vitest run test/policy/workflow.test.ts test/policy/schema.test.ts test/policy/load.test.ts test/policy/merge.test.ts test/policy/restrict.test.ts test/write-scope.test.ts` exits 0。
- [ ] `mise exec -- pnpm --filter @cheaper/workflow exec vitest run test/compiler.test.ts` exits 0。

## Notes

2026-10-06: S1 source と focused checks は commit 済みだが、coordinator checkpoint review msg_33814be597e8 は計3回・各600000msの待機後も未回答。独立 review を自己代行せず blocked とする。S2以降の依存は未解決。承認後に IR/task を completed/done にして engine slice へ進む。

planのfilemap/commandsを正本とし、dependencies/rootconfig/policy.yaml/sharedADR/APMbundleは変更しない。nestedworkers無し、coordinatorreviewを新askで受ける。canonical check deferred to coordinator merge run。
