---
id: "TASK-0018"
type: "task"
status: "todo"
title: "Implement models.yaml config schema and merge"
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
    - "docs/plans/0001-scaffold-monorepo-and-implement-cheaper-core.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0001-scaffold-monorepo-and-implement-cheaper-core.md"
    - "docs/tasks/0017-implement-model-tier-ordering.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/0001-scaffold-monorepo-and-implement-cheaper-core.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Implement models.yaml config schema and merge

## Work

PLAN-0001 step 18。`models.yaml`（user/project）: `version: 1` 必須、`overrides`（"<provider>/<modelId>": tier）・`preferences.<tier>`（"<provider>/<modelId>" または "<provider>/*"）、未知 field → `INVALID_MODEL_CONFIG`。merge は policy と同規則（overrides deep merge・preferences array replace）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0001-scaffold-monorepo-and-implement-cheaper-core.md`
- `docs/designs/0003-model-resolver.md`

## Dependencies

- `docs/tasks/0017-implement-model-tier-ordering.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/core/src/model/config.ts`
- `packages/core/test/model/config.test.ts`

## Done When

- [ ] 失敗するテスト（または scaffold 検証コマンド）を書く
- [ ] 対象テストを実行し red を確認する
- [ ] 最小実装を行う
- [ ] 再実行で green を確認する
- [ ] `pnpm biome check` と `tsc -b` が pass
- [ ] version 必須・未知 field 拒否・merge 規則のテストが green

## Verification

- [ ] `mise exec -- pnpm --filter @cheaper/core test -- test/model/config.test.ts` exits 0
- [ ] `mise exec -- pnpm biome check .` exits 0
- [ ] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- 実際のモデル一覧取得は runtime-pi の model-runtime（TASK-0036）。
