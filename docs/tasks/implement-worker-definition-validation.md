---
id: "TASK-034r05iHK8bA7gLHezkOrX"
type: "task"
status: "done"
title: "Implement worker definition validation"
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
    - "docs/tasks/implement-tool-vocabulary-and-alias-normalization.md"
    - "docs/tasks/implement-worker-schema-reference-resolution-and-ajv-validation.md"
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

# Implement worker definition validation

## Work

PLAN-034r05iHy2gdnpHMZZk4ZU step 9。frontmatter 検証: name `^[a-z0-9][a-z0-9-]*$`（`INVALID_WORKER_NAME`）、ファイル stem 一致（`WORKER_NAME_MISMATCH`）、`cheaper:` 配下は model/input/output のみ且つ各内部 additionalProperties false、top-level 未知 field は ignore（警告なし）、tools は vocabulary 正規化、schema refs 解決して inputSchema/outputSchema を WorkerDefinition に載せる。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/scaffold-monorepo-and-implement-cheaper-core.md`
- `docs/designs/worker-definition-resolver-runtime-policy-tool-vocabulary-error-model.md`
- `docs/specs/cheaper-v1-worker-runtime-cli.md`

## Dependencies

- `docs/tasks/implement-tool-vocabulary-and-alias-normalization.md` — 依存先（Dependency Graph）
- `docs/tasks/implement-worker-schema-reference-resolution-and-ajv-validation.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/core/src/worker/validate.ts`
- `packages/core/test/worker/validate.test.ts`

## Done When

- [ ] 失敗するテスト（または scaffold 検証コマンド）を書く
- [ ] 対象テストを実行し red を確認する
- [ ] 最小実装を行う
- [ ] 再実行で green を確認する
- [ ] `pnpm biome check` と `tsc -b` が pass
- [ ] name 規則・stem 一致・strict・alias・重複・tools non-empty の各テストが green

## Verification

- [ ] `mise exec -- pnpm --filter @cheaper/core test -- test/worker/validate.test.ts` exits 0
- [ ] `mise exec -- pnpm biome check .` exits 0
- [ ] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- resolveWorker の探索順は TASK-034r05iHK8bA7gN2lJtm5b。
