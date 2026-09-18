---
id: "TASK-034r05iHWkNF7zPe6DL42d"
type: "task"
status: "done"
title: "Devin provider docs, status sync, bundle, and final verification"
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
    - "docs/plans/devin-local-provider-plugin-implementation-plan.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/devin-provider-plugin-registration-and-tests.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/devin-local-provider-plugin.md"
    - "docs/specs/devin-local-bundled-provider.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/devin-local-bundled-provider.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# TASK-034r05iHWkNF7zPe6DL42d: Devin provider docs, status sync, bundle, and final verification

## Work

### 1. `docs/guides/providers.md` — Devin セクション

- install prerequisite: Devin CLI（`devin` on PATH または
  `DEVIN_CLI` env）
- login 方法: `cheaper auth login devin`（Devin CLI 自身の
  flow が走る）
- credential location:
  `<userConfigDir>/providers/devin/data/devin/credentials.toml`
  — global `~/.local/share/devin`・Devin Desktop credential
  は利用しない
- catalog: `devin models list --format json` が live source
  （bootstrap の fallback list は最小限）
- experimental/compatibility caveat（protocol は vendored
  upstream 実装由来、Windows の credential path は未検証）
- `DEVIN_API_SERVER_URL` override

### 2. doc status sync

- SPEC-034r05iHK8bA7gAHzYnfP8 → `implemented`、PLAN-034r05iHK8bA7g45xvDT4a → `completed`、
  TASK-034r05iHWkNF7zNyXgVkrI〜0123 → `done`（checkbox も同期）
- 各 index README（specs/plans/tasks）の status 列を同期
- `docs/impl/ir/` に IMPL record を追加

### 3. bundle 再生成

- `node scripts/bundle-cli.mjs` で
  `packages/apm/.apm/hooks/cli/bin/cheaper.mjs` を再生成

### 4. final verification

- `mise run check`

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/devin-local-provider-plugin-implementation-plan.md`
- `docs/designs/devin-local-provider-plugin.md`

## Dependencies

- TASK-034r05iHWkNF7zNyXgVkrI / TASK-034r05iHWkNF7zNWvK4AnL

## Affected Files

- `docs/guides/providers.md`
- `packages/apm/.apm/hooks/cli/bin/cheaper.mjs`（生成物）
- 各 doc status + index
- `docs/impl/ir/`（新規 IMPL record）

## Done When

- [x] providers.md に prerequisite・login・credential
      location・caveat が記載されている
- [x] bundle が最新 src から再生成されている
- [x] doc status が同期され IMPL record が存在する
- [x] `mise run check` green

## Verification

- [x] `mise run check` green
