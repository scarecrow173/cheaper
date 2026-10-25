---
id: "ADR-034BbhP6hJV7DFtv7k7AGs"
type: "adr"
status: "accepted"
title: "Migrate the cheaper CLI runtime to Rust and Rig"
created: "2026-10-06"
updated: "2026-10-08"
owners: []
relations:
  source:
    - "../research/pi-runtime-replacement/migration-comparison.md"
    - "../research/pi-runtime-replacement/comparison.md"
    - "../research/rig-agent-runtime/final-report.md"
    - "../research/rig-agent-runtime/migration-plan.md"
    - "../research/molo-agent-runtime/final-report.md"
    - "../research/molo-agent-runtime/migration-plan.md"
    - "../research/goose-sdk/final-report.md"
    - "../research/goose-sdk/migration-plan.md"
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
  supersedes:
    - "agent-runtime-pi-sdk-earendil-works-pi-coding-agent.md"
    - "typescript-pnpm-monorepo.md"
    - "pi-persistent-state-userconfigdir-pi-modelruntime-create-path.md"
    - "canonical-tokenizer-js-tiktoken.md"
  superseded-by: []
  related:
    - "distribute-the-cheaper-cli-as-a-committed-esbuild-bundle-inside-the-apm-hook-bun.md"
    - "remove-the-committed-esbuild-cli-payload-and-move-the-canonical-version-to-cargo.md"
    - "devin-local-provider-via-cli-delegated-auth-and-isolated-data-dir.md"
    - "usage-jsonl-in-user-config-dir-best-effort-append.md"
    - "version-pinned-launcher-acquisition.md"
    - "keep-node-js-as-the-pinned-dev-tooling-runtime.md"
    - "../discovery/node-dev-tooling-inventory-issue-182.md"
    - "../specs/node-dev-tooling-scope-issue-182.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/rust-rig-native-runtime-migration-for-cheaper-cli.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/0xPlaygrounds/rig"
    - "https://github.com/Silwings-git/molo"
    - "https://github.com/block/goose"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# Migrate the cheaper CLI runtime to Rust and Rig

## Context and Problem Statement

cheaper CLI は現在、`packages/runtime-pi` が `@earendil-works/pi-ai` / `pi-coding-agent` 0.85.1(Pi SDK)に依存し、esbuild bundle + Node 26 SEA として配布されている(現行 `cheaper-windows-x64.exe` ≈113MB)。配布サイズと Node.js ランタイム依存を除去するため、Pi SDK と TypeScript/Node.js を撤去して Rust ネイティブバイナリへ移行する。移行は src ≈16.4k 行 + test ≈23.5k 行の全面 rewrite を伴い、agent runtime / provider 抽象の基盤として Rust 製 SDK を選定する必要がある。上流 discovery(DISC-034B9B0PiBkxUgLGfI70g8)と 3 案の移行計画比較が本決定の入力。

## Decision Drivers

* 配布実行ファイルのサイズ削減(≈113MB → 一桁〜十数 MB 級)と Node.js 依存の除去が移行の主目的であり、目的不達の選択肢は成立しない
* 外部 contract(envelope `schemaVersion:1`・usage.jsonl・hook・config・Release asset 名・SHA256SUMS・launcher sh/ps1)は不変を維持する
* Anthropic native provider は cacheRead/cacheWrite 計測とコスト最適化の目的上必須であり、OpenAI 互換 endpoint 迂回は不可
* rewrite 総量が支配的コスト(src ≈16.4k 行)であり、SDK 差異は 15-25% 程度にしか効かない — provider 自作量と成熟度リスクで選ぶ
* 不可逆な全面移行の前に小さな PoC で go/no-go を判定し、撤退線を確保する

## Considered Options

* (1) Rig(0xPlaygrounds/rig): `rig-agent`/`rig-core` + `cheaper-rig-adapter` 隔離 crate で移行
* (2) molo(Silwings-git/molo): molo agent loop + Provider/Tool trait で移行
* (3) Goose GDK(block/goose): `goose-agent`+`goose-providers`+`goose-provider-types` のみ採用して移行
* (4) TypeScript + Pi SDK + Node SEA を維持する

## Decision Outcome

