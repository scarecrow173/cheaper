---
id: "TASK-034AJd4IRjNpoxGSXgU36n"
type: "task"
status: "in-progress"
title: "Workflow run CLI と public envelope を統合する"
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
    - "../impl/ir/workflow-run-cli-public-envelope.md"
  depends-on:
    - "../plans/implement-one-shot-workflow-execution-and-cli-integration.md"
    - "./implement-injected-workflow-engine.md"
    - "./wire-workflow-artifact-tools.md"
    - "./wire-workflow-worker-leases.md"
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
    - "../test-specs/workflow-run-cli-public-projection.md"
  references: []
  defers: []
  deferred-by: []
---

# Workflow run CLI と public envelope を統合する

## Work

S5。metadatahelper共用/lazyruntime-store-model/executors/publicenvelope/text/usageguardを導入する。parsed.tokens、pre-engineerrorcatchkind、enginetimer起点を維持しplan S5のRED/GREENとbuiltCLI E2Eを実行する。

## Classification

- `normal-plan-task`

## Upstream Documents

- [承認済みplan](../plans/implement-one-shot-workflow-execution-and-cli-integration.md) と task verified-by の test-spec。
- [spec](../specs/one-shot-workflow-engine-and-run-cli.md)、[design](../designs/one-shot-workflow-execution-and-cli-composition.md)、[local ADR](../adr/compose-one-shot-workflows-with-injected-executors-and-invocation-owned-resource.md)。

## Dependencies

front matter depends-onのtask全てがdoneになってから開始する。source/contractsがdesignを満たせない場合は原因とconcreterevisionをcoordinatorへ渡す。

## Affected Files

packages/cli/src/workflow-context.ts, workflow-run.ts (new), args.ts, cli.ts, envelope.ts, text-format.ts, usage-log.ts; cli/test/workflow-run.test.ts (new), workflow-interface.test.ts, args.test.ts, input.test.ts, envelope.test.ts, text-format.test.ts, usage-log.test.ts, main.test.ts, e2e/workflow-run.test.ts (new)（testは各先頭 packages/）。

## Done When

- [ ] input/defaults/stdin/path/timeout/lazymodel/partialfalse/privacy/text/usageの契約とexistingrun/interfaceが成立する。
- [ ] IRにRED/GREEN/変更/reviewevidenceを記録し論理commitを作る。

## Verification

- [ ] `mise exec -- pnpm --filter @cheaper/cli exec vitest run test/workflow-run.test.ts test/workflow-interface.test.ts test/args.test.ts test/input.test.ts test/envelope.test.ts test/text-format.test.ts test/usage-log.test.ts test/main.test.ts` exits 0。
- [ ] `mise exec -- pnpm build` exits 0。
- [ ] `mise exec -- pnpm --filter @cheaper/cli exec vitest run test/e2e/workflow-run.test.ts test/e2e/workflow-interface.test.ts` exits 0。

## Notes

planのfilemap/commandsを正本とし、dependencies/rootconfig/policy.yaml/sharedADR/APMbundleは変更しない。nestedworkers無し、coordinatorreviewを新askで受ける。canonical check deferred to coordinator merge run。
