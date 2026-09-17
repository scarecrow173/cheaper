---
id: "TASK-0106"
type: "task"
status: "todo"
title: "Copilot adapter script, hooks.json registration, and stdin-injection tests"
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
    - "docs/plans/0020-copilot-pre-read-hook-adapter.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0020-copilot-pre-read-hook-adapter.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/0025-copilot-pre-read-hook-adapter.md"
    - "docs/specs/0020-copilot-pre-read-hook-adapter.md"
    - "docs/adr/0023-copilot-adapter-separate-script-never-nonzero-exit.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/0020-copilot-pre-read-hook-adapter.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# TASK-0106: Copilot adapter script, hooks.json registration, and stdin-injection tests

## Work

TDD で adapter を実装する。まず
`packages/cli/test/e2e/copilot-pre-read-hook.test.ts` を作成して
red を確認し、`packages/apm/.apm/hooks/pre-read-code-copilot.js` と
`hooks.json` の `preToolUse` 登録を実装して green にする。

### 1. test file（先に作成）

`pre-read-hook.test.ts` と同じ流儀（`spawnSync` で script に stdin
注入、`makeIsolatedEnv` で workspace 隔離）で以下を網羅:

- block: `{toolName:"view", toolArgs:{path}}` と
  `{toolArgs:"<JSON string>"}` の両 shape で threshold 超過 file が
  stdout deny JSON（`permissionDecision:"deny"`、reason に
  `bulk-read` + workspace-relative path）+ exit 0
- FILE_TOO_LARGE envelope 経路でも deny JSON + exit 0
- never-nonzero: 非 JSON stdin・`toolName` 違い・parse 不能な
  toolArgs string・path key 欠落・workspace 外・不存在 file・
  directory・CLI 不在 — すべて `status === 0`
- small file / threshold 内で exit 0・deny JSON なし
- fast path（bytes ≤ threshold で CLI spawn しない、fake CLI marker）

### 2. `pre-read-code-copilot.js`

DESIGN-0025 のフローで self-contained 実装（`pre-read-code.js` と
同じ判定骨格、stdin/stdout contract のみ差分）。常に exit 0、
block は stdout deny JSON。

### 3. `hooks.json`

`preToolUse` entry を追加（DESIGN-0025 の JSON どおり。
`pre_read_code` entry は無変更）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0020-copilot-pre-read-hook-adapter.md`
- `docs/designs/0025-copilot-pre-read-hook-adapter.md`
- `docs/specs/0020-copilot-pre-read-hook-adapter.md`
- `docs/adr/0023-copilot-adapter-separate-script-never-nonzero-exit.md`

## Dependencies

- なし（本 chain の先頭 task）

## Affected Files

- `packages/apm/.apm/hooks/pre-read-code-copilot.js`（新規）
- `packages/apm/.apm/hooks/hooks.json`
- `packages/cli/test/e2e/copilot-pre-read-hook.test.ts`（新規）

## Done When

- [ ] copilot shape stdin（object/string toolArgs）で threshold 超過
      file が stdout deny JSON + exit 0 で block される
- [ ] 全異常経路で `status === 0`（非 0 exit の不存在）が assert
      されている
- [ ] `hooks.json` に `preToolUse` 登録が追加され `pre_read_code`
      が無変更
- [ ] 新規 test file 全件 pass、既存 pre-read-hook test も pass

## Verification

- [ ] `pnpm --filter cheaper test -- copilot-pre-read-hook` pass
- [ ] `pnpm --filter cheaper test -- pre-read-hook` pass（回帰なし）
- [ ] `pnpm typecheck`・`pnpm lint` pass

## Verification results

（未実施）
