---
id: "ADR-0008"
type: "adr"
status: "accepted"
title: "TypeScript + pnpm monorepo 構成"
created: "2026-09-15"
updated: "2026-09-17"
owners: ["scarecrow173"]
relations:
  source:
    - "../handoff/2026-09-15-cheaper-v1-implementation-handoff.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/0001-cheaper-v1-worker-runtime-cli.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "0001-agent-runtime-pi-sdk-earendil-works-pi-coding-agent.md"
    - "0004-one-shot-json-first-cli.md"
    - "0007-workspace-filesystem-boundary-cheaper-sandboxed-tool.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/0001-cheaper-v1-pi-sdk.md"
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

# 8. TypeScript + pnpm monorepo 構成

## Context and Problem Statement

cheaper の実装言語・パッケージ分割を決める。責務境界（§45）と「Worker 追加で Runtime ソースを変更しない」（§44）を維持しつつ、Pi SDK（TypeScript/ESM）を利用する。

## Decision Drivers

* Pi SDK が TypeScript ESM パッケージであること
* Pi 非依存のコアロジック（parser/policy/security/resolver）と Pi 依存部を分離したい
* §45 の責務境界をパッケージ構造に反映したい

## Considered Options

* (1) 単一パッケージに全てを置く
* (2) 3 パッケージ monorepo: `core` / `runtime-pi` / `cli`
* (3) 細分化 monorepo（worker-registry / policy / tools / runtime / cli 等を分割）

## Decision Outcome

Chosen option: "(2) 3 パッケージ monorepo", because Pi 非依存 core・Pi 依存 runtime・CLI 境界を明確にできつつ、YAGNI に反する過剰分割を避けられるから。

### Consequences

* Good, because `core` が Pi 非依存でテスト容易・再利用可能。
* Good, because Pi SDK の変更影響が `runtime-pi` に限定される。
* Bad, because monorepo のツールチェーン管理コストがある。

### Confirmation

* 依存方向が `cli → runtime-pi → core` 一方向であり逆依存がないことを lint/レビューで確認。
* `packages/core` が `@earendil-works/*` を依存に持たないことを確認。

## Implementation Plan

* Affected paths: `packages/core`, `packages/runtime-pi`, `packages/cli`（いずれも予定 path、現在未作成）
  * `packages/core` — 型・Worker parser/validator/resolver・Runtime Policy・tool vocabulary・path security・model tier 分類・error model（**Pi 非依存**）
  * `packages/runtime-pi` — Pi SDK 統合・sandboxed tools・実行・計測（Pi 依存はここに閉じる）
  * `packages/cli` — コマンド・envelope・exit code・built-in workers/schemas/policy.yaml 同梱
* Patterns to follow: 依存方向 `cli → runtime-pi → core` の一方向のみ
* Patterns to avoid: core から Pi への import、Worker 固有ロジックの runtime ハードコード（§0）
* Dependencies or configuration: TypeScript / Node 26 / pnpm 11（mise.toml 管理）/ ESM。`minimum_release_age = 7d` を遵守。**依存追加は別途承認制**

## Verification

* [ ] `packages/core` が Pi 系パッケージを import していないことを確認
* [ ] 依存方向が `cli → runtime-pi → core` のみであることを確認
* [ ] `pnpm install` / build / test が Node 26 + pnpm 11（mise）で通ることを確認

## Pros and Cons

### Option 1: 単一パッケージ

* Good, because 最も単純。
* Bad, because Pi 非依存ロジックと Pi 統合が混ざり、core の独立テスト・再利用が難しい。

### Option 2: 3 パッケージ

* Good, because 責務境界と一致しつつ過剰でない。
* Bad, because workspace 管理が必要。

### Option 3: 細分化 monorepo

* Good, because 最も細かい分離。
* Bad, because YAGNI。V1 の規模ではパッケージ間オーバーヘッドが支配的になる。

## More Information

* 引継ぎ §44, §45
* mise.toml（Node/pnpm バージョン管理）、pnpm `minimum_release_age = 7d`

## 更新履歴

### 2026-09-15: 依存パッケージ承認と Biome 採用

* ユーザー承認済み依存: runtime `@earendil-works/pi-coding-agent@0.85.1`, `@earendil-works/pi-ai@0.85.1`, `typebox`, `ajv`, `yaml`, `diff`。dev `typescript`, `vitest`, `@types/node`, `@biomejs/biome`。`minimum_release_age = 7d` を満たすバージョンを選ぶ。
* linter/formatter は Biome（`biome.json`）を採用。
* プローブで pnpm の ignored build scripts（esbuild/@google/genai/protobufjs）により `pnpm add` が exit 1 になることを確認。install 手順で `pnpm approve-builds`/`onlyBuiltDependencies` の扱いが必要（DESIGN-0008）。
