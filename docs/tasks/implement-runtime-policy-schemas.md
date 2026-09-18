---
id: "TASK-034r05iHK8bA7gNEcUV7hq"
type: "task"
status: "done"
title: "Implement runtime policy schemas"
created: "2026-09-15"
updated: "2026-09-15"
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
    - "docs/plans/scaffold-monorepo-and-implement-cheaper-core.md"
  implemented-by: []
  depends-on:
    - "docs/plans/scaffold-monorepo-and-implement-cheaper-core.md"
    - "docs/tasks/implement-core-error-model-and-exit-code-table.md"
    - "docs/tasks/implement-duration-parser.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/scaffold-monorepo-and-implement-cheaper-core.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Implement runtime policy schemas

## Work

PLAN-034r05iHy2gdnpHMZZk4ZU step 11。Runtime Policy の complete schema（全 field 必須）と partial schema（`version` 以外任意）、両方 additionalProperties false 全階層。duration 文字列は `parseDuration` で `timeoutMs` に正規化。未知 field は `INVALID_RUNTIME_POLICY`。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/scaffold-monorepo-and-implement-cheaper-core.md`
- `docs/designs/worker-definition-resolver-runtime-policy-tool-vocabulary-error-model.md`
- `docs/specs/cheaper-v1-worker-runtime-cli.md`

## Dependencies

- `docs/tasks/implement-core-error-model-and-exit-code-table.md` — 依存先（Dependency Graph）
- `docs/tasks/implement-duration-parser.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/core/src/policy/schema.ts`
- `packages/core/test/policy/schema.test.ts`

## Done When

- [x] 失敗するテスト（または scaffold 検証コマンド）を書く
- [x] 対象テストを実行し red を確認する
- [x] 最小実装を行う
- [x] 再実行で green を確認する
- [x] `pnpm biome check` と `tsc -b` が pass
- [x] 必須/任意・additionalProperties・duration 正規化のテストが green

## Verification

- [x] `mise exec -- pnpm --filter @cheaper/core test -- test/policy/schema.test.ts` exits 0
- [x] `mise exec -- pnpm biome check .` exits 0
- [x] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- ロード順と merge は TASK-034r05iHK8bA7gPbLk4c5Q/0013。
