---
id: "TASK-034AJcZY4VW7Wfwx1AOzvh"
type: "task"
status: "in-progress"
title: "Workflow Worker permission と lease lifetime を配線する"
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
  implemented-by:
    - "../impl/ir/workflow-worker-permission-lease-lifetime.md"
  depends-on:
    - "../plans/implement-one-shot-workflow-execution-and-cli-integration.md"
    - "./implement-workflow-execution-contracts.md"
    - "./wire-workflow-artifact-tools.md"
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
    - "../test-specs/workflow-worker-permissions-and-lease-lifetimes.md"
  references: []
  defers: []
  deferred-by: []
---

# Workflow Worker permission と lease lifetime を配線する

## Work

S4。ResourceAccess.prepareWorkerManifest、WorkerExecutionBinding narrowedmanifest、caller-ownedleasecleanup、Pihelperscope/journal配線を行う。blanketreleaseの原因をregressionで確認してplan S4 RED/GREENを実行する。

## Classification

- `normal-plan-task`

## Upstream Documents

- [承認済みplan](../plans/implement-one-shot-workflow-execution-and-cli-integration.md) と task verified-by の test-spec。
- [spec](../specs/one-shot-workflow-engine-and-run-cli.md)、[design](../designs/one-shot-workflow-execution-and-cli-composition.md)、[local ADR](../adr/compose-one-shot-workflows-with-injected-executors-and-invocation-owned-resource.md)。

## Dependencies

front matter depends-onのtask全てがdoneになってから開始する。source/contractsがdesignを満たせない場合は原因とconcreterevisionをcoordinatorへ渡す。

## Affected Files

packages/tool-runtime/src/access.ts, invocation.ts; packages/runtime-pi/src/run.ts, tools/registry.ts, index.ts; tool-runtime/test/invocation.test.ts, resources.test.ts; runtime-pi/test/resource-leases.test.ts, run.test.ts, tools/read.test.ts, tools/registry.test.ts（testは各先頭 packages/）。

## Done When

- [ ] childtoken有無/dispose前後のparentretention、literalwritepermission、single-workercleanup、childtimeoutisolationが成立する。
- [ ] IRにRED/GREEN/変更/reviewevidenceを記録し論理commitを作る。

## Verification

- [ ] `mise exec -- pnpm --filter @cheaper/tool-runtime exec vitest run test/invocation.test.ts test/resources.test.ts` exits 0。
- [ ] `mise exec -- pnpm --filter @cheaper/runtime-pi exec vitest run test/resource-leases.test.ts test/run.test.ts test/tools/read.test.ts test/tools/registry.test.ts` exits 0。

## Notes

planのfilemap/commandsを正本とし、dependencies/rootconfig/policy.yaml/sharedADR/APMbundleは変更しない。nestedworkers無し、coordinatorreviewを新askで受ける。canonical check deferred to coordinator merge run。
