---
id: "DISC-034B9B0PiBkxUgLGfI70g8"
type: "discovery"
status: "resolved"
title: "Rust + Rig native-runtime migration for cheaper CLI"
created: "2026-10-06"
updated: "2026-10-06"
owners: []
relations:
  source:
    - "docs/research/pi-runtime-replacement/comparison.md"
    - "docs/research/pi-runtime-replacement/migration-comparison.md"
    - "docs/research/rig-agent-runtime/final-report.md"
    - "docs/research/rig-agent-runtime/migration-plan.md"
    - "docs/research/molo-agent-runtime/final-report.md"
    - "docs/research/molo-agent-runtime/migration-plan.md"
    - "docs/research/goose-sdk/final-report.md"
    - "docs/research/goose-sdk/migration-plan.md"
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
    - "docs/discovery/standalone-cheaper-executable-distribution-without-node-js.md"
    - "docs/discovery/apm-self-contained-cli-distribution.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "docs/specs/rust-rig-native-runtime-migration-for-cheaper-cli.md"
    - "docs/adr/migrate-the-cheaper-cli-runtime-to-rust-and-rig.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Rust + Rig native-runtime migration for cheaper CLI

## Exploration Goal

配布用実行ファイル(現行 Node SEA `cheaper-windows-x64.exe` ≈113MB)のサイズを
削減するため、Pi SDK(`@earendil-works/pi-ai` / `pi-coding-agent`)と
TypeScript/Node.js ランタイム依存を除去し、Rust ネイティブバイナリへ移行できるか。
移行先 SDK として Rig / molo / Goose GDK を調査・比較し、採用候補と段階的移行計画を
確定させることが目的。

トリガー: ユーザーからの Rust+Rig 移行計画要求。SteER 方式の Web リサーチ 3 系統と、
現行コード(src ≈16.4k 行 TS)との照合による移行計画 3 案が既に揃っている。

## Key Issues and Assumptions

- rewrite 総量(≈16.4k 行 src + ≈23.5k 行 test)が支配的コストであり、SDK 差異は
  全体の 15-25% 程度にしか効かない(3 案の計画が独立に同結論)。
- Pi ModelRuntime が担っていた model catalog・credential store・OAuth flow は
  どの候補にも存在せず自作必須。devin provider(Connect wire ≈656 行)も全案で自作移植。
- 外部 contract(envelope schemaVersion:1・usage.jsonl・hook・Release asset 名・
  SHA256SUMS・launcher sh/ps1)は全案で不変維持可能と確認済み。
- Rig 固有: 0.x で breaking change 多発(0.40 で 31 件)。`=0.43.x` pin +
  `cheaper-rig-adapter` 隔離 crate で緩和。
- バイナリサイズは未実測(Rig 案 PoC 閾値 ≤20MB、目標 5-15MB)。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| Rig (0xPlaygrounds/rig) | provider 26 種内蔵(Anthropic 含む)、v0.43/8.8k★、adapter 隔離で breaking 吸収可、要件適合ほぼ全項目 Yes | 0.x breaking 頻発、docs 一部陳腐化、catalog/auth は自作 | yes |
| molo (Silwings-git/molo) | kernel/harness/effect 分離が sandboxed tool モデルに構造適合、最小バイナリ見込み | Anthropic native 自作要(provider 自作計 20-35 人日)、単一作者・採用ゼロ・pre-1.0 | no(watchlist) |
| Goose GDK (block/goose) | Anthropic native 内蔵、Block 本体は活発、goose-agent 埋め込み可 | 全 crate 0.1.0-alpha、structured output repair 自作、フル crate 未公開 | maybe(Rig 不達時の同格代替) |
| TS 維持 | rewrite コストゼロ | 113MB SEA・Node 依存が残り目的不達 | no |

## Tentative Conclusions and Hypotheses

- (confirmed) Rig が第一候補。molo 案・Goose 案の両計画も「不達なら Rig へ
  ピボット」と相互に fallback 指名しており、実質的な基準線。
- (confirmed) Phase 0 = PoC/スパイクを go/no-go ゲートとし、バイナリサイズ
  実測(≤20MB)・agent loop の envelope 粒度・repair 動線・custom Wire/Decoder
  見通しを先に潰す構成が妥当。
- (confirmed) Phase 1(core/tool-runtime 移植)までは候補中立の投資であり、
  Rig PoC が不達でも salvage 可能。
- (hypothesis) parity は fake/scripted provider fixture による TS/Rust binary 間
  differential golden test で確立できる。

## Open Questions

- 必須 provider の棚卸し(現行は Pi builtin 全露出 — サポート範囲を絞るか)
- usage meta の field 粒度: `costUsd` 供給源・`cacheRead/Write` 分離・
  `transientRetries`・error 時 partial usage
- `<userConfigDir>/pi/` の auth.json/models.json 移行方針(読み互換 vs 再 login)
- OAuth 対象範囲(openai-codex/anthropic/github-copilot 等の全移植 vs api_key 限定)
- darwin-* target 追加可否、version canonical source(package.json → Cargo.toml)

## Promotion Candidates

- [x] spec-doc: `SPEC-034BbgvL9s41K7NflOddZ6`(proposed)
- [x] adr-doc: `ADR-034BbhP6hJV7DFtv7k7AGs`(proposed)

## Sources and Relations

- `docs/research/pi-runtime-replacement/comparison.md` — 3候補機能調査比較
- `docs/research/pi-runtime-replacement/migration-comparison.md` — 移行計画 3 案比較
- `docs/research/rig-agent-runtime/` / `molo-agent-runtime/` / `goose-sdk/` —
  各 final-report(evidence ledger・audit 含む)と migration-plan
