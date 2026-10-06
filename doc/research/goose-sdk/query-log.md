# Query Log

| # | Query / URL | 目的 | 結果概要 |
|---|-------------|------|----------|
| Q1 | web_search: "block/goose github repository rust agent architecture crates" | リポジトリ構成の一次確認 | aaif-goose/goose へ移管済み、workspace 構成(crates/*)、AGENTS.md、architecture docs を得た |
| Q2 | web_search: "goose crates.io block agent library rust" | crates.io 公開状況 | crates.io の `goose` は別物(Tag1 の負荷試験)。workspace Cargo.toml(Apache-2.0, rust 1.91.1, vendored v8 patch)を確認 |
| Q3 | web_search: "goose agent embed as library rust sdk headless" | 埋め込み可否 | `goose-sdk`(GDK) crate とドキュメント発見。UniFFI surface は provider 層のみ。npm `@aaif/goose-sdk`(ACP client)も存在 |
| Q4 | webfetch crates.io/api/v1/crates/goose-cli, goose-sdk | crates.io 直接確認 | 403 で拒否 → lib.rs / docs.rs で代替 |
| Q5 | web_search: "goose CLI run headless non-interactive output-format json stream documentation" | 非対話実行 | `goose run --output-format json|stream-json`, `--no-session`, `--max-turns`, `--provider/--model` を確認(cli.rs ソース含む) |
| Q6 | webfetch lib.rs/crates/goose-cli | Block 製 goose-cli の crates.io 公開確認 | crates.io の `goose-cli` は第三者の別ツール(Michael Mendy 製)。Block/AAIF の CLI は crates.io 未公開(名前衝突) |
| Q7 | webfetch lib.rs/crates/goose-sdk | GDK 詳細 | goose-sdk 0.1.0-alpha.11、Apache-2.0。UniFFI = provider 構築/stream/complete + ObservabilityHook(usage, durationMs)。依存に goose-providers, goose-context-management |
| Q8 | webfetch lib.rs/crates/goose-agent | agent loop のライブラリ性 | **決定的証拠**。StateMachine/Operation/ToolOperation(SyncTool/AsyncTool で Rust ツール直接登録)/Emitter(Usage, MessageUsage イベント)/CancellationToken/SessionLoader trait。参照実装は goose::agents::state_machine |
| Q9 | webfetch lib.rs/crates/goose-providers | provider 範囲 | native: anthropic/openai/openai_compatible/google/databricks(+oauth)/azure_foundry/snowflake/ollama/local_inference。declarative JSON で ~30+ + `load_custom_providers(dir)`。trait は goose-provider-types |
| Q10 | webfetch github.com/aaif-goose/goose/releases | バイナリサイズ実測 | v1.53.0: CLI tar.bz2 74-99MB(圧縮)、RPM(desktop) 159-173MB。"Lean ACP-only Goose binary" #11961、GDK wasm32 対応が進行中 |
| Q11 | web_search: structured output / final_output_tool | structured output | recipe `response.json_schema` → `final_output` tool、検証失敗時は再試行させる repair ループ。toolshim(非 tool-call モデル向け shim)も存在 |
| Q12 | webfetch docs.rs search "goose" | AAIF 公開 crate の網羅確認 | goose-agent/-providers/-provider-types/-context-management/-sdk/-sdk-types/-local-inference/-download-manager/-mlx-lm*, v8-goose を確認。`goose` 本体は未公開 |
| Q13 | web_search: issue "use goose as a library" / embed | 批判的・歴史的経緯 | Issue #1726(当時は消極的→現在 GDK で解消方向)。PR #10600 は外部プロジェクトが goose をライブラリ利用している実例。Discussion #7697(単一 ACP バイナリ化計画) |
| Q14 | web_search: token usage / accumulated_usage | usage 取得 | session/mod.rs: json 出力の metadata.total_tokens(累積,#8910 で修正)、stream-json の Complete イベント、Session struct(input/output/accumulated_*)、SQLite sessions.db |
| Q15 | web_search: binary size / v8 | サイズ要因 | Issue #3914「release binary is huge」(~86Mi __text)。v8-goose(V8 fork)が goose crate 依存に存在。lancedb 削除など削減努力中 |
