# Research Report: Rig as Pi SDK replacement

調査日: 2026-10 時点(評価基準 version: rig / rig-core / rig-agent **0.43.0**, 2026-09-30 リリース)

## Scope and Assumptions

- 対象: `0xPlaygrounds/rig` — Rust 製 LLM アプリケーション/エージェントライブラリ。
- 目的: cheaper(`packages/runtime-pi` が `@earendil-works/pi-ai` + `@earendil-works/pi-coding-agent` 0.85.1 に依存する TypeScript/Node.js SEA 配布 CLI)の worker runtime を Rust ネイティブ化する際の基盤候補としての適合度評価。
- 前提: 採用する場合は CLI 本体・sandboxed tools・envelope 処理も含めた Rust 全面 rewrite となる。Rig が提供するのは provider 抽象 + agent loop であり、CLI 層・tool 実装・sandbox 境界は cheaper 側の責務のまま。
- 評価は公開ドキュメント・リポジトリ・レジストリの一次ソース中心。実測していない数値(バイナリサイズ等)は推測と明記。

## Executive Summary

**Rig は cheaper の機能要件をほぼ全て満たす、現時点で最も成熟した Rust 製 LLM agent framework の一つ。** ライブラリ埋め込み、マルチターン tool-call ループ、カスタム tool、26+ provider(Anthropic/OpenAI/Ollama/Gemini/Bedrock 等)、run 集計・per-call の usage、streaming + cancel、structured output(schema + retry + 中途 invalid tool-call の repair)まで一次ソースで確認できた。

単一ネイティブバイナリ化も現実的: `rig-core` は tokio/reqwest を通常依存に持たず、transport は opt-in(`rig-reqwest` → reqwest+tokio+rustls)。release profile は `lto + opt-level="s"`。Node SEA の ~80-120MB 級から一桁 MB〜十数 MB 級への削減が見込める(推測・要実測)。

最大のリスクは **API 安定性**。0.x で每リリース(2-4週間毎)に複数の breaking change が入る(0.40 は 31件)。README が "future updates will contain breaking changes" を明言し、後方互換要求の issue #628 は not planned で close。対策は version pin + MIGRATING.md ベースの計画的 upgrade。加えて 0.4x の再編(Wire/Decoder/Transport アーキテクチャ)に公式 docs が追従し切っていない箇所がある。cheaper 側の総移行コスト(TS→Rust 全面 rewrite)が支配的リスクである点も忘れない。

## Key Findings

