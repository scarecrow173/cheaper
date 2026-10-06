# query-log.md — 実行クエリ記録

## Iteration 1: 同定

| # | Query / URL | 目的 | 結果 |
|---|------------|------|------|
| Q1 | `"molo" LLM agent framework SDK` | 同名プロジェクトの同定 | 即ヒット: crates.io/crates/molo, github.com/Silwings-git/molo(Rust agent runtime "Model Loop") |
| Q2 | `"molo" Rust LLM agent library github` | Rust 系エージェントライブラリ確認 | 同上。crates の版 0.3/0.4、docs.rs 確認 |
| Q3 | fetch github.com/Silwings-git/molo | repo 概況 | 4★/0 fork/40 commits。workspace 構成(molo-core/agent/harness/coding/mcp/skills/openai/macros/cli) |
| Q4 | fetch crates.io/crates/molo | registry 情報 | 403 で取得不可(2回) → lib.rs で代替 |
| Q5 | fetch raw README.md(master) | 機能一次確認 | version 0.6、feature flags、ReActAgent、OpenAiProvider、RunSummary、CancellationToken、MCP、skills、coding SDK |

## Iteration 2: API・メタデータ深掘り

| # | Query / URL | 目的 | 結果 |
|---|------------|------|------|
| Q6 | fetch docs.rs/molo/latest | 最新版・license・deps | 0.6.0(2026-09-29)、MIT OR Apache-2.0、100% documented、feature flag 一覧 |
| Q7 | fetch docs.rs Provider trait | カスタム provider 可否 | `Provider: Send+Sync`、required = `chat_with_context`+`stream_chat_with_context`、provided = chat/stream_chat/model/capabilities。外部実装可能 |
| Q8 | fetch lib.rs/crates/molo | version 履歴・採用度 | 9 releases(4 breaking): 0.1.0(8/3,yanked)→0.3.0(8/16)→0.4.0(9/4)→0.5.0(9/25)→0.6.0(9/29)。40 DL/月。MIT/Apache、15K SLoC |
| Q9 | fetch docs.rs crate/molo-openai | provider 依存 | reqwest 0.13, tokio, serde 系。203KB src |
| Q10 | fetch examples/usage.rs | usage/turn 計測の実装確認 | `RunSummary{rounds, tool_calls, usage{prompt/completion/total/cached}, usage_omitted, cache_omitted}`、streaming `include_usage`、Ollama 明記(api_key 空可) |
| Q11 | fetch api.github.com repos/commits | メンテ状況 | repo created 2026-08-03、last push 2026-09-29、author "silwings" 単独、Apache-2.0、conventional commit で詳細な changelog 的 message |
| Q12 | fetch api.github.com contents/docs | 設計 doc 一覧 | architecture.md / coding-agent.md / react-agent-flow.md |
| Q13 | fetch api.github.com search/code anthropic | Anthropic 言及 | 401(認可必須)で失敗 → ソース直接確認に切替 |
| Q14 | fetch molo-openai/Cargo.toml | TLS/dep 構成 | reqwest = "0.13" features json+stream(default features のまま → native-tls 系の可能性)、tokio net/rt/sync/time |
| Q15 | fetch docs/architecture.md | 設計思想 | 中国語 doc。kernel/harness/effect 分離、副作用 governance chain、「Claude Code 的な CLI は molo 上に構築」= cheaper と同じレイヤ。pre-1.0 は minor で破壊的変更許容 |
| Q16 | fetch molo-openai/src/lib.rs + local grep | endpoint 形状・Anthropic 言及 | `POST {base}/chat/completions` のみ(Responses API 非対応)、StructuredOutputMode(Native json_schema strict/JsonObject/Off)、timeout 4種、"anthropic" 文字列なし |
| Q17 | `Anthropic OpenAI SDK compatibility endpoint tool use limitations` | Anthropic を OpenAI 互換で使う場合の制約 | Anthropic 公式: 互換 endpoint ありだが prompt caching 非対応、response_format 無視、tool strict 無視、system msg 結合、主にテスト用途 |
| Q18 | fetch docs.rs RunSummary | フィールド確定 | rounds/tool_calls/usage/usage_omitted/cache_omitted/finish_reason/latency/provider_model — cheaper usage.jsonl 要件と一致 |
| Q19 | fetch docs/react-agent-flow.md | loop/harness 詳細 | kernel=Provider-free、HarnessRuntime が外側 driver、ToolRegistry panic 捕捉、effect observation 境界(中国語) |

## Iteration 3: 同定確度と代替比較

| # | Query / URL | 目的 | 結果 |
|---|------------|------|------|
| Q20 | `"molo" github AI framework OR runtime OR SDK -model-loop -Silwings` | 同名競合の負の証拠 | 競合なし。LLM/agent 領域で "molo" は Silwings-git/molo のみ(0.1 系 molo-kernel/molo-rt は同一プロジェクト旧構造) |
| Q21 | `rig Rust LLM agent framework providers Anthropic github stars` | 成熟代替との比較 | 0xPlaygrounds/rig: 8,806★、MIT、20+ provider(Anthropic/Ollama/OpenRouter 等)、rig-core 0.43.0、2024-06 開始 |

## 備考
- crates.io 本体と GitHub code search API は 403/401 で取得不可。lib.rs・GitHub REST API・raw ファイル・docs.rs で代替し一次情報を確保。
- webfetch 長文はローカル一時ファイルへ保存されるため、grep で内部確認(molo-openai lib.rs の endpoint/anthropic 検索に使用)。
