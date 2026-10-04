---
id: "TASK-034A9ylSD9a2UjmTcNHl9R"
type: "task"
status: "done"
title: "managed process policy と resource 宣言を実装する"
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
    - "../plans/implement-managed-process-canonical-tool.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-managed-process-canonical-tool.md"
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
  references: []
  defers: []
  deferred-by: []
---

# managed process policy と resource 宣言を実装する

## Work

core に ProcessPolicy/task config resolver と pure resource 宣言を実装する。executor は公開しない。

## Classification

- `normal-plan-task`

## Upstream Documents

- approved plan と spec、accepted ADR。

## Dependencies

- plan-approval: msg_e3d23f7a3334。

## Affected Files

- core policy/process/schema/load/index、core policy tests、builtin policy ×2。

## Done When

- [ ] strict task config、legacy/default、trusted/project cap、ResourceManifest ownership が検証済み。

## Verification

- [ ] `mise exec -- pnpm exec vitest run packages/core/test/policy packages/core/test/resources.test.ts` exits 0。
- [ ] `mise exec -- pnpm lint` exits 0。
- [ ] `mise exec -- pnpm typecheck` exits 0。
- [ ] `mise exec -- pnpm build` exits 0。
