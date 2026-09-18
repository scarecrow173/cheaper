---
id: "ADR-034r05iHrZUE6Wn59hpaOg"
type: "adr"
status: "accepted"
title: "Pi persistent state は userConfigDir/pi 配下へ ModelRuntime.create の明示 path で集約する"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/pi-state-under-cheaper-config-dir.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "./absorb-provider-oauth-and-api-key-login-into-the-cheaper-cli-cheaper-auth.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./usage-jsonl-in-user-config-dir-best-effort-append.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/pi-state-under-cheaper-config-dir.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# 27. Pi persistent state は userConfigDir/pi 配下へ ModelRuntime.create の明示 path で集約する

## Context and Problem Statement

`createModelRuntime()` は現在 `ModelRuntime.create()` を
options 無しで呼ぶため、Pi の credential（`auth.json`）・
provider 定義（`models.json`）・catalog cache
（`models-store.json`）は Pi の既定 dir（`~/.pi/agent`、
`PI_CODING_AGENT_DIR` で変更可）へ書かれる。cheaper 自身の
永続データは `resolveConfigDirs` の `userConfigDir` に集約
済みであり、Pi 側だけが別体系のグローバル領域へ分散して
いる。SPEC-034r05iHK8bA7gxlRVZtAv はこの 3 file を `<userConfigDir>/pi/` へ
集約することを要求する。決めるべきこと: (a) 保存先の
指定方法（env var vs 明示 path）、(b) 配置先（Pi dir 直下
vs `pi/` subdir vs project-local）、(c) path 組み立ての
canonical 位置。

## Decision Drivers

- Issue #25: 「`PI_CODING_AGENT_DIR` を cheaper の保存先制御に
  利用しない」「runtime に path を明示する」「project-local
  `.cheaper/` に credential を保存しない」「canonical helper
  を runtime 初期化でも再利用する」を明示する。
- credential は secret — consumer の workspace・project-local
  `.cheaper/` へ置くと repo 汚染・誤 commit の危険がある。
- `userConfigDir` 解決は `CHEAPER_CONFIG_DIR` で隔離可能で、
  e2e が既に tmpdir へ向けている — `PI_CODING_AGENT_DIR`
  隔離を置き換えられる。
- Pi 0.85.1 の `CreateModelRuntimeOptions` は `authPath` /
  `modelsPath` / `modelsStorePath` を受け付けるため、Pi 側の
  変更なしに保存先を制御できる。

## Considered Options

### 保存先の指定方法

- **A: `PI_CODING_AGENT_DIR` env を cheaper が設定する**。
  runtime 生成コードの変更が最小だが、cheaper process 全体の
  Pi 設定を env で書き換える — 他の Pi API 経路（extension
  discovery・`PI_CODING_AGENT_DIR` を読む全機能）まで cheaper
  の所有物に引きずり込み、影響範囲が不透明になる。Issue #25
  が明示的に禁止。
- **B: `ModelRuntime.create()` へ `authPath`/`modelsPath`/
  `modelsStorePath` を明示する**（採用）。効果が cheaper の
  生成する runtime だけに閉じ、他の Pi 機能へ波及しない。
  Pi の public API で完結する。

### 配置先

- **i: `<userConfigDir>` 直下に `auth.json` 等を置く**。
  cheaper の `models.yaml`・`policy.yaml`・`usage.jsonl` と
  混在し、Pi 由来と cheaper 由来の区別がつかない。
- **ii: `<userConfigDir>/pi/` subdir**（採用）。Issue #25 の
  想定構成と一致し、Pi runtime の所有物を 1 dir にまとめて
  隔離・削除・permission 管理しやすい。
- **iii: `<cwd>/.cheaper/pi/`**。project-local は config
  override 用で secret を置かない — Issue #25 が明示禁止。

### path 組み立ての canonical 位置

- **α: CLI の各呼び出し点で組み立てる**。`cli.ts`（run）と
  `auth.ts`（auth 系）の 2 箇所で同じ path を組み立てると
  表現ドリフトの元になる。
- **β: `createModelRuntime()` 側で `userConfigDir` を解決
  して組み立てる**（採用）。cli・auth 双方の唯一の
  runtime 生成経路に集約され、「同一 path policy」が構造で
  保証される。`resolveConfigDirs`（`env`・`cwd` 注入可能）を
  再利用し、test では `CHEAPER_CONFIG_DIR` で隔離する。

## Decision Outcome

- `createModelRuntime()` は `resolveConfigDirs` で
  `userConfigDir` を解決し、`ModelRuntime.create()` に
  `authPath: <userConfigDir>/pi/auth.json`・
  `modelsPath: <userConfigDir>/pi/models.json`・
  `modelsStorePath: <userConfigDir>/pi/models-store.json`
  を渡す。
- `pi/` dir は runtime 生成時に `mkdir -p`（POSIX `0700`）
  で作成する。credential file の `0600` は Pi 側の contract
  に委ね、cheaper は緩めない。
- `PI_CODING_AGENT_DIR` は読まない・設定しない。
- `~/.pi/agent` 等からの migration・fallback・暗黙 import は
  実装しない（実運用前のため）。
- project-local `.cheaper/` は credential を含め Pi state を
  持たない。

## Consequences

- `createModelRuntime()` が `env`/`cwd` に依存するため、
  options に path 解決用の注入 seam（`env`・`cwd` または
  `userConfigDir` 自体）が必要になる。CLI は process の
  `io.env`/`cwd` を渡し、test は tmpdir を注入する。
- 既存 e2e の `PI_CODING_AGENT_DIR` 隔離は不要になり、
  `CHEAPER_CONFIG_DIR` 隔離へ置き換わる — Pi state が
  `<CHEAPER_CONFIG_DIR>/pi/` へ出ることを assert する形に
  更新する。
- `docs/guides/providers.md`・`docs/guides/configuration.md`
  の保存場所記述を `<userConfigDir>/pi/` へ更新する。
- `auth.json` を `~/.pi/agent` に持つ既存環境では login が
  無い状態として見える（migration しない仕様）。実運用前の
  ため許容。

## Confirmation

- unit test: 一時 `CHEAPER_CONFIG_DIR` を指定して
  `createModelRuntime()` を呼び、`authPath`/`modelsPath`/
  `modelsStorePath` が `<dir>/pi/` 配下へ解決されること
  （options の組み立てを spy または file 作成で assert）。
- e2e: 隔離 env（`CHEAPER_CONFIG_DIR` のみ、`PI_CODING_AGENT_DIR`
  未設定）で `auth status` / worker 実行後に
  `<CHEAPER_CONFIG_DIR>/pi/auth.json` 系へ state が出ること、
  workspace `.cheaper/` へ credential が出ないことを assert。
- `pnpm lint`・`pnpm typecheck`・`pnpm test` が green。
- bundled CLI 再生成で bundle freshness check が pass。