1. **正体・メンテナンス**: MIT license の Rust ライブラリ。GitHub 8.8k stars / 987 forks / 1,558 commits、open issues 59、2024-06 作成。crates.io `rig-core` 累計 ~330万 DL、dependents ~210、ほぼ 2-4 週間毎にリリース(0.43.0 = 2026-09-30)。MSRV Rust 1.95.0 / edition 2024。[S1][S2][S11]
2. **構成**: `rig` facade + `rig-core`(provider-neutral contracts: `Model`/`Wire`/`Transport`/`DynModel`、provider 実装、tool/memory/vector-store contracts)+ `rig-agent`(classic runtime: `AgentBuilder`/`Agent`/serializable `AgentRun`/hooks/extractor)+ 20+ companion crates。[S1][S3][S4]
3. **Providers**: rig-core 内蔵 = anthropic, azure, chatgpt, cohere, copilot, deepseek, doubleword, gemini, groq, huggingface, hyperbolic, llamacpp, minimax, mira, mistral, moonshot, **ollama**, openai(OpenAI-shaped dialect 群), openrouter, perplexity, together, venice, voyageai, xai, xiaomimimo, zai。Bedrock は `rig-bedrock`、Vertex AI は `rig-vertexai`。[S5][S1]
4. **Ollama**: 専用 provider(ネイティブ `/api/chat`、NDJSON decode)。`OLLAMA_API_BASE_URL`/`OLLAMA_API_KEY` env、QWEN3_5/3_6/3_8/QWEN3_CODER 等の定数あり(cheaper e2e の `qwen3.5` と整合)。[S6]
5. **Agent loop**: `AgentBuilder::default_max_turns`、tool/dynamic_tool/retrieved_tools、`model_route`、hooks(`CompletionCallEvent`/`ModelTurnFinished`/`ToolCallDelta`/`RunSettled` 等)。`AgentRun` は serializable sans-I/O state machine — ヘッドレス worker に好適。[S7][S4]
6. **計測**: `PromptResponse.usage`(run 集計)+ `completion_calls: Vec<CompletionCall>`(per-request usage)+ `requests()`。`Usage` は input/output/total/cached_input/cache_creation_input/tool_use_prompt/reasoning tokens を全 provider 同一 contract の `Option<u64>` で保持。cache 対応済みで cheaper の usage.jsonl 要件を上回る粒度。[S8][S16]
7. **Tools**: `PortableTool`(context-free)/`Tool`(`ToolContext` 付き)trait、`#[rig_tool]` macro、`ErasedTool` adapter。sandboxed read/grep/write の自前実装をそのまま登録可能。[S9]
8. **Streaming/abort**: `MultiTurnStreamItem` の streaming run、`StreamingCompletionResponse::cancel()`/`Streamed` drop = cancel、`PromptError::prompt_cancelled`。[S14]
9. **Structured output**: `output_schema`/`output_schema_raw`/`output_mode` + `StructuredOutputError`。`ExtractorBuilder::<T>::retries(n)`(submit output tool + retry)。中途 invalid tool-call を `StreamedResolution::Repaired` で repair する protocol 内蔵 + "recover from malformed streamed tool arguments"(#2532)。[S7][S14][S12][S15]
10. **Binary size**: rig-core に tokio/reqwest なし(0.43 で transport 分離)、opt-in `rig-reqwest`(reqwest 0.13+tokio+hyper+rustls、native-tls 切替可)。workspace profile は lto+opt-level=s。実測値なし — 推測: release binary 概ね 5-15MB 級。[S11][S2]
11. **WASM**: `wasm32-unknown-unknown` を portable core + classic runtime でサポート(WASI 非対象、rig-rmcp/MCP は native-only)。[S1][S4]
12. **採用実績**: VT Code(Rust terminal coding agent)、Neon app.build、Coral Protocol、St Jude、Dria、Nethermind、ilert、Archestra 等。ただし VT Code は自社 ARCHITECTURE で「agent runtime は自前 harness、Rig は provider/model validation と reasoning payload shaping に限定」と明記 — coding agent 文脈での Rig 採用の限界事例。[S1][S22]
13. **リスク(最重要)**: 0.x で breaking change 多発。0.43.0 だけで ~13件、0.40 は 31件。README が breaking を予告、issue #628 は not planned close。MIGRATING.md は cargo public-api diff ベースで整備されており移行手順の質は高いが、追従コストは継続的。docs.rig.rs の一部ガイド(custom provider 等)が旧 trait API のまま。[S12][S13][S18][S19][S20][S23]
14. **カスタム provider**: 0.43 では `Wire`(endpoint を data として記述)+`Decoder`+`Transport` 分離設計。旧 `CompletionModel` trait は消失。`serve::adapters` で provider author 向け domain trait を保持。実装は可能だが旧 docs と乖離しており実装コストの見積は要 PoC。[S3][S5][S23]

## Evidence Matrix

| Claim | Support | Source IDs | Confidence | Notes |
|-------|---------|-----------|------------|-------|
| Rig は MIT・Rust 製・活発にメンテ(8.8k stars, 月2回級 release, 0.43.0 最新) | GitHub repo + crates.io | S1, S2 | High | |
| MSRV 1.95 / edition 2024 | workspace Cargo.toml | S11 | High | ツールチェーン要件が新しい |
| Anthropic/OpenAI/Ollama/Gemini/Groq 等 26 providers 内蔵 + Bedrock/Vertex は companion crate | providers module docs + README | S5, S1 | High | |
| Ollama は専用 wire + env 設定(QWEN3_5 定数あり) | ollama module docs | S6 | High | cheaper e2e model と一致 |
| OpenAI-compatible endpoint は openai dialect 経由 | providers module(dialect constants) | S5 | High | 任意 base_url 経路の詳細は要コード確認 |
| マルチターン agent loop + max_turns | AgentBuilder::default_max_turns | S7 | High | |
| turn/tool call 計測可能 | PromptResponse.completion_calls/requests + hooks | S8, S7 | High | per-call usage 付き |
| usage token 取得(cache 含む Option<u64> 群) | Usage struct | S16 | High | provider 未報告時は None |
| カスタム tool 定義(read/grep/write 相当) | Tool/PortableTool trait + #[rig_tool] | S9 | High | ToolContext で host state 注入可 |
| streaming + abort | cancel()/drop/PromptError::prompt_cancelled | S14 | High | |
| structured output + repair | output_schema/output_mode + Extractor retries + StreamedResolution::Repaired | S7, S15, S14, S12 | High | "repair" は retry + mid-stream repair の組合せ。壊れた JSON の構文的修復そのものではない |
| ライブラリ埋め込み・非対話 CLI 化 | library crate; AgentRun は sans-I/O state machine | S4 | High | stdin/stdout envelope は cheaper 側実装 |
| 小さいネイティブバイナリ | rig-core に tokio/reqwest なし、transport opt-in、lto+opt-level="s" | S11, S2 | Medium | 実測なし、5-15MB は推測 |
| WASM 対応(browser wasm32; WASI 不可) | README + rig-agent docs | S1, S4 | High | cheaper には不要だが portability 証跡 |
| API 不安定・breaking 多発 | README warning + issue #628(not planned)+ MIGRATING(0.40 は 31 breaking) | S18, S19, S13, S12 | High | version pin + 計画 upgrade で緩和 |
| docs が API に追随し切れていない | custom provider ガイドが旧 CompletionModel 前提;FAUN.dev 指摘 | S23, S20 | Medium-High | 0.4x 再編の影響 |
| 実装の学習コストが高め | 新 Wire/Decoder/Transport 設計 + FAUN.dev「trait bounds が初心者を遅らせる」 | S3, S20 | Medium | |
| 採用実績あり(coding agent 含む)が VT Code は agent runtime を自前化 | README adopter list + VT Code ARCHITECTURE | S1, S22 | High | Rig の agent loop を採るかは検討点 |

## Analysis — cheaper 要件適合表

| cheaper の要求能力 | 適合度 | 証拠 | 補足 |
|---|---|---|---|
| ライブラリ埋め込み(ヘッドレス worker runtime) | Yes | Rig は library。`AgentRun` が serializable sans-I/O state machine で host-driven bus 設計 [S4] | stdin/stdout JSON envelope は cheaper 側で実装(serde で容易) |
| エージェントループ(マルチターン tool-call, turn/tool call 計測) | Yes | `default_max_turns`, `completion_calls`, `requests()`, AgentHook イベント群 [S7][S8] | turns/toolCalls 集計は `completion_calls` + hook で正確に取れる |
| カスタムツール(sandboxed read/grep/find/write) | Yes | `Tool`/`PortableTool` trait, `#[rig_tool]`, `ToolContext`, `DynamicTool` [S9] | sandbox 境界(allowlist, path 制限)のロジックは cheaper 側で再実装 |
| Anthropic | Yes | `providers::anthropic` [S5] | Messages API wire |
| OpenAI | Yes | `providers::openai`(Responses + Chat Completions wire) [S5] | |
| OpenAI-compatible endpoint | Yes | openai module が OpenAI-shaped dialect 群を内包 [S5] | 任意 base_url の指定経路は実装時確認 |
| Ollama | Yes | `providers::ollama` 専用 wire、env 設定、QWEN3_5 定数 [S6] | cheaper の docker 検証と整合 |
| その他 provider プラグイン機構 | Partial-Yes | カスタム provider は `Wire`+`Decoder`+`Transport` で実装可。`serve::adapters` あり [S3][S5] | devin provider 相当(CLI delegation auth)は custom Wire or OpenAI-compat mapping が必要。実装コスト要検証 |
| usage(token)取得 | Yes | `Usage` struct(input/output/total/cache/reasoning、Option<u64>)、run 集計 + per-call [S16][S8] | provider 未報告は None — Pi の meta.usage 相当は表現可能 |
| streaming/abort | Yes | streaming run, `cancel()`, drop=cancel, `prompt_cancelled` [S14] | |
| structured output(JSON schema + repair) | Yes | `output_schema(_raw)`, `output_mode`, Extractor retries, mid-stream invalid tool-call repair [S7][S15][S14] | cheaper の「repair」要件には retry + repair protocol で対応可 |
| 非対話・stdin/stdout JSON envelope CLI | Yes(自前) | library ゆえ自由。`PromptResponse` 等は Serialize [S8] | envelope schema は cheaper が定義 |
| 小さいネイティブバイナリ | Partial-Yes(高見通し) | rig-core は tokio/reqwest なし、transport opt-in、lto+opt-level=s [S11] | 実測なし。推測 5-15MB vs Node SEA 大幅削減。musl static build も選択肢 |

### 副次効果・注意点
- `CacheCost`/`CacheRates` は「caller が価格表を供給」設計 — cheaper の runtime catalog/pricing.json 解決と思想が一致、そのまま接続可能。[S16 関連, S5]
- `rig-cassette`(record/replay)は e2e テスト・offline replay に有用(Ollama 不要な cassette 検証を cheaper に移植可能)。[S1]
- MCP 連携が必要になれば `rig-rmcp`(native-only)。現状 cheaper に MCP 要件は無し。
- `rig-ecs` は heavyweight な別 runtime;cheaper は `rig-agent`(classic)で十分(YAGNI)。

## Gaps / Uncertainties

1. **実測バイナリサイズ**: 公開データ無し。PoC で `cargo build --release`(+strip, 可能なら musl)の実測が必要。
2. **カスタム provider の実装コスト**: 0.43 の Wire/Decoder 設計での新規 provider 実装例は docs が古く、実作業量は未検証。`devin` provider(CLI delegation auth + データ隔離)相当の実現経路が最大の機能的不確定点。OpenAI-compatible endpoint として扱えるなら dialect 経由でほぼ無料。
3. **docs の新設計への追従度**: guides の一部が旧 API。docs.rs API reference と source が正。
4. **upgrade 運用コストの実測**: MIGRATING.md は整備されているが、2-4週毎の breaking 追従を実際に運用した場合のコストは未知数。「pin して四半期毎に upgrade」運用を推奨。
5. **tokio 依存の現実**: native HTTP では rig-reqwest→tokio が事実上必須。Rust なので問題ではないが「runtime フリー」ではない。

## Recommendations or Next Actions

1. **結論: Rig は技術要件を満たす有力候補。** 機能面のギャップはカスタム provider 経路の検証のみ。採否は「TS→Rust 全面 rewrite コスト vs 配布サイズ削減便益」の判断に依存(Rig 自体の欠点ではない)。
2. **PoC(小):** `rig`(default features + `reqwest`)で (a) Ollama+qwen3.5、(b) Anthropic or OpenAI に対し、sandboxed tool 2-3個を `#[rig_tool]`/Tool trait で登録し、multi-turn run の `PromptResponse.usage`/`completion_calls` を JSON envelope で出力する最小 CLI を作る。binary size を実測。
3. **version pin 戦略**: `rig = "=0.43.x"` 相当で pin、upgrade は MIGRATING.md 手順に従い計画的に。silent behavior changes 節を必読。
4. **devin provider**: OpenAI-compatible として表現できるか仕様確認 → 不可なら `Wire`+`Decoder` の custom provider 実装を試作。
5. **agent loop の選択**: VT Code が Rig agent runtime ではなく自前 harness を選んだ事例を参照。cheaper の要件(turn/tool 計測、repair、非対話)には rig-agent で足りる見込みだが、`rig-core` の Model/Streaming だけ採用して自前 loop にする代替も比較検討の価値あり。
6. **検証容易性**: `rig-cassette` の record/replay を e2e offline 化に活用検討。

## Source List

- S1: https://github.com/0xPlaygrounds/rig — repo README(stars/license/features/adopters/companion crates/WASM/breaking warning)
- S2: https://crates.io/crates/rig-core — versions/downloads/dependencies
- S3: https://docs.rs/rig-core/latest/rig_core/ — rig-core API(0.43.0)
- S4: https://docs.rs/rig-agent/latest/rig_agent/ — rig-agent API(0.43.0)
- S5: https://docs.rs/rig-core/latest/rig_core/providers/index.html — provider module 一覧
- S6: https://docs.rs/rig-core/latest/rig_core/providers/ollama/index.html — Ollama provider
- S7: https://docs.rs/rig-agent/latest/rig_agent/agent/struct.AgentBuilder.html — AgentBuilder methods
- S8: https://docs.rs/rig-agent/latest/rig_agent/run/response/struct.PromptResponse.html — PromptResponse/usage/completion_calls
- S9: https://docs.rs/rig-core/latest/rig_core/tool/index.html — Tool/PortableTool
- S10: https://docs.rig.rs/ — 公式ドキュメント top
- S11: https://raw.githubusercontent.com/0xPlaygrounds/rig/main/Cargo.toml + crates/rig-core/Cargo.toml — MSRV/features/release profile
- S12: https://raw.githubusercontent.com/0xPlaygrounds/rig/main/CHANGELOG.md — 0.43.0 changelog
- S13: https://raw.githubusercontent.com/0xPlaygrounds/rig/main/MIGRATING.md — breaking change 履歴と移行手順
- S14: https://docs.rs/rig-core/latest/src/rig_core/streaming/mod.rs.html ほか — cancel/repair protocol source
- S15: https://docs.rs/rig-agent/latest/rig_agent/extractor/index.html — Extractor
- S16: https://docs.rs/rig/0.43.0/rig/completion/struct.Usage.html — Usage struct
- S17: https://github.com/0xPlaygrounds/rig/discussions/2225 — v0.41.0 release notes
- S18: https://github.com/0xplaygrounds/rig/blob/main/README.md — breaking changes warning
- S19: https://github.com/0xPlaygrounds/rig/issues/628 — 後方互換 issue(closed not planned)
- S20: https://faun.dev/toolbox/rig/review/ — 外部レビュー(maturity 低評価、docs lag)
- S21: https://dev.co/ai/frameworks/rig — 外部レビュー(pin 推奨)
- S22: https://github.com/vinhnx/vtcode/blob/d2c579d3/docs/ARCHITECTURE.md — VT Code の Rig 利用境界
- S23: https://docs.rig.rs/guides/extension/write_your_own_provider — custom provider ガイド(旧 API)
