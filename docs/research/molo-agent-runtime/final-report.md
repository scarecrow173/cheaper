# Research Report: molo as Pi SDK replacement

## Scope and Assumptions

- 調査対象: 依頼者指定の「molo」という名称の LLM/エージェント系 SDK・フレームワーク・ランタイムの同定と、cheaper の `packages/runtime-pi`(Pi SDK + TypeScript + Node.js SEA)の代替としての適合評価。
- 評価基準: 依頼書の「代替候補に要求される能力」8 項目 + 「必ず確認すること」7 項目。
- 調査時点: 2026-10 上旬と推定(molo 最新 release 2026-09-29、repo updated_at 2026-10-02 を確認)。
- スコープ外: cheaper 全体の Rust 移行工数評価、実ビルドによるバイナリサイズ実測、コードスパイク。

## Executive Summary

- **同定結果(確度: High)**: 「molo」は `Silwings-git/molo` — **Mo**del **Lo**op、Rust 製 embeddable agent runtime & harness framework(crates.io `molo` 0.6.0, MIT OR Apache-2.0)。依頼文脈に完全一致し、LLM/agent 領域に同名の競合プロジェクトは確認されなかった。
- **機能適合度は高い**: 埋め込み SDK、マルチターン tool-call ループ、カスタムツール、streaming/abort、usage/turn/tool_call 計測、structured output(validation+retry repair)、sandbox 的 side-effect governance はすべて一次ソースで確認。要求 8 項目中 7 項目が Yes、provider 多様性のみ Partial。
- **決定的リスクは成熟度**: 公開から約 2 ヶ月(2026-08-03 作成)、単一作者、4★/0 fork/月間 40 DL、外部採用ゼロ、pre-1.0 で minor ごとの破壊的変更を明言(2 ヶ月で 4 回 breaking)。本番基盤としての採用は現時点でハイリスク。
- **provider カバレッジが最弱**: built-in は OpenAI Chat Completions 互換のみ。Anthropic は公式 OpenAI 互換 endpoint 経由なら動くが prompt caching 非対応・response_format 無視で「テスト用途」。native 対応は自前 `Provider` 実装が必要。
- **代替比較**: 同じ目的なら成熟した **rig**(0xPlaygrounds, 8.8k★, 20+ provider 内蔵, Anthropic 含む)が存在。molo の独自価値は governed side-effect harness(kernel/harness/effect 分離)のアーキテクチャが cheaper の sandboxed tool モデルと構造的に合う点。
- **結論**: 技術適合度は高いが、依存先としては未成熟すぎる。「採用して全面 Rust 化」の即断は非推奨。watchlist 登録 + 小規模スパイク(FakeProvider + 自作 sandboxed tool + Ollama での PoC)が妥当。ネイティブ化を本気で進めるなら rig を第一候補として比較評価すべき。

## Key Findings

### 同定
- `molo` = https://github.com/Silwings-git/molo (crates.io `molo`)。「**Mo**del **Lo**op」: "An embeddable Rust agent runtime and harness framework for building safe, extensible tool-calling agents"。自らを "framework and SDK, not an end-user agent product" と明確に位置付け、「Claude Code / Codex のような CLI coding agent は molo 上に構築されるべき」と設計 doc に記載 — cheaper の runtime 層と同じレイヤの製品(S4, S11)。
- 同名・類似名の競合なし(mojentic/mofa/smolagents/mojo/mollie/moondream は別物、文脈非適合)。docs.rs の `molo-rt`/`molo-kernel` 0.1.0 は同一プロジェクトの旧構造(yank/再編済み)(S15, Q20)。

### 正体・メンテ状況
- 言語: Rust(edition 2024, MSRV 1.88)。ライセンス: MIT OR Apache-2.0(GitHub 検出 Apache-2.0)。
- リリース履歴(lib.rs): 0.1.0(2026-08-03, yanked)→ 0.3.0(08-16)→ 0.4.0(09-04)→ 0.5.0(09-25)→ 0.6.0(09-29)。**9 releases / 4 breaking を約 2 ヶ月で発行**。
- repo: created 2026-08-03、last push 2026-09-29、**4 stars / 0 forks / 0 open issues / 0 subscribers、40 commits、作者 "silwings" 単独**。crates DL は月間 ~40(molo-agent は累計 27)、dependents は自 workspace のみ — 事実上ゼロ採用。
- 品質面のプラス材料: rustdoc 100% documented、CI badge、conventional commit の詳細 message(破壊的変更に理由と移行方針を記載)、evals/, benches/, 設計 doc 3 本、examples 30 本超。作者の設計力量は高い。

