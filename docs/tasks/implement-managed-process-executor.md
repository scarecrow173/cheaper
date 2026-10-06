---
id: "TASK-034A9ymtnIQaZqcchZR6Bg"
type: "task"
status: "done"
title: "managed process executor と adapter を実装する"
created: "2026-10-04"
updated: "2026-10-05"
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
    - "../plans/implement-managed-process-canonical-tool.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-managed-process-canonical-tool.md"
    - "./implement-managed-process-policy.md"
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
    - "../test-specs/managed-process-task-authorization.md"
    - "../test-specs/managed-process-lifecycle.md"
  references: []
  defers: []
  deferred-by: []
---

# managed process executor と adapter を実装する

## Work

native spawn、bounded capture、tree cleanup、scheduler pool、catalog/Worker/Pi/Workflow 接続と guide を実装する。

## Classification

- `normal-plan-task`

## Upstream Documents

- approved plan/spec/design、accepted ADR と test-spec 二件。

## Dependencies

- task A の verified contract に依存。nested dispatch は使わない。

## Affected Files

- tool-runtime process/resolver/catalog/registry/invocation/scheduler/index、core vocabulary/worker predicate、Pi run/registry、workflow inference と各 focused tests、builtin policy ×2、guide。

## Done When

- [x] 実子と孫 cancel、bounded output、readonly pool/FIFO、Pi/Workflow 同一呼出し、producer authority が検証済み。
- [x] coordinator implementation review と scoped exit-audit が完了。

## Verification

- [x] `mise exec -- pnpm exec vitest run packages/tool-runtime/test packages/runtime-pi/test/process.test.ts packages/runtime-pi/test/tool-runtime-contract.test.ts packages/runtime-pi/test/tool-scheduler.test.ts packages/workflow/test` exits 0。
- [x] `mise exec -- pnpm lint` exits 0。
- [x] `mise exec -- pnpm typecheck` exits 0。
- [x] `mise exec -- pnpm build` exits 0。
