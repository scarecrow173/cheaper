# Running Summary — Rig 調査

## Iteration 1 確定事実(primary sources)

### 正体・メンテナンス (T1)
- Rig = 0xPlaygrounds/rig、Rust 製 LLM アプリ/エージェント用ライブラリ。MIT license。[S1]
- GitHub: 8.8k stars / 987 forks / 1,558 commits / open issues 59 / created 2024-06-05。[S1]
- crates.io `rig-core`: 最新 0.43.0 (2026-09-30 公開)。total downloads ~3.3M、dependents 211。0.26→0.43 までほぼ 2-4 週間ごとの定期リリース。[S2]
- アーキテクチャ再編済み: `rig`(facade) + `rig-core`(provider-neutral contracts: Model/Wire/Transport/DynModel) + `rig-agent`(classic agent runtime) + companion crates(rig-bedrock, rig-reqwest, rig-http, rig-rmcp, rig-cassette, rig-ecs, vector stores 等)。[S1][S3][S4][S11]
- 採用実績: VT Code, Neon app.build, Coral Protocol, St Jude, Dria, Nethermind, Archestra, ilert 等 + awesome-rig。[S1]

### Providers (T2)
- rig-core providers module: anthropic, azure, chatgpt, cohere, copilot, deepseek, doubleword, gemini, groq, huggingface, hyperbolic, llamacpp, minimax, mira, mistral, moonshot, ollama, openai(複数 dialect 含む), openrouter, perplexity, registry, together, venice, voyageai, xai, xiaomimimo, zai。[S5]
- AWS Bedrock = `rig-bedrock` companion crate。Vertex AI = `rig-vertexai`。[S1]
- Ollama: 専用 provider(ネイティブ `/api/chat` NDJSON)。`Ollama::from_env` は `OLLAMA_API_BASE_URL`/`OLLAMA_API_KEY` を読む。QWEN3_5/3_6/3_8/QWEN3_CODER constants あり。[S6]
- OpenAI-compatible: openai module が "every OpenAI-shaped dialect" を内包(DEEPSEEK/MIRA dialect constants)。[S5]

### Agent loop / tools (T3)
- `AgentBuilder`: `default_max_turns`, `tool`/`dynamic_tool`/`retrieved_tools`, `tool_choice`, `output_schema`/`output_schema_raw`/`output_mode`, `memory`, `model_route`, `add_hook`。[S7]
- `PromptResponse`: `output`, `usage`(run 集計), `completion_calls: Vec<CompletionCall>`(request 単位), `messages`, `requests()`。[S8]
- `Usage`: input/output/total/cached_input/cache_creation_input/tool_use_prompt/reasoning tokens、全て `Option<u64>`、全 provider で同一 contract。[S16]
- `AgentHook`: CompletionCallEvent, ModelTurnFinished, TextDelta, ToolCallDelta, RunSettled 等。`MultiTurnStreamItem` streaming。[S4][S7]
- Tool: `PortableTool`/`Tool` trait + `#[rig_tool]` macro + `ErasedTool`。[S9]
- `AgentRun` = serializable sans-I/O state machine。ヘッドレス適合。[S4]

## Iteration 2 確定事実

### MSRV / リリース運用 (T1/T6)
- MSRV: Rust 1.95.0、edition 2024(workspace.package rust-version)。高めの MSRV。[S11]
- workspace release profile: `lto = true`, `opt-level = "s"` — サイズ志向。[S11]
- CHANGELOG は Keep a Changelog + SemVer 表記だが 0.x で breaking 多数: 0.43.0 単体で ~13件の breaking エントリ。[S12]
- MIGRATING.md: 「0.36, 0.37, 0.40, 0.41, 0.43 が disruptive。0.40 だけで 31 breaking changes」。cargo public-api diff を骨格にした厳密な移行ガイド運用。[S13]
- Silent behavior changes 節あり(0.31 TLS rustls 化、0.33 preamble→system message 化、0.35 tool output 文字列の JSON encode 廃止等)。[S13]
- 0.43 で「rig-core has no default transport and no reqwest/tokio」(breaking #2397) — transport は opt-in。[S12]
- #2532: "recover from malformed streamed tool arguments" — ツール引数 JSON の修復機構あり。[S12]
- Usage counters が Option<u64> 化(#2535) — provider が usage を返さない場合を表現可能。[S12]

### Cancellation / structured output (T3)
- streaming: `StreamingCompletionResponse::cancel()`(AbortHandle)、`Streamed` は "drop the stream to cancel"。run 側は `PromptError::prompt_cancelled`/`cancel_error`。[S14]
- StreamedTurnAssembler: ストリーム中の invalid tool call を即検出し `StreamedResolution::Repaired`(同ストーム継続)/`TurnAbandoned` を選択可能 — tool-call repair が protocol に内蔵。[S14]
- Extractor: `ExtractorBuilder::<T>::new(model).retries(n)` — submit output tool による型付き抽出 + retry。`extract_with_usage` は失敗 attempt の usage も集計(v0.41 で修正済み)。[S15][S17]
- `output_schema`/`output_mode`/`StructuredOutputError` で agent 自体にも structured output 経路あり。[S7][S16]

### 批判・リスク (T6)
- README 公式警告: "Here be dragons! ... future updates **will** contain **breaking changes**"。[S18]
- Issue #628「minor update で breaking changes を出すな」→ **closed as not planned**。破壊的変更は方針として継続。[S19]
- FAUN.dev review: score 71/100、"maturity & stability" が最低評価。"Guides lag behind the fast-moving API"。[S20]
- dev.co review: "Lock to specific minor versions; review CHANGELOG before upgrades"。[S21]
- 重要なニュアンス: VT Code の ARCHITECTURE.md —「Rig の request-shaping pattern は借用するが runtime control は rig-core agents に委譲せず、自前 harness を維持」。即ち VT Code は provider 抽象としてのみ Rig を利用。[S22]
- docs.rig.rs の custom provider ガイドは旧 `CompletionModel` trait ベースの記述;0.43 では CompletionModel trait は消失し `Wire`+`Decoder`+`Transport` 設計に置換。docs が API に追従していない実例。[S20][S23]

### Binary size (T4)
- rig-core 通常依存は軽量(tokio/reqwest 無し)。native HTTP には rig-reqwest(opt-in, reqwest 0.13→tokio+hyper+rustls)が事実上必要。[S2][S11]
- 実測バイナリサイズの公開データは発見できず → 推測扱い。

## 残ギャップ(監査対象)
- 実測バイナリサイズ → 依存構成からの推定のみ(Low confidence)。
- カスタム provider の新アーキテクチャ(Wire/Decoder)における実装コスト → trait 構成は確認済みだが実装例の深掘り未実施(Medium)。
- cheaper の `devin` provider plugin 相当(CLI delegation auth)の実現経路 → OpenAI-compatible 経由 or カスタム Wire。要 PoC。

## 監査後の結論
調査は網羅済み。2イテレーションで一次ソース+批判ソース双方を確認。final-report へ移行可能。