### 構成と API
- Facade `molo` + 層別 crates: `molo-core`(message/run/tool/effect 契約、dep 最小)/ `molo-agent`(AgentKernel, ReActAgent, Memory, ToolRegistry)/ `molo-harness`(policy/approval/audit/executor)/ `molo-coding`(LocalWorkspace, WorkspacePath, CommandExecutor, RipgrepSearcher, GitInspector, PatchApplier)/ `molo-mcp` / `molo-skills` / `molo-openai` / `molo-macros`。feature flag で opt-in、重依存は層で隔離。
- エージェントループ: `ReActAgent`(tool-round limit, cooperative cancellation, usage aggregation)。`run`(text)/`run_request`(RunOutput+RunSummary)/`run_stream`(MessageChunk: Delta/ToolCall/ToolResult/Done/Cancelled)/`run_typed*`(structured)。
- `RunSummary{rounds, tool_calls, usage{prompt,completion,total,cached}, usage_omitted, cache_omitted, finish_reason, latency, provider_model}` — cheaper の usage.jsonl(turns/toolCalls/usage/durationMs/model)とほぼ 1:1 対応。
- `Provider` trait は公開・外部実装可能: required = `chat_with_context` + `stream_chat_with_context` の 2 メソッドのみ。`Box<dyn Provider>` 自体も Provider。
- ツール: `Tool` trait(`schema()` + async `call`)または `#[molo::tool]` macro。tool 失敗は error text を model へ回帰し loop 継続。副作用 tool は `EffectRequest` を返し、harness(policy→approval→sandbox→executor→redact→audit)経由で実行 — cheaper の sandboxed tool 設計と構造的に整合。
- structured output: `run_typed` は JSON Schema(schemars/jsonschema)で最終回答を検証し、失敗は model へ error feedback で retry(=repair)。endpoint 側は `response_format` を 3 mode で送信(Native: json_schema+strict 自動判定 / JsonObject / Off)。

### provider カバレッジ(最弱項)
- built-in: `molo-openai` のみ。`POST {base_url}/chat/completions` 固定(Responses API 非対応)。base_url/api_key/model はコンストラクタ指定 → OpenAI, DeepSeek, Zhipu, Moonshot, Ollama(api_key 空可と明記), vLLM, LiteLLM, OpenRouter 等 OpenAI 互換は網羅。
- **Anthropic**: native provider なし(コードベースに "anthropic" の言及ゼロ)。Anthropic 公式の OpenAI 互換 endpoint(api.anthropic.com/v1)経由なら動く見込みだが、公式 doc 上 prompt caching 非対応・response_format 無視・tool strict 無視・「主にテスト用途」— cheaper の cached_tokens 計測・コスト最適化動機を損なう。native 対応には `Provider` 実装の自作が必要(Messages API + SSE 写像。trait は2メソッドのみで境界は小さい)。
- cheaper の `CheaperProviderPlugin`(devin CLI delegation 等)も同様に Rust で自前実装が必要。

### バイナリサイズ見通し
- Pure Rust・外部ランタイム不要・静的単一バイナリ化が可能。workspace は ~15K SLoC/1MB と小さい。
- dep: `molo-openai` = reqwest 0.13 + tokio + serde 系。tokio も net/rt/sync/time に限定。推定で release+strip+LTO 後 4-8MB 程度(Low-Medium confidence、実測なし)→ Node SEA(~100MB級)から大幅削減の見込み。
- 留意: molo-openai の reqwest は default features 有効のため TLS は native-tls 系の可能性が高い。Windows(Schannel)/macOS では問題ないが、静的 musl Linux では OpenSSL の取り回しが必要になるか、upstream への feature flag 化要望が必要(推定)。

## Evidence Matrix

