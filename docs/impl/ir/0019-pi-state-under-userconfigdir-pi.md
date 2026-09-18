---
id: "IMPL-0019"
type: "impl"
status: "completed"
title: "Pi state paths under userConfigDir/pi (PLAN-0024)"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "file"
        path: "packages/core/src/config/dirs.ts"
      - type: "file"
        path: "packages/core/test/config/dirs.test.ts"
      - type: "file"
        path: "packages/runtime-pi/src/model-runtime.ts"
      - type: "file"
        path: "packages/runtime-pi/src/index.ts"
      - type: "file"
        path: "packages/runtime-pi/test/model-runtime.test.ts"
      - type: "file"
        path: "packages/cli/src/cli.ts"
      - type: "file"
        path: "packages/cli/src/auth.ts"
      - type: "file"
        path: "packages/cli/test/e2e/helpers.ts"
      - type: "file"
        path: "packages/cli/test/e2e/delegation-loop.test.ts"
      - type: "file"
        path: "packages/cli/test/e2e/real-provider-ollama.test.ts"
      - type: "file"
        path: "docs/guides/providers.md"
      - type: "file"
        path: "docs/guides/configuration.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "packages/apm/.apm/hooks/cli/bin/cheaper.mjs"
  implements:
    - "docs/tasks/0115-add-resolveuserconfigdir-helper-and-pi-state-path-wiring.md"
    - "docs/tasks/0116-wire-env-into-createmodelruntime-callers-and-update-e2e-isolation.md"
    - "docs/tasks/0117-update-docs-regenerate-bundle-and-run-final-verification.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0024-pi-state-under-userconfigdir-pi-implementation-plan.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/0029-pi-state-paths-under-userconfigdir-pi.md"
    - "docs/specs/0024-pi-state-under-cheaper-config-dir.md"
    - "docs/adr/0027-pi-persistent-state-userconfigdir-pi-modelruntime-create-path.md"
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
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Pi state paths under userConfigDir/pi (PLAN-0024)

## Summary

Pi `ModelRuntime` の永続 file（`auth.json` / `models.json` /
`models-store.json`）の保存先を、Pi の既定 dir（`~/.pi/agent`）
から cheaper の user-scope config dir 配下
（`<userConfigDir>/pi/`）へ移した。`createModelRuntime()` が
`env` seam 経由で `resolveUserConfigDir` を解決し、3 path を
`ModelRuntime.create()` へ明示的に渡す。`PI_CODING_AGENT_DIR`
への依存は排除し、テスト隔離は `CHEAPER_CONFIG_DIR` のみで
完結する。

## Context

TASK-0115/0116/0117（PLAN-0024 / SPEC-0024 / ADR-0027 /
DESIGN-0029）に従う。Issue #25。runtime 生成経路は CLI の
`executeRun`・`executeAuth` の 2 箇所のみのため、path policy
を factory 内に閉じることで「auth と worker が同じ credential
store を参照する」を構造的に保証した。

## Implementation

### Changes

- `packages/core/src/config/dirs.ts`: user-scope config dir の
  解決を `resolveUserConfigDir({ env?, homedir? })` として
  抽出。`resolveConfigDirs` は委譲する純粋 refactor（挙動
  不変）。`export *` 経由で自動 export。
- `packages/runtime-pi/src/model-runtime.ts`:
  - `CreateCheaperModelRuntimeOptions`（`CreateModelRuntimeOptions
    & { env?: ProcessEnv }`）を追加
  - `resolvePiStatePaths(userConfigDir)` を新設し、
    `<dir>/pi/` 配下の 3 path を 1 箇所で組み立てる
  - `createModelRuntime` は `env` を剥がして
    `resolveUserConfigDir` → `mkdir(piDir, { recursive,
    mode: 0o700 })` → `ModelRuntime.create({ authPath,
    modelsPath, modelsStorePath, ...rest })`。caller 明示
    path が spread 順序で既定を上書く
- `packages/cli/src/cli.ts`・`auth.ts`: 呼出しを
  `{ env: io.env }` に変更。hook 型を options 対応に拡張
- e2e: `PI_CODING_AGENT_DIR` を全箇所から除去し
  `CHEAPER_CONFIG_DIR` 隔離へ統一。real-provider e2e の
  `models.json` を `<configDir>/pi/models.json` に配置
- `delegation-loop.test.ts`: `<configDir>/pi/` の存在と
  workspace `.cheaper/` に credential 系 file が出ないことを
  assert 追加
- `docs/guides/providers.md`・`configuration.md`: 保存場所表と
  env var 表を新構造に更新

### Decisions / Notes

- `mkdir` 失敗は throw として表面化（Pi 既定 dir への silent
  fallback をしない fail-closed 方針、ADR-0027）。
- credential file の `0600` contract は Pi 側に委ね、cheaper は
  dir の `0700`（POSIX）のみ明示。Windows では `mode` は無視
  される。
- `~/.pi/agent` からの migration・fallback・暗黙 import は
  Issue #25 の方針どおり未実装。

## Verification

- `pnpm typecheck`: green
- `pnpm lint`（biome）: 新規 error/warning なし（既存 6
  warning のみ）
- `pnpm --filter @cheaper/core test`: 274 passed（
  `resolveUserConfigDir` 4 件追加）
- `pnpm --filter @cheaper/runtime-pi test`: 259 passed（
  `createModelRuntime`/`resolvePiStatePaths` 5 件追加 — spy で
  `ModelRuntime.create` への 3 path・`0700` dir・caller
  override 優先・HOME fallback を assert）
- `pnpm --filter cheaper test`: 全 155 件 pass。うち:
  - `delegation-loop.test.ts`: bundled CLI 実行後に
    `<CHEAPER_CONFIG_DIR>/pi/` へ state が出ること・
    `.cheaper/` に credential が出ないことを実測 assert
  - `real-provider-ollama.test.ts`: `<configDir>/pi/
    models.json` を読む bundled CLI が Ollama 実 provider で
    bulk-read/code-write/doc-write を完走（実測 usage 記録も
    assert）
- `mise run check`: lint/version/typecheck/test すべて green。
  bundle freshness は commit 後に working tree == HEAD で
  成立する（生成物を commit に含める）

## Acceptance Criteria 確認

- [x] `auth login` で作成される credential が user-scope
      config 配下に保存される（authPath 明示）
- [x] `auth status` / worker 実行が同じ credential store を
      参照（同一 factory）
- [x] `models.json` / `models-store.json` も config 配下
- [x] project-local `.cheaper/` に credential が保存されない
      （e2e assert）
- [x] `PI_CODING_AGENT_DIR` に依存しない（全参照を除去）
- [x] auth 関連テストが実 credential を触らない（tmpdir 隔離）
- [x] docs の保存場所説明が実装と一致
