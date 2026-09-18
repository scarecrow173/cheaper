---
id: "TASK-034r05iHWkNF7zGlMzs17M"
type: "task"
status: "done"
title: "Wire env into createModelRuntime callers and update e2e isolation"
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
    - "docs/plans/pi-state-under-userconfigdir-pi-implementation-plan.md"
  implemented-by: []
  depends-on:
    - "docs/plans/pi-state-under-userconfigdir-pi-implementation-plan.md"
    - "docs/tasks/add-resolveuserconfigdir-helper-and-pi-state-path-wiring.md"
  blocks:
    - "docs/tasks/update-docs-regenerate-bundle-and-run-final-verification.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/pi-state-paths-under-userconfigdir-pi.md"
    - "docs/specs/pi-state-under-cheaper-config-dir.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/pi-state-under-cheaper-config-dir.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# TASK-034r05iHWkNF7zGlMzs17M: Wire env into createModelRuntime callers and update e2e isolation

## Work

TASK-034r05iHWkNF7zFyMzc7zU の factory 変更を CLI 経路へ配線し、e2e の隔離を
`PI_CODING_AGENT_DIR` から `CHEAPER_CONFIG_DIR` へ置き換える。

### 1. CLI 呼出し側

- `packages/cli/src/cli.ts` `executeRun`:
  `(hooks.createModelRuntime ?? createModelRuntime)({ env:
  io.env })`。hook 型を options を受ける形に合わせる
- `packages/cli/src/auth.ts` `executeAuth`: 同様に
  `{ env: io.env }` を渡す

### 2. e2e 隔離の更新

- `packages/cli/test/e2e/helpers.ts` `makeIsolatedEnv`:
  `PI_CODING_AGENT_DIR` 行を削除（`PI_OFFLINE`・
  `CHEAPER_CONFIG_DIR` は維持）
- `packages/cli/test/e2e/delegation-loop.test.ts`: 同上 +
  worker 実行後に `<CHEAPER_CONFIG_DIR>/pi/` 配下へ state
  file が出ること・workspace `.cheaper/` へ credential が
  出ないことを assert
- `packages/cli/test/e2e/real-provider-ollama.test.ts`:
  `piAgent` dir を廃止し `models.json` を
  `<configDir>/pi/models.json` に書く。env から
  `PI_CODING_AGENT_DIR` を削除

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/pi-state-under-userconfigdir-pi-implementation-plan.md`
- `docs/designs/pi-state-paths-under-userconfigdir-pi.md`

## Dependencies

- TASK-034r05iHWkNF7zFyMzc7zU（factory の `env` seam に依存）

## Affected Files

- `packages/cli/src/cli.ts`
- `packages/cli/src/auth.ts`
- `packages/cli/test/e2e/helpers.ts`
- `packages/cli/test/e2e/delegation-loop.test.ts`
- `packages/cli/test/e2e/real-provider-ollama.test.ts`

## Done When

- [x] `run`・`auth` 双方が `{ env: io.env }` で runtime を生成
- [x] e2e が `PI_CODING_AGENT_DIR` を使わず
      `CHEAPER_CONFIG_DIR` のみで隔離される
- [x] `<CHEAPER_CONFIG_DIR>/pi/` に Pi state が出る e2e
      assert がある
- [x] workspace `.cheaper/` に credential が出ない assert
      がある

## Verification

- [x] `pnpm --filter cheaper test` green
- [x] `pnpm typecheck` green