Chosen option: "(1) Rig", because provider 内蔵数が最大(Anthropic 含む 26 種)で自作量が最小、3 候補中の成熟度・採用実績が最良(v0.43 / 8.8k stars / crates.io 累計 ≈330万 DL)であり、最大リスクの 0.x breaking change は version pin + adapter 1 crate 隔離で緩和可能。molo 案・Goose 案の両計画が「不達なら Rig へピボット」と相互に指名しており、Rig が実質的な基準線であるため。

確定した決定事項:

* **採用形態**: Rust workspace の全面 rewrite。`rig-agent`(classic runtime)を第一選択とし、agent loop の hook 表現力が不足と判明した場合のみ `rig-core` の Model/streaming に退避する。`rig = "=0.43.x"` で pin し、upgrade は MIGRATING.md 手順の計画的運用とする。Rig 依存は `cheaper-rig-adapter` 隔離 crate に閉じ込め、core/tool-runtime/workflow/cli は Rig 型を直接参照しない
* **段階移行**: Phase 0 PoC(go/no-go ゲート)→ Phase 1 core → Phase 2 tool-runtime → Phase 3 runtime → Phase 4 CLI/workflow → Phase 5 配布切替 → Phase 6 TS 撤去。Phase 1(core/tool-runtime 移植)までは候補中立の投資であり、Rig PoC が不達でも salvage 可能
* **contract 不変**: envelope `schemaVersion:1`・usage.jsonl・`hook <event> --target`・Release asset 名・SHA256SUMS・launcher(sh/ps1)は無改変を維持。変わるのはビルド層(esbuild+SEA → cargo matrix build)と version canonical source のみ
* **provider/auth 範囲**: 主要 provider + GitHub Copilot + devin を scope とし、OAuth flow も移植してサブスクリプション認証に対応する。model catalog・credential store は Pi ModelRuntime の機能に代替物がないため自作する
* **usage meta**: `costUsd` は cheaper の pricing 層で算出する口径に変更する。`<userConfigDir>/pi/` の Pi state(`auth.json`/`models.json`/`models-store.json`)は移行せず再 login を要求する
* **fallback**: PoC 不達時は Goose GDK を第二候補 PoC に切替。さらに不達なら移行を中止し、TS 維持 + 配布改善のみに縮小する

### Consequences

* Good, because バイナリサイズは ≈113MB → 5-15MB 見込み(PoC go 条件 ≤20MB)に縮小し、Node.js ランタイム依存と SEA ビルドが除去できる。
* Good, because Rust 化で darwin-* target の追加が容易になる(現行は launcher が darwin を検知するが acquisition は未提供)。
* Good, because provider 標準系は Rig 内蔵を利用でき、自作は devin・auth・catalog・retry 層に限定される。
* Bad, because src ≈16.4k 行 + test ≈23.5k 行の全面 rewrite が必要で、並走期間中の TS/Rust 二重保守が発生する。
* Bad, because model catalog・credential store・OAuth flow・devin provider(Connect wire ≈656 行)・transient retry 層を自作する必要がある。
* Bad, because Rig 0.x の breaking change 追従コストが継続する(`=0.43.x` pin + adapter 隔離で緩和するが upgrade は計画作業)。
* Bad, because usage.jsonl の `costUsd` 口径が変わり(Pi 供給 → pricing 層算出)、新旧 record で値の母数が混在する。`cheaper usage` 集計と `savings.directlyComparable` の扱いに影響する。
* Bad, because Pi state を移行しないため、ユーザーは全 provider で再 login が必要になる。
* Neutral, because Phase 0 スパイクが go/no-go ゲートとして機能し、不達時の撤退線(Goose GDK → TS 維持縮小)が事前に確保されている。
* Neutral, because `rig` の pin + `cheaper-rig-adapter` 隔離は運用ルールとして維持コストを伴うが、Rig 以外の層を breaking change から保護する。

### Confirmation

* Phase 0 の go 条件: (a) release build(`lto`+`opt-level="s"`+strip)のバイナリサイズ実測 ≤20MB(目標 5-15MB)、(b) agent loop が turn/tool call/usage/cancel を `WorkerRunMeta` に写せる粒度で露出、(c) tool 無し + json_schema + retry の repair 動線が構成できる、(d) custom `Wire`/`Decoder` で devin 相当の独自 provider が書ける設計見通し、(e) tokio+rustls で 3 OS ビルドが成立。
* parity は fake/scripted provider fixture による TS/Rust binary 間 differential golden test で確認する(normalize 規則: `meta.durationMs`・`usage.costUsd`・tokenizer identity・JSON key 順・schema error message 文言は比較対象外)。
* 配布切替は「同一 asset 名・SHA256SUMS・`--version` probe で Release が組め、launcher が無改修で Rust binary を取得・検証・exec できる」ことで確認する。