| Claim | Support | Source IDs | Confidence | Notes |
|-------|---------|------------|------------|-------|
| "molo" = Silwings-git/molo(Rust agent runtime) | repo + crates.io + docs.rs が同一 README/owner で一致。他候補検出せず | S1,S5,S6,Q20 | High | 同定確定 |
| ライブラリとして埋め込み可能 | "framework and SDK, not an end-user agent product"、facade+層別 crate、CLI coding agent は上位に構築と明記 | S4,S6,S11 | High | Pi SDK と同じレイヤ |
| マルチターン tool-call ループ + turn/tool 計測 | ReActAgent、RunSummary.rounds/tool_calls | S4,S8,S12,S16 | High | usage.jsonl 要件と一致 |
| カスタム tool 定義可能 | Tool trait、#[molo::tool]、ToolRegistry、EffectRequest/harness、molo-coding primitives | S4,S11,S12,S16 | High | sandboxed tool は自前 or coding feature |
| OpenAI 対応 | OpenAiProvider、chat/completions | S10 | High | Responses API は非対応 |
| OpenAI 互換(Ollama 等) | Ollama/DeepSeek/Zhipu/Moonshot 明記、api_key 空可 | S10,S16 | High | cheaper の Ollama 検証経路と合致 |
| Anthropic 対応 | native provider なし。OpenAI 互換 endpoint 経由は可能だが prompt caching 不可等の制約 | S10,S13 | High(制約事実) | Partial 判定 |
| provider プラグイン機構 | Provider trait 公開・必須2メソッド・外部実装可 | S7 | High | 実装は自前(Rust) |
| usage 取得 | RunSummary.usage + cached + omitted latch | S8,S16 | High | |
| streaming / abort | run_stream MessageChunk、CancellationToken、Cancelled chunk | S4,S12 | High | |
| structured output + repair | run_typed、jsonschema 検証 + error feedback retry、response_format 3 mode | S4,S10 | High | |
| 非対話 stdin/stdout JSON CLI | ライブラリのためホスト(cheaper)が envelope 実装。阻害要因なし | S4,S11 | High | 現行 TS と同じ責務配置 |
| 小さいネイティブバイナリ | ランタイム不要・dep 軽量・15K SLoC。実測なし | S5,S9,S10 | Medium | 4-8MB は推定 |
| メンテ・採用が未成熟 | 4★/0 fork/40 DL 月/単一作者/外部 dependent なし/2ヶ月で4 breaking | S1,S2,S3,S5,S15 | High | 最大のリスク |
| rig が成熟な代替 | 8.8k★、20+ provider(Anthropic 含む)、MIT | S14 | High | 比較対象として記録 |

## Analysis — cheaper 要件適合表

| cheaper 要件 | 判定 | 根拠(Source) |
|--------------|------|--------------|
| ライブラリ埋め込み(headless worker runtime) | **Yes** | S4,S11。「製品ではなく SDK」明言、kernel/harness 分離で cheaper 側が outer loop を握れる |
| エージェントループ(複数ターン tool-call、turn/tool call 計測) | **Yes** | S4,S8,S12,S16。ReActAgent + RunSummary(rounds/tool_calls/latency/finish_reason/provider_model) |
| カスタムツール(sandboxed read/grep/find/write) | **Yes** | S4,S11,S12。Tool trait 直実装 or #[molo::tool]。molo-coding の WorkspacePath(traversal 拒否)/LocalWorkspace/CommandExecutor/RipgrepSearcher が sandboxed tool の下請けとして使える。governed effect 経路も用意 |
| provider: Anthropic | **Partial** | S10,S13。built-in なし。OpenAI 互換 endpoint 経由は動くが prompt caching 非対応・response_format 無視。native は自前 Provider 実装(境界は小さいが工数必要) |
| provider: OpenAI | **Yes** | S10 |
| provider: OpenAI-compatible(Ollama 等) | **Yes** | S10,S16。Ollama 明記 |
| provider プラグイン機構 | **Yes(trait)** | S7。ただしプラグイン=動的拡張ではなく、自前 Provider 実装の静的登録。CheaperProviderPlugin 相当の機構は cheaper 側で設計 |
| usage(token)取得 | **Yes** | S8,S16。cached_tokens + omitted latch まであり、現行記録仕様と整合 |
| streaming / abort | **Yes** | S4,S12。MessageChunk + CancellationToken、run 単位の協調 cancel |
| structured output(JSON schema repair 含む) | **Yes** | S4,S10。jsonschema validation → error feedback retry、endpoint 側 response_format 3 mode |
| 非対話 stdin/stdout JSON envelope | **Yes(ホスト実装)** | S4。ライブラリのため cheaper が CLI/envelope を実装する責務配置は現行と同じ |
| 小さいネイティブバイナリ | **Yes(推定)** | S5,S9,S10。ランタイム不要の単一バイナリ、dep は tokio+reqwest 中心。実測値は未確認 |

