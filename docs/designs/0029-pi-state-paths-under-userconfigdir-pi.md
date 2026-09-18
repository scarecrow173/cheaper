---
id: "DESIGN-0029"
type: "design"
status: "approved"
title: "Pi state paths under userConfigDir/pi"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/0024-pi-state-under-cheaper-config-dir.md"
    - "../adr/0027-pi-persistent-state-userconfigdir-pi-modelruntime-create-path.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/0024-pi-state-under-cheaper-config-dir.md"
    - "../adr/0027-pi-persistent-state-userconfigdir-pi-modelruntime-create-path.md"
  implemented-by: []
  depends-on:
    - "./0005-cli-contract-envelope-exit-codes.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./0016-auth-command.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/0024-pi-state-under-cheaper-config-dir.md"
    - "../adr/0027-pi-persistent-state-userconfigdir-pi-modelruntime-create-path.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Pi state paths under userConfigDir/pi

## Context

SPEC-0024 / ADR-0027 の実装設計。`createModelRuntime()`（
`packages/runtime-pi/src/model-runtime.ts`）が Pi の永続
file の保存先を `<userConfigDir>/pi/` に固定する。runtime
生成経路は `cli.ts`（run 系）と `auth.ts`（auth 系）の
2 箇所のみで、path policy は factory 内に閉じる。

## Scope

### In Scope

- `packages/core/src/config/dirs.ts`: user-scope config dir の
  canonical 解決を `resolveUserConfigDir()` として抽出し、
  `resolveConfigDirs()` から再利用する形に整理
- `packages/runtime-pi/src/model-runtime.ts`:
  `createModelRuntime()` に `env` 注入 seam を追加し、
  `authPath`/`modelsPath`/`modelsStorePath` を
  `<userConfigDir>/pi/` へ解決して `ModelRuntime.create()`
  へ渡す。`pi/` dir を `mkdir -p`（POSIX `0700`）で作成
- `packages/cli/src/cli.ts` / `packages/cli/src/auth.ts`:
  `createModelRuntime()` 呼び出しに `env: io.env` を渡す
- `packages/cli/test/e2e/helpers.ts`・`delegation-loop.test.ts`・
  `real-provider-ollama.test.ts`: `PI_CODING_AGENT_DIR` 隔離を
  `CHEAPER_CONFIG_DIR` 隔離へ置き換え。real-provider e2e の
  `models.json` 配置を `<configDir>/pi/` へ移す
- `packages/runtime-pi/test/model-runtime.test.ts`:
  path 解決・dir 作成・呼出し options の unit test
- `docs/guides/providers.md`・`docs/guides/configuration.md`:
  保存場所記述の更新

### Out of Scope

- `~/.pi/agent` からの migration・fallback・暗黙 import
- `PI_CODING_AGENT_DIR` の読み取り・設定
- project-local `.cheaper/` への Pi state 配置
- Pi SDK 側の変更

## Components and Boundaries

- **`resolveUserConfigDir(opts)`**（core・新規 export）:
  `env.CHEAPER_CONFIG_DIR` → `env.XDG_CONFIG_HOME/cheaper` →
  `<home>/.config/cheaper` の順で user-scope dir を返す。
  `resolveConfigDirs()` はこの helper に委譲し、解決規則の
  表現を 1 箇所に保つ。
- **`resolvePiStatePaths(userConfigDir)`**（runtime-pi・
  module-local helper）: `authPath`/`modelsPath`/
  `modelsStorePath` の 3 path を組み立てる唯一の場所。
- **`createModelRuntime(options?)`**: 引数を
  `CreateModelRuntimeOptions & { env?: NodeJS.ProcessEnv }`
  に拡張。`env` を剥がして `resolveUserConfigDir` に渡し、
  3 path を既定として組み立てたうえで caller options を
  spread する（caller が明示した path が優先）。
- **CLI / auth caller**: `env: io.env` を渡すだけで、path
  を個別に知らない。

## Data and Control Flow

```text
cli.ts / auth.ts
  → createModelRuntime({ env: io.env })
    → resolveUserConfigDir({ env })          (core)
    → mkdir(<dir>/pi, { recursive, mode:0700 })
    → ModelRuntime.create({
        authPath:        <dir>/pi/auth.json,
        modelsPath:      <dir>/pi/models.json,
        modelsStorePath: <dir>/pi/models-store.json,
        ...callerOptions })
```

`auth status`/`login`/`logout`・`run` のいずれも同じ factory
を通るため、credential store の参照先は構造的に一致する。

## Non-Functional Constraints

- `pi/` dir は POSIX `0700`。Windows では `mode` は無視される
  （ACL ベースのため）が、`0700` を明示して POSIX contract を
  維持する。credential file の `0600` は Pi 側が管理し、
  cheaper は緩めない。
- `mkdir` 失敗（permission 等）は `createModelRuntime` の
  throw として表面化させる — 黙って Pi 既定 dir へ
  fallback しない（保存先の曖昧さは credential 分散に
  直結するため fail-closed）。
- `env` 未指定時は `process.env` を使う（現行の
  `resolveConfigDirs` と同じ既定）。
- test は `env` に tmpdir の `CHEAPER_CONFIG_DIR` を含めて
  注入し、実 `~/.config/cheaper`・`~/.pi` に触れない。

## Risks and Trade-offs

- `createModelRuntime` の options 型が Pi の
  `CreateModelRuntimeOptions` に cheaper 固有 key（`env`）を
  混ぜる。`env` は Pi 側に存在しない key で衝突しないが、
  型を `&` 交差にせず明示的な wrapper interface にすると
  読みやすい（実装時に判断）。
- `auth.json` が `~/.pi/agent` にある環境では未 login に
  見える（migration しない）。実運用前のため許容 — 初回
  `auth login` で新しい場所に書かれる。
- `mkdir -p` するのは `pi/` のみ。`userConfigDir` 自体も
  recursive mkdir で一緒に作られる。

## Verification Notes

- [ ] unit test: `env.CHEAPER_CONFIG_DIR` → `<dir>/pi/` 下の
      3 path が `ModelRuntime.create` へ渡ること（mock/spy）、
      dir が `0700` で作られること
- [ ] unit test: caller が `authPath` 等を明示した場合は
      既定より優先されること
- [ ] e2e: `PI_CODING_AGENT_DIR` 未設定・`CHEAPER_CONFIG_DIR`
      のみの隔離 env で `<configDir>/pi/` に state が出ること
- [ ] e2e: workspace `.cheaper/` に credential が出ないこと
- [ ] real-provider e2e が `<configDir>/pi/models.json` を
      使って動くこと