## Implementation Plan

* Affected paths: 新設 Rust workspace(`rust/` or `packages-rs/` 相当。`cheaper-core`/`cheaper-tool-runtime`/`cheaper-runtime`/`cheaper-rig-adapter`/`cheaper-cli` crate)。既存 `packages/core`、`packages/tool-runtime`、`packages/runtime-pi`(+`providers/devin`)、`packages/workflow`、`packages/cli` は Phase 6 で撤去。`packages/apm/.apm/hooks/`(launcher・hooks.json・skills)は contract 不変のため無改変。`scripts/lib/release-artifacts.mjs` の SHA256SUMS 生成・検証フローは asset 名不変のまま流用。`.github/workflows/build-executables.yml` を cargo matrix build に置換。`mise.toml` に Rust toolchain を追加する
* Patterns to follow: `rig-agent` `AgentBuilder` + `default_max_turns` + hooks(`ModelTurnFinished`/`ToolCallDelta`)で `WorkerRunMeta` を構築。sandboxed tool 8 種は `Tool`/`PortableTool` trait + `ToolContext` で invocation scope を注入する。repair session は「tool 無し completion + json_schema mode + schema error 反馈 retry」を cheaper 側に再実装する(`Extractor` ではなく現行意味に近い自前 loop)。abort/timeout/grace は tokio `CancellationToken` + `tokio::time::timeout` で包み reason 分類を保持する。schema 検証は `jsonschema` crate(draft 2020-12)、token 計測は `tiktoken-rs`(o200k_base)、builtin resource は `include_dir!` 埋込(SEA VFS 代替)
* Patterns to avoid: Rig 型を `cheaper-rig-adapter` 以外の crate から直接参照すること。main loop への `output_schema` 適用(tool call 抑制を避ける現行設計を維持)。`rig-ecs` 等の heavyweight runtime の採用(YAGNI)。Pi state の互換 import(再 login 方針で割り切る)
* Dependencies or configuration: `rig`/`rig-core`/`rig-agent` `=0.43.x` pin、`rig-reqwest`(transport opt-in)、`serde`/`serde_json`、`jsonschema`、`serde_yml` or `serde_norway`、`tiktoken-rs`、`include_dir`、`tokio`。`CHEAPER_TEST_FAKE_STREAM` 相当の dev-only fake provider seam を `cheaper-rig-adapter` に用意する
* Migration steps: Phase 0 PoC → go/no-go → Phase 1 core → Phase 2 tool-runtime → Phase 3 runtime(model catalog・credential store/OAuth・transient retry・devin provider・run オーケストレーション)→ Phase 4 CLI/workflow → Phase 5 配布切替 → Phase 6 TS 撤去・docs 更新。並走期間は TS 版を CI で完全に生かし、Phase 0 通過後は TS 側への新機能追加を凍結する

## Verification

* [ ] Phase 0 PoC の go 条件 5 項目(バイナリ ≤20MB 実測・`WorkerRunMeta` 粒度・repair 動線・custom `Wire`/`Decoder` 見通し・3 OS ビルド)を実測で確認
* [ ] `Cargo.toml` の `rig` 系依存が `=0.43.x` で pin され、Rig 型の参照が `cheaper-rig-adapter` に限定されていることを code review で確認
* [ ] TS/Rust binary 間 differential golden test(代表 worker × 正常系/repair/abort/PARTIAL_MUTATION/error の 20-40 case)が normalize 規則内で全パス
* [ ] envelope `schemaVersion:1`・hook 4 target byte 一致・usage.jsonl 新旧 record 互換・exit code 表が TS 版と一致することを e2e/contract test で確認
* [ ] api_key/env/OAuth(openai-codex・anthropic・github-copilot)+ devin CLI 委譲の auth flow が動作し、`auth status` が auth type 別に表示できることを確認
* [ ] 同一 asset 名・SHA256SUMS・`--version` probe で Release が組め、launcher が無改修で Rust binary を取得・検証・exec できることを release-acquisition smoke で確認

