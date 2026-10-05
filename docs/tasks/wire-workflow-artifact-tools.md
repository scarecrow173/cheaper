---
id: "TASK-034AJcULdYBQSjyy18XSVw"
type: "task"
status: "todo"
title: "Canonical artifact read/publisher を配線する"
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
    - "../test-specs/workflow-artifact-publication-and-consumption.md"
  references: []
  defers: []
  deferred-by: []
---

# Canonical artifact read/publisher を配線する

## Work

S3。三つのexclusive read modes、owner-fixedpublisher、boundedcap+1publication、artifactnamespace/catalog無アクセス、borrowedportsを導入する。plan S3のRED/GREEN手順を使いexistingstoreを再実装しない。

## Classification

- `normal-plan-task`

## Upstream Documents

- [承認済みplan](../plans/implement-one-shot-workflow-execution-and-cli-integration.md) と task verified-by の test-spec。
- [spec](../specs/one-shot-workflow-engine-and-run-cli.md)、[design](../designs/one-shot-workflow-execution-and-cli-composition.md)、[local ADR](../adr/compose-one-shot-workflows-with-injected-executors-and-invocation-owned-resource.md)。

## Dependencies

front matter depends-onのtask全てがdoneになってから開始する。source/contractsがdesignを満たせない場合は原因とconcreterevisionをcoordinatorへ渡す。

## Affected Files

packages/tool-runtime/src/artifact.ts, read.ts, catalog.ts, registry.ts, invocation.ts, index.ts; packages/workflow/src/artifacts.ts; packages/tool-runtime/test/artifact-read.test.ts (new), runtime.test.ts; packages/workflow/test/artifacts.test.ts。

## Done When

- [ ] 既存workspace read互換、publication/consumerのcaps/deny/foreignref/cleanupが成立しprivatepreviewを自動公開しない。
- [ ] IRにRED/GREEN/変更/reviewevidenceを記録し論理commitを作る。

## Verification

- [ ] `mise exec -- pnpm --filter @cheaper/tool-runtime exec vitest run test/artifact-read.test.ts test/runtime.test.ts test/no-pi-deps.test.ts` exits 0。
- [ ] `mise exec -- pnpm --filter @cheaper/workflow exec vitest run test/artifacts.test.ts` exits 0。

## Notes

planのfilemap/commandsを正本とし、dependencies/rootconfig/policy.yaml/sharedADR/APMbundleは変更しない。nestedworkers無し、coordinatorreviewを新askで受ける。canonical check deferred to coordinator merge run。
