---
id: "TASK-034r05iHWkNF7zLmXY8W9D"
type: "task"
status: "done"
title: "Update docs, regenerate bundle, and run final verification"
created: "2026-09-18"
updated: "2026-09-18"
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
    - "docs/plans/provider-only-registration-layer-implementation-plan.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/provider-plugin-tests-and-resource-isolation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/provider-only-registration-layer.md"
    - "docs/specs/provider-only-registration-layer.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/provider-only-registration-layer.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# TASK-034r05iHWkNF7zLmXY8W9D: Update docs, regenerate bundle, and run final verification

## Work

provider-only registration layer の user-facing 記述を docs に
反映し、bundle を再生成して canonical verification を通す。

### 1. `docs/guides/providers.md`

- bundled provider plugin の仕組みを追記:
  - Pi extension loader は有効化しない（Worker 環境最小化の
    制約維持）
  - cheaper が明示的に bundle する provider plugin のみが
    `ModelRuntime` に登録される
  - external provider package の discovery / load は未対応
- `cheaper auth` が plugin provider も通常 provider と同様に
  扱うことを明記

### 2. doc status sync

- SPEC-034r05iHK8bA7gz3bFqgXr → `implemented`、PLAN-034r05iHK8bA7g2BMsfjFu → `completed`、
  TASK-034r05iHWkNF7zJyX85ZOT〜0120 → `done`（checkbox も同期）
- 各 index README（specs/plans/tasks）の status 列を同期
- `docs/impl/ir/` に IMPL record を追加（#26 実装内容・
  検証結果・AC 確認）

### 3. bundle 再生成

- `node scripts/bundle-cli.mjs`（`pnpm bundle` 相当）で
  `packages/apm/.apm/hooks/cli/bin/cheaper.mjs` を再生成

### 4. final verification

- `mise run check`（lint + version + typecheck + test +
  bundle freshness）

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/provider-only-registration-layer-implementation-plan.md`
- `docs/designs/provider-only-registration-layer.md`

## Dependencies

- TASK-034r05iHWkNF7zJyX85ZOT / TASK-034r05iHWkNF7zKMZPhFky（実装と test の完了が前提）

## Affected Files

- `docs/guides/providers.md`
- `packages/apm/.apm/hooks/cli/bin/cheaper.mjs`（生成物）
- 各 doc の status front matter + index
- `docs/impl/ir/`（新規 IMPL record）

## Done When

- [x] providers.md が plugin 機構・extension 非対応を説明
- [x] bundle が最新 src から再生成されている
- [x] doc status が同期され IMPL record が存在する
- [x] `mise run check` green

## Verification

- [x] `mise run check` green