## Pros and Cons

### Option 1: Rig (0xPlaygrounds/rig) — 採用

* Good, because provider 内蔵が最大(Anthropic/OpenAI/Ollama/openai-compat 等 26 種 + Bedrock/Vertex は companion crate)で、provider 自作量が devin + catalog + auth に限定される。
* Good, because 成熟度・採用実績が 3 候補中最良(v0.43、8.8k stars、crates.io 累計 ≈330万 DL、VT Code 等の採用例)。turn/tool call/usage(cache 含む)/streaming/cancel/structured output + repair が一次ソースで確認済み。
* Good, because `rig-core` が tokio/reqwest を通常依存に持たず transport opt-in のため小さいバイナリの見通しが高い(推測 5-15MB)。
* Good, because molo 案・Goose 案の両計画が fallback 先として Rig を指名しており、選定の基準線となる。
* Bad, because 0.x で breaking change が多発(0.40 で 31 件、2-4 週毎 release、後方互換 issue #628 は not planned close)。pin + adapter 隔離 + 計画 upgrade で緩和するが追従コストは残る。
* Bad, because docs の一部(custom provider ガイド等)が旧 API で、docs.rs API reference / source を正とする必要がある。

### Option 2: molo (Silwings-git/molo) — 不採用

* Good, because kernel/harness/effect 分離が cheaper の sandboxed tool モデルに構造適合し、最小バイナリが見込める(PoC 閾値 ≤15MB)。
* Bad, because Anthropic native provider が内蔵されず自作が必要(provider 自作計 20-35 人日)。cacheRead/cacheWrite 計測上 OpenAI 互換迂回は不可で、構造的に不利。
* Bad, because v0.6・作成 2 ヶ月・単一作者・採用実績ゼロ・pre-1.0 で bus factor リスクが大きい。vendoring は可能だが保守が自前に転嫁される。構造適合の恩恵も audit/observation 接続に限定的と molo 案自身が評価しており、provider 自作量と未成熟リスクが割に合わない。
* Neutral, because watchlist として継続監視する価値はある。

### Option 3: Goose GDK (block/goose) — 次点(fallback)

* Good, because Anthropic native が内蔵(provider native 10 種 + declarative JSON)で、`goose-agent` の埋め込み採用が可能。Block 本体は活発に開発されている。
* Good, because structured output は tools 無し 2 本目 StateMachine で現行設計を直移植できる見通し。
* Bad, because 採用 crate が全て 0.1.0-alpha で、フル crate が未公開。structured output の repair は自作が必要で、`response_format` 注入可否は未確認。
* Neutral, because Rig PoC 不達時の同格代替として第二候補 PoC の対象とする。

### Option 4: TypeScript + Pi SDK + Node SEA 維持 — 不採用

* Good, because rewrite コストがゼロで、既存の検証済み資産をそのまま使える。
* Bad, because ≈113MB の SEA バイナリと Node.js ランタイム依存が残り、移行の主目的(配布サイズ削減・Node 依存除去)を達成できない。
* Neutral, because Rig・Goose の両 PoC が不達となった場合の最終 fallback として「TS 維持 + 配布改善のみに縮小」を選択肢として保持する。

## More Information

* 上流 discovery: DISC-034B9B0PiBkxUgLGfI70g8(`docs/discovery/rust-rig-native-runtime-migration-for-cheaper-cli.md`)
* 移行計画 3 案比較: `docs/research/pi-runtime-replacement/migration-comparison.md`
* Rig 詳細調査・移行計画: `docs/research/rig-agent-runtime/final-report.md`・`docs/research/rig-agent-runtime/migration-plan.md`
* 本 ADR が置き換える決定: `agent-runtime-pi-sdk-earendil-works-pi-coding-agent.md`(Pi SDK 採用)。accept 後に同 ADR の `superseded-by` を本 ADR へ更新し、`typescript-pnpm-monorepo.md`・`distribute-the-cheaper-cli-as-a-committed-esbuild-bundle-inside-the-apm-hook-bun.md`・`pi-persistent-state-userconfigdir-pi-modelruntime-create-path.md`・`canonical-tokenizer-js-tiktoken.md` の status/relations も見直す
