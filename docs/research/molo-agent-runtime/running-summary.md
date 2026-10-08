# running-summary.md — 反復サマリ

## Iteration 1 後(同定フェーズ)
**確定**
- 「molo」= `Silwings-git/molo`、"**Mo**del **Lo**op"。Rust 製 embeddable agent runtime & harness framework。crates.io `molo`、最新 0.6.0(2026-09-29)。同一プロジェクトの旧構造 molo-kernel/molo-rt(0.1.x)は yank/再編済み。
- 依頼文脈(ネイティブバイナリ化可能な LLM エージェント SDK)と完全一致。同名競合なし。
- ライセンス MIT OR Apache-2.0(GitHub 検出は Apache-2.0)。MSRV 1.88、edition 2024。
- Facade `molo` + workspace crates(molo-core/agent/harness/coding/mcp/skills/openai/macros + publish=false の molo-cli)。

**残ギャップ**: version 履歴詳細、provider 範囲、RunSummary 項目、バイナリサイズ根拠。

## Iteration 2 後(API・メタ深掘り)
**確定**
- メンテ状況: repo created 2026-08-03、最終 push 2026-09-29。単一作者 "silwings"。4★/0 fork/40 DL 月。9 release(4 breaking)を2ヶ月で発行 — 非常に若く高速に破壊的変更している。
- Provider: built-in は OpenAiProvider のみ。`POST {base_url}/chat/completions` 固定 → OpenAI/DeepSeek/Zhipu/Moonshot/Ollama 等 OpenAI 互換は網羅。Responses API 非対応。"anthropic" の言及・実装なし。
- `Provider` trait は公開かつ外部実装可能(必須2メソッド) → 独自 provider(Anthropic native, devin delegation 等)は自前実装すれば可能。
- RunSummary: rounds/tool_calls/usage(prompt/completion/total/cached)/usage_omitted/cache_omitted/finish_reason/latency/provider_model — cheaper の usage.jsonl 記録要件とほぼ1:1。
- streaming: run_stream→MessageChunk(Delta/ToolCall/ToolResult/Done/Cancelled)、CancellationToken による run 単位の協調 cancel。
- structured output: run_typed/run_typed_request + jsonschema validation、失敗は model へ error feedback で retry(=repair)。endpoint 側は response_format 3 mode(json_schema strict 自動/JsonObject/Off)。
- sandboxed tools: Tool trait 直実装 or #[molo::tool]; molo-coding に WorkspacePath(絶対パス/traversal 拒否)/LocalWorkspace/CommandExecutor(timeout/output limit)/RipgrepSearcher/GitInspector/ApplyPatch。effect/harness で policy/approval/audit/redaction を挟む設計は cheaper の sandboxed tool モデルと相性が良い。

**新規判明リスク**
- Anthropic: 公式 OpenAI 互換 endpoint 経由なら動くが prompt caching 非対応・response_format 無視・tool strict 無視で「テスト用途」扱い。cheaper の cached_tokens 計測・コスト動機と相性が悪い。native 対応は自前 Provider 実装が必要。
- 設計 doc(architecture/react-agent-flow)は中国語のみ。README・rustdoc は英語。
- reqwest が default features(json+stream 追加のみ)のため TLS は native-tls 系の可能性。musl 静的ビルドでは OpenSSL 取り回しが課題になり得る(要検証)。

## Iteration 3 後(同定確度・代替比較・監査)
**確定**
- 同名競合の負の証拠を記録: LLM/agent 領域で "molo" は Silwings-git/molo のみ。
- 代替候補 rig(0xPlaygrounds): 8.8k★、20+ provider(Anthropic/Ollama 含む)、MIT、2024 開始、rig-core 0.43。molo との成熟度差は約3桁の採用規模。molo の差別化点は governed side-effect harness(kernel/harness/effect 分離)と coding workload SDK。

**矛盾**: なし(crates/docs/repo の情報は整合。検索結果の 0.3/0.4 表示はインデックス遅延で 0.6.0 が最新)。

**残ギャップ(確定できなかったもの)**
- 実バイナリサイズの実測値(公開データなし → 依存構成からの推定のみ)。
- Anthropic native Provider 実装の工数(概算のみ)。
- molo の今後のメンテ継続性(単一作者、継続意思は不明)。
- cheaper 全面 Rust 化に伴う周辺機能(count-tokens tokenizer、APM hooks、pricing catalog、SEA 代替の配布手段)の移行評価 — 本調査スコープ外として記録。
