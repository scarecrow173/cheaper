---
id: "TASK-034r05iHWkNF7zFyMzc7zU"
type: "task"
status: "done"
title: "Add resolveUserConfigDir helper and Pi state path wiring"
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
  blocks:
    - "docs/tasks/wire-env-into-createmodelruntime-callers-and-update-e2e-isolation.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/pi-state-paths-under-userconfigdir-pi.md"
    - "docs/specs/pi-state-under-cheaper-config-dir.md"
    - "docs/adr/pi-persistent-state-userconfigdir-pi-modelruntime-create-path.md"
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

# TASK-034r05iHWkNF7zFyMzc7zU: Add resolveUserConfigDir helper and Pi state path wiring

## Work

DESIGN-034r05iHy2gdnoZAxhwPFa に従い、user-scope config dir の canonical 解決を
core に抽出し、`createModelRuntime()` が Pi state path を
`<userConfigDir>/pi/` へ組み立てるよう変更する。TDD で進める。

### 1. `packages/core/src/config/dirs.ts`

- `resolveUserConfigDir(opts: { env?; homedir? }): string` を
  新設。現行の解決規則（`CHEAPER_CONFIG_DIR` →
  `XDG_CONFIG_HOME/cheaper` → `<home>/.config/cheaper`）をその
  まま移す
- `resolveConfigDirs()` は `resolveUserConfigDir` に委譲する
  純粋 refactor（挙動不変）
- `packages/core/src/index.ts`（または config barrel）から
  export する

### 2. `packages/runtime-pi/src/model-runtime.ts`

- options 型を `CreateModelRuntimeOptions & { env?:
  NodeJS.ProcessEnv }` に拡張（明示 wrapper interface も可）
- `env` を剥がして `resolveUserConfigDir({ env })` を解決し、
  `<dir>/pi` を `mkdir({ recursive: true, mode: 0o700 })` で作成
- `ModelRuntime.create()` に
  `authPath: <dir>/pi/auth.json`・`modelsPath: <dir>/pi/
  models.json`・`modelsStorePath: <dir>/pi/models-store.json`
  を既定として渡し、caller options を後から spread する
  （caller 明示 path が優先）
- `PI_CODING_AGENT_DIR` は読まない・設定しない

### 3. unit test（`packages/runtime-pi/test/model-runtime.test.ts`）

- tmpdir を `CHEAPER_CONFIG_DIR` にした `env` で
  `createModelRuntime()` を呼び、`<dir>/pi/` が作成される
  こと・3 path が `<dir>/pi/` 配下で解決されることを assert
  （`ModelRuntime.create` の spy または file 作成で確認）
- caller が `authPath` 等を明示した場合に既定より優先される
  こと
- `resolveUserConfigDir` の core 側単体 test（既存の
  `resolveConfigDirs` テストがあればそこに追加）

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/pi-state-under-userconfigdir-pi-implementation-plan.md`
- `docs/designs/pi-state-paths-under-userconfigdir-pi.md`

## Dependencies

- なし（chain の先頭）

## Affected Files

- `packages/core/src/config/dirs.ts`
- `packages/core/src/index.ts`
- `packages/runtime-pi/src/model-runtime.ts`
- `packages/runtime-pi/test/model-runtime.test.ts`
- （core dirs の既存 test があれば同様に）

## Done When

- [x] `resolveUserConfigDir` が core から export され、
      `resolveConfigDirs` がそれに委譲している
- [x] `createModelRuntime({ env })` が `<userConfigDir>/pi/`
      配下の 3 path で `ModelRuntime.create` を呼ぶ
- [x] `pi/` dir が `0700` で作成される（POSIX）
- [x] caller 明示 path が既定を上書く

## Verification

- [x] `pnpm --filter @cheaper/core test`・
      `pnpm --filter @cheaper/runtime-pi test` green
- [x] `pnpm typecheck` green
