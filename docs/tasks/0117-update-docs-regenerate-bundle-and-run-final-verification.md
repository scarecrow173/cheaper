---
id: "TASK-0117"
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
    - "docs/plans/0024-pi-state-under-userconfigdir-pi-implementation-plan.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0024-pi-state-under-userconfigdir-pi-implementation-plan.md"
    - "docs/tasks/0116-wire-env-into-createmodelruntime-callers-and-update-e2e-isolation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/0029-pi-state-paths-under-userconfigdir-pi.md"
    - "docs/specs/0024-pi-state-under-cheaper-config-dir.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/0024-pi-state-under-cheaper-config-dir.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# TASK-0117: Update docs, regenerate bundle, and run final verification

## Work

保存場所の変更をユーザー向け docs に反映し、bundled CLI を
再生成して status sync・最終検証を行う。

### 1. docs 更新

- `docs/guides/providers.md`: 保存場所表の `models.json` /
  `auth.json` を `<userConfigDir>/pi/` 配下に更新。
  `PI_CODING_AGENT_DIR` 言及を新しい構造に合わせる
- `docs/guides/configuration.md`: env var 表の
  `PI_CODING_AGENT_DIR` 行を更新（cheaper はこの env を
  読まず `<userConfigDir>/pi/` を使う旨）
- `AGENTS.md`: 必要なら credential/state の場所を簡潔に追記

### 2. bundle 再生成

- `pnpm build` → `node scripts/bundle-cli.mjs` で
  `packages/apm/.apm/hooks/cli/bin/cheaper.mjs` を再生成
  （CI の bundle freshness check と一致させる）

### 3. status sync + 最終検証

- SPEC-0024 → `implemented`、PLAN-0024 → `completed`、
  TASK-0115/0116/0117 → `done`、IMPL record 作成
- `mise run check`（lint → check-version → typecheck → test
  → bundle freshness）が green

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0024-pi-state-under-userconfigdir-pi-implementation-plan.md`
- `docs/designs/0029-pi-state-paths-under-userconfigdir-pi.md`

## Dependencies

- TASK-0116（CLI/e2e 変更の完了に依存）

## Affected Files

- `docs/guides/providers.md`
- `docs/guides/configuration.md`
- `AGENTS.md`（必要なら）
- `packages/apm/.apm/hooks/cli/bin/cheaper.mjs`（生成物）
- 各 doc の status front matter + index

## Done When

- [x] docs の保存場所説明が実装と一致
- [x] bundle が最新 src から再生成されている
- [x] doc status が同期され IMPL record が存在する
- [x] `mise run check` green

## Verification

- [x] `mise run check` green
