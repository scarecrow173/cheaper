---
id: "DESIGN-OVERVIEW"
type: "design"
status: "draft"
title: "System Design Overview"
created: "2026-09-15"
updated: "2026-10-10"
owners: ["scarecrow173"]
relations:
  source:
    - "../handoff/2026-09-15-cheaper-v1-implementation-handoff.md"
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "worker-definition-resolver-runtime-policy-tool-vocabulary-error-model.md"
    - "workspace-filesystem-boundary-sandboxed-tools.md"
    - "model-resolver.md"
    - "pi-execution-engine.md"
    - "cli-contract-envelope-exit-codes.md"
    - "bulk-reader-contract.md"
    - "cheap-writer-contract.md"
    - "repository-structure-toolchain.md"
    - "sea-executable-build-pipeline-and-embedded-resource-resolution.md"
    - "apm-executable-payload-layout-and-platform-launchers.md"
    - "cheaper-hook-subcommands-replacing-node-based-hook-adapters.md"
    - "pi-sibling-tool-call-parallelism-inside-workers.md"
    - "rust-rig-runtime-migration-architecture.md"
    - "cargo-source-install-distribution-and-thin-path-launchers.md"
    - "post-migration-structural-cleanup-reorganization.md"
    - "observability-foundation-logging-and-instrumentation-wiring.md"
    - "devin-native-hooks-and-apm-bootstrap.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/cheaper-v1-worker-runtime-cli.md"
    - "../adr/typescript-pnpm-monorepo.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# System Design Overview

## System Boundaries

- `packages/core`（Pi 非依存）: Worker Definition/Resolver、Runtime Policy、tool vocabulary、path security、model tier 分類、error model
- `packages/runtime-pi`: Pi SDK 統合（session・resource loader・制限・abort・retry・出力抽出・repair・usage）、sandboxed tools、mutation journal
- `packages/cli`: コマンド・envelope・exit code・built-in workers/schemas/policy.yaml 同梱
- 依存方向は `cli → runtime-pi → core` の一方向のみ

## Core Components

| 段階（引継ぎ §4） | 担当 design |
| --- | --- |
| CLI 引数解析・stdin 入力・envelope 出力・exit code | DESIGN-034r05iHrZUE6WvDuYjZB4 |
| Worker 解決・frontmatter 検証・input schema 検証 | DESIGN-034r05iHrZUE6WrdtQIOmE |
| Runtime Policy ロード/マージ・CLI restriction・capability 交差 | DESIGN-034r05iHrZUE6WrdtQIOmE |
| Model tier 分類・available model 選択 | DESIGN-034r05iHrZUE6Wu1cMdeJ1 |
| sandboxed tool 実装・workspace boundary・mutation journal | DESIGN-034r05iHrZUE6WsgJOy6Ny |
| Pi session 実行・制限強制・abort 分類・retry・repair・usage | DESIGN-034r05iHrZUE6WvaQsRRsM |
| built-in Worker 定義（bulk-reader / cheap-writer） | DESIGN-034r05iHrZUE6WxkSJi9GF / DESIGN-034r05iHrZUE6Wy5X0Hc88 |
| リポジトリ構成・ツールチェーン | DESIGN-034r05iHrZUE6Wzyjd1NS8 |

## Data Flow

- 親 Coding Agent → `cheaper run <worker> --input -`（stdin JSON）→ envelope 1 行（stdout）+ 診断（stderr）
- core で Worker/Policy/model 解決 → runtime-pi が Pi session を生成し、sandboxed tools と budget を wrap して実行 → journal と events から deterministic meta を実測 → cli が envelope に詰めて返す

## Runtime Invariants の実装位置

| Invariant | 保証するコード |
| --- | --- |
| stdout は result envelope のみ | `cli/envelope.ts`（runtime-pi/core は stdout に書かない） |
| workspace 外アクセス不許可 | `runtime-pi/tools/operations.ts` + 自前 grep/find の `resolveWorkspacePath` |
| requested tools は宣言済み vocabulary のみ | `core/tools/vocabulary.ts` の alias 正規化と `UNKNOWN_TOOL` |
| policy 超過は拒否 | `core/policy` の capability 交差（tools.allow / maxTier） |
| maxTurns/maxToolCalls/timeout | `runtime-pi` の tool execute wrap・`turn_end` 監視・timer abort |
| abort 時 rollback なし・変更は実測報告 | `runtime-pi/journal.ts` → `meta.changes` |
| LLM 出力は schema 適合 JSON のみ | `runtime-pi/output.ts` + repair session |
| 未認証モデルを選択しない | `ModelRuntime.getAvailable()` のみを候補に使う |

## Non-Functional Constraints

- Node 26 / pnpm 11 / ESM only / `minimum_release_age = 7d`
- `@cheaper/core` は `@earendil-works/*` に依存しない
- Pi SDK の sandbox 不在・maxTurns 不在を cheaper 側コードで補完する（ADR-034r05iHfSMW9SGV4dpcy2/0007）

