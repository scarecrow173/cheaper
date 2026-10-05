---
id: "TASK-034AJd9d4eo1B6Yt81QjdO"
type: "task"
status: "todo"
title: "Workflow 統合を検証して review/exit-audit を記録する"
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
    - "./integrate-workflow-run-cli.md"
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
    - "../test-specs/workflow-execution-verification-boundaries.md"
  references: []
  defers: []
  deferred-by: []
---

# Workflow 統合を検証して review/exit-audit を記録する

## Work

S6。plan V1-V6 focusedchecks、lint/typecheck/build、CLItest単独を実行し、coordinatorimplementationreviewを受けてownchain/sharedADRscopeauditを記録する。実行evidenceが無いscopeを成功と呼ばずcanonicalcheckをcoordinatormergeへdeferする。

## Classification

- `normal-plan-task`

## Upstream Documents

- [承認済みplan](../plans/implement-one-shot-workflow-execution-and-cli-integration.md) と task verified-by の test-spec。
- [spec](../specs/one-shot-workflow-engine-and-run-cli.md)、[design](../designs/one-shot-workflow-execution-and-cli-composition.md)、[local ADR](../adr/compose-one-shot-workflows-with-injected-executors-and-invocation-owned-resource.md)。

## Dependencies

front matter depends-onのtask全てがdoneになってから開始する。source/contractsがdesignを満たせない場合は原因とconcreterevisionをcoordinatorへ渡す。

## Affected Files

own docs/specs, docs/designs, docs/plans, docs/test-specs, docs/tasks, docs/impl/irのchainとgeneratedREADME; report。製品の追加変更はreviewfixで必要な範囲のみ。

## Done When

- [ ] focusedchecksとlint/typecheck/build/CLI単独がpassし、reviewverdictとscopedexit-auditの証拠が保存される。
- [ ] IRにRED/GREEN/変更/reviewevidenceを記録し論理commitを作る。

## Verification

- [ ] `mise exec -- pnpm lint` exits 0。
- [ ] `mise exec -- pnpm typecheck` exits 0。
- [ ] `mise exec -- pnpm build` exits 0。
- [ ] `mise exec -- node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` exits 0、own chainのblocking0（shared/globalの既存findingはreportへ区別して記録）。
- [ ] `mise exec -- node .agents/skills/impl-doc/scripts/audit_impl_record.js --json` exits 0、own chainのblocking0（shared/globalの既存findingはreportへ区別して記録）。

## Notes

planのfilemap/commandsを正本とし、dependencies/rootconfig/policy.yaml/sharedADR/APMbundleは変更しない。nestedworkers無し、coordinatorreviewを新askで受ける。canonical check deferred to coordinator merge run。
