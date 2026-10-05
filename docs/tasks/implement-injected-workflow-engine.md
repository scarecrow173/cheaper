---
id: "TASK-034AJcQgIUFFRXqpy0U1oB"
type: "task"
status: "todo"
title: "Injected one-shot workflow engine を実装する"
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
  implemented-by: []
  depends-on:
    - "../plans/implement-one-shot-workflow-execution-and-cli-integration.md"
    - "./implement-workflow-execution-contracts.md"
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
    - "../test-specs/workflow-one-shot-scheduling-and-cancellation.md"
  references: []
  defers: []
  deferred-by: []
---

# Injected one-shot workflow engine を実装する

## Work

S2。compiled edgesに基づくreadyqueue/非slotadmission/collect-all/outerabortとgrace/latefinallylease/publicprojectionを導入する。plan S2のRED/GREEN手順で実装し、engine portsの証拠をCLI compositionの証拠と混同しない。

## Classification

- `normal-plan-task`

## Upstream Documents

- [承認済みplan](../plans/implement-one-shot-workflow-execution-and-cli-integration.md) と task verified-by の test-spec。
- [spec](../specs/one-shot-workflow-engine-and-run-cli.md)、[design](../designs/one-shot-workflow-execution-and-cli-composition.md)、[local ADR](../adr/compose-one-shot-workflows-with-injected-executors-and-invocation-owned-resource.md)。

## Dependencies

front matter depends-onのtask全てがdoneになってから開始する。source/contractsがdesignを満たせない場合は原因とconcreterevisionをcoordinatorへ渡す。

## Affected Files

packages/workflow/src/engine.ts (new), index.ts; packages/workflow/test/engine.test.ts (new), no-pi-deps.test.ts。

## Done When

- [ ] dependency/fan-in/独立branch/capacity/privacy/output-validation/preabort/grace/late-settleの契約が成立する。
- [ ] IRにRED/GREEN/変更/reviewevidenceを記録し論理commitを作る。

## Verification

- [ ] `mise exec -- pnpm --filter @cheaper/workflow exec vitest run test/engine.test.ts test/no-pi-deps.test.ts test/resource-inference.test.ts` exits 0。

## Notes

planのfilemap/commandsを正本とし、dependencies/rootconfig/policy.yaml/sharedADR/APMbundleは変更しない。nestedworkers無し、coordinatorreviewを新askで受ける。canonical check deferred to coordinator merge run。
