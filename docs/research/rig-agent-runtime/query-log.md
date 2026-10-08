# Query Log — Rig 調査

| # | 種別 | Query / URL | 理由 |
|---|------|-------------|------|
| Q1 | web_search | `0xPlaygrounds rig Rust LLM agent framework GitHub` | Rig の正体・リポジトリ特定(T1) |
| Q2 | web_search | `rig-core crates.io Rust LLM library version` | 最新 version・リリース頻度・依存(T1/T4) |
| F1 | webfetch | https://github.com/0xPlaygrounds/rig | README 全文: license, stars, providers 表, 採用実績, WASM(T1/T5) |
| F2 | webfetch | https://docs.rs/rig-core/latest/rig_core/ | rig-core API 構造・依存(T1/T3/T4) |
| F3 | webfetch | https://docs.rs/rig-core/latest/rig_core/providers/index.html | provider 一覧(T2) |
| F4 | webfetch | https://docs.rs/rig-agent/latest/rig_agent/ | agent runtime API(T3) |
| F5 | webfetch | https://docs.rs/rig-agent/latest/rig_agent/agent/index.html | agent module: hook/runner/response 型(T3) |
| F6 | webfetch | https://docs.rs/rig-core/latest/rig_core/completion/index.html | completion/usage 型(T3) |
| F7 | webfetch | https://docs.rs/rig-agent/latest/rig_agent/agent/struct.AgentBuilder.html | builder API: max_turns/output_schema/tool(T3) |
| F8 | webfetch | https://docs.rs/rig-agent/latest/rig_agent/run/response/struct.PromptResponse.html | usage/completion_calls 計測(T3) |
| F9 | webfetch | https://docs.rs/rig-core/latest/rig_core/tool/index.html | Tool/PortableTool trait(T3) |
| F10 | webfetch | https://docs.rs/rig-core/latest/rig_core/providers/ollama/index.html | Ollama provider 詳細(T2) |