**総合適合度**: 機能要件の ~85-90% を一次ソース確認済みで充足。不足は Anthropic native と、採用に伴う cheaper 全面 Rust 化(CLI、worker registry、count-tokens、usage.jsonl、APM hooks、pricing 等の再実装)という依存先外のコスト。

## Gaps / Uncertainties

- 実バイナリサイズ・musl 静的リンク可否(reqwest TLS backend)は未実測。
- Anthropic native Provider の実装工数・Messages API/SSE 写像の詳細は未検証(trait 境界は小さいが確実ではない)。
- 作者の継続メンテ意思・コミュニティ形成の見通しは不明。単一作者プロジェクトの bus factor。
- 0.x 破壊的変更の頻度が高く、追従コストが読めない(pre-1.0 で migration path 付与の方針は明言されている)。
- 設計 doc 2 本が中国語のみ(チーム内の可読性次第で障壁)。
- cheaper 全面 Rust 化に伴う周辺機能の移植評価(本調査スコープ外だが決定材料)。
- rig 等他候補との詳細 API 比較は未実施(規模・provider 数の定量差のみ確認)。

## Recommendations or Next Actions

1. **即採用は非推奨**。機能適合は高いが、2 ヶ月・単一作者・ゼロ採用の pre-1.0 crate に cheaper のランタイムを委ねるのはリスク過大。
2. **watchlist + 小スパイク**: `FakeProvider` + 自作 `Tool`(read/grep/write 相当)+ Ollama(OpenAiProvider)で非対話 PoC を作り、binary サイズ実測・RunSummary の JSON envelope 化・cancellation 挙動を検証(1-2 日規模)。
3. **ネイティブ化を推進するなら rig を比較評価**: 20+ provider(Anthropic native 含む)・8.8k★・実績あり。governed side-effect harness が cheaper に必要なら、molo の設計を参照しつつ rig + 自前 harness という構成も検討可能。
4. **Anthropic が必須 provider なら molo 採用時は native Provider 自作が前提** — Provider trait は実装可能だが、Messages API/SSE/usage/cache_control 写像の保守責任が cheaper に移る。
5. 中間案: 「TS 維持 + molo/rig を別言語 worker として段階導入」はアーキテクチャを複雑化するだけで、バイナリ化動機に反するため非推奨。

## Source List

- S1: https://github.com/Silwings-git/molo — リポジトリ(stars/commits/構成)
- S2: https://api.github.com/repos/Silwings-git/molo — repo metadata(created/pushed/license/owner)
- S3: https://api.github.com/repos/Silwings-git/molo/commits — commit log(0.6.0 changelog commit)
- S4: https://raw.githubusercontent.com/Silwings-git/molo/master/README.md — 機能一次情報
- S5: https://lib.rs/crates/molo — version 履歴・DL 数・crate サイズ
- S6: https://docs.rs/molo/latest/molo/ — 0.6.0 API・feature flags
- S7: https://docs.rs/molo-core/latest/molo_core/provider/trait.Provider.html — Provider trait
- S8: https://docs.rs/molo-core/latest/molo_core/run/struct.RunSummary.html — RunSummary 定義
- S9: https://docs.rs/crate/molo-openai/latest — molo-openai deps
- S10: https://raw.githubusercontent.com/Silwings-git/molo/master/molo-openai/src/lib.rs + molo-openai/Cargo.toml — chat/completions、timeout、StructuredOutputMode、reqwest dep
- S11: https://raw.githubusercontent.com/Silwings-git/molo/master/docs/architecture.md — 設計・副作用 chain・破壊的変更方針
- S12: https://raw.githubusercontent.com/Silwings-git/molo/master/docs/react-agent-flow.md — kernel/harness flow
- S13: https://docs.anthropic.com/en/api/openai-sdk — Anthropic OpenAI 互換 endpoint の公式制約
- S14: https://github.com/0xPlaygrounds/rig / https://rig.rs/docs/ / https://docs.rs/crate/rig-core/latest — 代替候補 rig
- S15: https://crates.io/crates/molo-agent — molo-agent registry スニペット(downloads/dependents)
- S16: https://raw.githubusercontent.com/Silwings-git/molo/master/examples/usage.rs — usage/rounds/tool_calls 実装例