## Detailed Design Documents

読み順: 0008 → 0001 → 0002 → 0003 → 0004 → 0005 → 0006 → 0007

- [0008 リポジトリ構成とツールチェーン](./repository-structure-toolchain.md)
- [0001 コアドメインモデル](./worker-definition-resolver-runtime-policy-tool-vocabulary-error-model.md)
- [0002 workspace filesystem boundary と sandboxed tools](./workspace-filesystem-boundary-sandboxed-tools.md)
- [0003 Model Resolver](./model-resolver.md)
- [0004 Pi 実行エンジン](./pi-execution-engine.md)
- [0005 CLI contract](./cli-contract-envelope-exit-codes.md)
- [0006 bulk-reader contract](./bulk-reader-contract.md)
- [0007 cheap-writer contract](./cheap-writer-contract.md)

## Standalone Executable Distribution（issue #71）

- [SEA executable build pipeline と embedded resource 解決](./sea-executable-build-pipeline-and-embedded-resource-resolution.md)
- [APM executable payload layout と platform launcher](./apm-executable-payload-layout-and-platform-launchers.md)（superseded — issue #85）
- [cheaper hook subcommand 化による hook の脱 Node](./cheaper-hook-subcommands-replacing-node-based-hook-adapters.md)（hook subcommand 契約は存続、配布配线条項は issue #85 で superseded）

## Cargo source install 配布（issue #85 / ADR-034Dmb3kNAO4Ub6FxuQCtC）

- [Cargo source install distribution and thin PATH launchers](./cargo-source-install-distribution-and-thin-path-launchers.md) — CLI を `cargo install --git … --locked` で consumer へ配布し、APM package は thin launcher（`cheaper.sh` / `cheaper.ps1`）+ skills + hooks.json のみに縮小。旧 `cli/` payload・sibling binary・Release 取得機構は全撤去。

## Worker 内並列性（issue #55）

- [Pi sibling tool-call parallelism inside Workers](./pi-sibling-tool-call-parallelism-inside-workers.md)

## Rust + Rig ランタイム移行（SPEC-034BbgvL9s41K7NflOddZ6 / ADR-034BbhP6hJV7DFtv7k7AGs）

- [Rust + Rig runtime migration architecture](./rust-rig-runtime-migration-architecture.md) — Pi SDK + TypeScript/Node.js SEA から Rust + Rig ネイティブへの全面 rewrite。crate 分割・Rig adapter 隔離・自前ギャップ（catalog/auth/retry/devin）・differential golden 検証・配布切替を設計。既存の Pi 前提 design（上記）は移行元の仕様源として残る。

## Plugin catalog 永続化と user models merge（issue #81・#82）

- [Plugin catalog snapshot persistence and user models merge](./plugin-catalog-snapshot-persistence-and-user-models-merge.md) — plugin live catalog を `<userConfigDir>/providers/<id>/catalog.json` へ永続化し、plugin provider の user `models[]` を additive merge + catalog error stderr surfacing する修正設計。

## Post-migration 構造整理（SPEC-034DyDAOW1my2ihNQIVNfN）

- [Post-migration structural cleanup reorganization](./post-migration-structural-cleanup-reorganization.md) — Rust 移行後の behavior-preserving な構造整理。`cheaper-tools`/`cheaper-runtime` の module 再配置、`js16` core 集約、`js_json` shim 削除、`poc-rig` 撤去、`cheaper-provider-devin` → `cheaper-provider`(`devin` module)rename、`dev-seam` cargo feature による dev/test seam gate の target layout を定義。

## Observability foundation（SPEC-034DUFyMl47o924O1KW8Ya）

- [Observability foundation logging and instrumentation wiring](./observability-foundation-logging-and-instrumentation-wiring.md) — `cheaper-observability` crate（activation 解釈・file+stderr dual sink・format 選択）の公開面、`--log`/`--log-format` activation 配線、hook 経路の stderr sink 抑制、daily rolling + 世代 retention、全 crate の `cheaper.*` span/field 規約と instrumentation 配置、log 無効時 overhead 計測方法を定義。

## Devin native hook bootstrap（SPEC-034GssA0IjLKSiDdIHpAC8 / ADR-034GssAufIiId3hgh4jrSQ）

- [Devin native hooks and APM bootstrap](./devin-native-hooks-and-apm-bootstrap.md) — `HookTarget::Devin` + 共有 `tool_use_normalize`（tool_name ASCII lowercase→per-tool routing）・`devin_render`・`cheaper hook targets --json` capability query・配布 `hooks.json` matcher 拡大・`install-devin-hooks.ps1`（preview/`-Apply`・capability handshake・non-destructive atomic write）の機械設計。#107 の範囲評価本体は引き続き deferred。
