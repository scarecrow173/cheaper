---
id: "TASK-034zTslwNSSK73F5SbJfe6"
type: "task"
status: "in-progress"
title: "implement-workflow-loader"
created: "2026-10-04"
updated: "2026-10-04"
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
    - "docs/plans/implement-workflow-file-interface.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-workflow-file-interface.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: ["../test-specs/workflow-loader-contract.md"]
  references: []
  defers: []
  deferred-by: []
---

# Workflow loader と input API

## Work
loadWorkflowFile と validateInputs を既存 compiler 経路へ接続する。

## Classification
- normal-plan-task

## Upstream Documents
承認済み plan/spec/design、共有 ADR D1/D5/D6。

## Dependencies
#64 は main 統合済み。実装前に in-progress IR を開く。

## Affected Files
packages/workflow/src/{loader,compiler,index}.ts、packages/workflow/test/loader.test.ts。

## Done When
- [ ] file/dynamic 同一経路、defaults/missing/unknown/schema、file errors/boundary/deny がテスト済み。
- [ ] suite GREEN、IR 完了、論理 commit。

## Verification
- mise exec -- pnpm exec vitest run packages/workflow/test
- mise exec -- pnpm typecheck
- 新規 loader のレビューで parallel schema 検証がないことを確認。
