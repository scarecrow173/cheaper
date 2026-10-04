---
id: "TASK-034zTsqC9tEk1qHUgWFefU"
type: "task"
status: "todo"
title: "wire-workflow-interface"
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
    - "docs/tasks/implement-workflow-loader.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: ["../test-specs/workflow-public-interface.md"]
  references: []
  defers: []
  deferred-by: []
---

# Workflow interface の CLI 接続

## Work
args/HELP/utility projection/error-path を接続し、unit/e2e で private 非公開を pin する。

## Classification
- normal-plan-task

## Upstream Documents
承認済み plan/spec/design、共有 ADR D6。

## Dependencies
loader/API task が done 後に着手する。実装前に in-progress IR を開く。

## Affected Files
packages/cli/src/{args,cli}.ts、packages/cli/test/workflow-interface.test.ts、packages/cli/test/e2e/workflow-interface.test.ts。

## Done When
- [ ] 既存 WorkflowInterface 型、option 限定、file errors、metadata precedence、非実行、JSON/text/exit が単独 test GREEN。
- [ ] lint/typecheck/build GREEN、IR 完了、scoped audit、論理 commit。

## Verification
- mise exec -- pnpm exec vitest run packages/cli/test/workflow-interface.test.ts
- mise exec -- pnpm exec vitest run packages/cli/test/e2e/workflow-interface.test.ts
- mise exec -- pnpm lint
- mise exec -- pnpm typecheck
- mise exec -- pnpm build
- canonical check deferred to coordinator merge run。
