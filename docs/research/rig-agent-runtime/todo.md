# Task Decomposition — Rig as Pi SDK replacement

| ID | Status | Priority | Domain | Task | Evidence target | Dependency | Provenance |
|----|--------|----------|--------|------|-----------------|------------|------------|
| T1 | pending | High | Identity | Rig の正体確認: 言語・ライセンス・メンテ状況(star/commit/release/version/MSRV) | GitHub repo, crates.io, CHANGELOG | - | 依頼要件1 |
| T2 | pending | High | Providers | provider 対応範囲(Anthropic/OpenAI/OpenAI-compat/Ollama/Gemini/Bedrock)とカスタム provider 可否 | docs.rs, repo docs, examples | T1 | 依頼要件2 |
| T3 | pending | High | Agent loop | agent loop/tool calling API: マルチターン, max_turns, Tool trait, streaming, usage, structured output, abort | docs.rs API, examples, guide | T1 | 依頼要件3 |
| T4 | pending | Medium | Binary size | Rust 単一バイナリ化の見通し: 必須依存(tokio等), サイズ感, feature flag | Cargo.toml, deps tree, 議論/issue | T1 | 依頼要件4 |
| T5 | pending | Medium | Maturity | WASM/組み込み実例, SDK 成熟度, 採用実績 | README, awesome lists, blog, crates.io downloads | T1 | 依頼要件5 |
| T6 | pending | High | Risk | API 安定性(0.x), breaking changes 頻度, ドキュメント品質, 批判/不満 | CHANGELOG, issues, Reddit/blog | T1-T3 | 依頼要件6 |
| T7 | pending | High | Fit | cheaper 要件適合度マトリクス評価 + 最終報告 | 全証拠 | T1-T6 | 依頼要件7 |
