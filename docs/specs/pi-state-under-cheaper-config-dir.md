---
id: "SPEC-034r05iHK8bA7gxlRVZtAv"
type: "spec"
status: "implemented"
title: "Pi auth and model state under the Cheaper config directory"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/25"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./auth-login-command.md"
    - "./delegation-usage-measurement-log.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  spec:
    priority: "should"
---

# Pi auth and model state under the Cheaper config directory

## What Should Be Built

`cheaper` が生成する Pi `ModelRuntime` の永続ファイル
（credential `auth.json`・provider 定義 `models.json`・
catalog cache `models-store.json`）を、Pi のグローバル既定
dir（`~/.pi/agent`）ではなく cheaper の **user-scope config
directory** 配下（`<userConfigDir>/pi/`）へ保存するよう
runtime 初期化で明示する。

```text
$CHEAPER_CONFIG_DIR/
├── models.yaml
├── policy.yaml
├── usage.jsonl
└── pi/
    ├── auth.json
    ├── models.json
    └── models-store.json
```

`cheaper auth login/status/logout` と Worker 実行は同一の
ModelRuntime path policy を共有し、credential store の参照先が
常に一致する。

## Why Now

現在 `createModelRuntime()` は `ModelRuntime.create()` を
options 無しで呼ぶため、Pi は auth・model state を自分の
既定 dir（通常 `~/.pi/agent`）へ書く。cheaper の永続データ
（`models.yaml`・`policy.yaml`・`usage.jsonl`）はすでに
`CHEAPER_CONFIG_DIR` / XDG 規則で user scope に集約されて
おり、Pi 側だけがグローバル領域へ分散している（Issue #25）。

実運用開始前の今のうちに保存先を統合しておかないと、
credential の所在が 2 箇所に分かれ、後からの移行が
migration 問題になる。

## Users and Value

- cheaper の利用者: credential・model state の所在が
  `CHEAPER_CONFIG_DIR` 1 箇所に統一され、バックアップ・
  隔離・削除が一意にできる。
- parent agent / 開発者: テストが `CHEAPER_CONFIG_DIR` の
  向け替えだけで Pi state まで完全に隔離でき、実ユーザーの
  credential を一切触らない構造が保証される。

## Scope

### In Scope

- `createModelRuntime()` が cheaper の user-scope config dir
  を解決し、`ModelRuntime.create()` に `authPath`・
  `modelsPath`・`modelsStorePath` を明示的に渡すこと。
- CLI の `run` 系・`auth` 系コマンドが同じ path policy を
  使うこと（`resolveConfigDirs` の `userConfigDir` 解決を
  再利用）。
- credential directory/file の permission contract の維持
  （directory は POSIX で原則 `0700`、credential file は
  Pi の `0600` contract を壊さない）。
- テストの隔離を `CHEAPER_CONFIG_DIR`（tmpdir）で完結させ、
  開発者の実 `~/.pi`・`~/.config/cheaper` を参照・変更しない
  こと。現行の `PI_CODING_AGENT_DIR` による隔離は
  `CHEAPER_CONFIG_DIR` 隔離へ置き換える。
- provider/auth 関連ドキュメントの保存場所記述の更新
  （`docs/guides/providers.md`・`docs/guides/configuration.md`）。

### Out of Scope

- `~/.pi/agent/auth.json` 等からの自動 migration・fallback
  compatibility（実運用前のため不要。既存 Pi credential の
  暗黙 import も行わない）。
- `PI_CODING_AGENT_DIR` env を cheaper の保存先制御に使う
  こと。cheaper process 全体の Pi 設定を env で書き換える
  のではなく、cheaper が生成する runtime に path を明示する。
- repository-local `<cwd>/.cheaper/` への credential 保存。
  project-local `.cheaper/` は現行どおり project override
  専用とし、secret/credential は必ず user scope に置く。
- Pi 側の変更（Pi 0.85.1 の `ModelRuntime.create()` は
  `authPath` / `modelsPath` / `modelsStorePath` を明示指定
  できるため不要）。

## Requirements

1. **保存先**: `authPath`・`modelsPath`・`modelsStorePath` は
   `resolveConfigDirs` の `userConfigDir` 配下の `pi/`
   subdirectory に固定する。`userConfigDir` の解決規則
   （`CHEAPER_CONFIG_DIR` → `XDG_CONFIG_HOME/cheaper` →
   `~/.config/cheaper`）は現行を維持する。
2. **canonical helper**: user-scope config dir の解決は
   `resolveConfigDirs`（または同等の canonical helper）を
   runtime 初期化でも再利用し、path 組み立ては 1 箇所に
   集約する（表現ドリフト防止）。
3. **同一 policy**: `cheaper run`・`cheaper auth status`・
   `auth login`・`auth logout` が同じ ModelRuntime path
   policy で生成されることを保証する。
4. **permission**: `pi/` directory は POSIX で原則 `0700`、
   credential file は Pi の `0600` contract を壊さない。
5. **隔離**: テストは一時 `CHEAPER_CONFIG_DIR` を使い、実
   auth/config を参照・変更しない。`PI_CODING_AGENT_DIR` に
   依存しない。
6. **docs**: provider/auth 関連ドキュメントの保存場所説明が
   実装と一致する。

## Acceptance Criteria

- [ ] `cheaper auth login <provider>` で作成される Pi
      credential が user-scope cheaper config 配下に保存
      される。
- [ ] `cheaper auth status` / Worker 実行が同じ credential
      store を参照する。
- [ ] Pi `models.json` / `models-store.json` も cheaper
      config 配下に保存される。
- [ ] project-local `.cheaper/` に credential が保存されない。
- [ ] `PI_CODING_AGENT_DIR` に依存しない。
- [ ] auth 関連テストが実ユーザーの Pi/Cheaper credential を
      触らない。
- [ ] docs の保存場所説明が実装と一致する。

## Sources and Relations

- Issue #25: 要件の起点（想定構成・実装指示・migration 不実装
  の方針・acceptance criteria）。
- SPEC-034r05iHK8bA7ghSWRbNU8 / ADR-034r05iHrZUE6W71rbZ6aR: provider auth の cheaper CLI 吸収 —
  credential store の単一参照先を引き継ぐ対象。
- ADR-034r05iHrZUE6WljM1ZMiX: `usage.jsonl` の userConfigDir 配置 — 同じ
  user-scope 集約の考え方。
- `packages/core/src/config/dirs.ts`: `resolveConfigDirs` の
  canonical 解決。
- `packages/runtime-pi/src/model-runtime.ts`: 現行の
  `ModelRuntime.create()` 呼び出し点。
