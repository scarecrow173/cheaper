---
id: "TASK-034r05iHK8bA7gX5ANC9Qn"
type: "task"
status: "done"
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
    - "docs/plans/scaffold-monorepo-and-implement-cheaper-core.md"
  implemented-by: []
  depends-on:
    - "docs/plans/scaffold-monorepo-and-implement-cheaper-core.md"
    - "docs/tasks/implement-model-tier-ordering.md"
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

# Implement models.yaml config schema and merge

## Work

PLAN-034r05iHy2gdnpHMZZk4ZU step 18。`models.yaml`（user/project）: `version: 1` 必須、`overrides`（"<provider>/<modelId>": tier）・`preferences.<tier>`（"<provider>/<modelId>" または "<provider>/*"）、未知 field → `INVALID_MODEL_CONFIG`。merge は policy と同規則（overrides deep merge・preferences array replace）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/scaffold-monorepo-and-implement-cheaper-core.md`
- `docs/designs/model-resolver.md`

## Dependencies

- `docs/tasks/implement-model-tier-ordering.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/core/src/model/config.ts`
- `packages/core/test/model/config.test.ts`

## Done When

- [x] 失敗するテスト（または scaffold 検証コマンド）を書く
- [x] 対象テストを実行し red を確認する
- [x] 最小実装を行う
- [x] 再実行で green を確認する
- [x] `pnpm biome check` と `tsc -b` が pass
- [x] version 必須・未知 field 拒否・merge 規則のテストが green

## Verification

- [x] `mise exec -- pnpm --filter @cheaper/core test -- test/model/config.test.ts` exits 0
- [x] `mise exec -- pnpm biome check .` exits 0
- [x] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- 実際のモデル一覧取得は runtime-pi の model-runtime（TASK-034r05iHK8bA7hj3YNSFb8）。
