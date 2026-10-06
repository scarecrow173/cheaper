# todo.md — molo agent runtime 調査タスク分解

| ID | Status | Priority | Domain | Task | Evidence target | Dependency | Provenance |
|----|--------|----------|--------|------|-----------------|------------|------------|
| T1 | done | high | identification | 「molo」の同定(LLM/agent系SDKか、同名候補の切り分け) | crates.io / GitHub / docs.rs の一致確認 | — | 依頼書「名称の同定から開始」 |
| T2 | done | high | profile | 正体・言語・ライセンス・メンテ状況(star/commit/release/version) | GitHub API、lib.rs バージョン履歴 | T1 | 依頼書「必ず確認すること 1」 |
| T3 | done | high | capability | provider 対応範囲とカスタム provider 可否(Anthropic/OpenAI/Ollama/plugin) | Provider trait 定義、molo-openai ソース、Anthropic 互換 endpoint 公式 doc | T1 | 依頼書「必ず確認すること 2」 |
| T4 | done | high | capability | agent loop/tool calling API(マルチターン、カスタムツール、streaming、usage、abort、structured output) | docs.rs API、examples/usage.rs、設計 doc | T1 | 依頼書「必ず確認すること 3」 |
| T5 | done | medium | packaging | ネイティブバイナリ化見通し(ランタイム要否、依存の重さ、TLS backend) | Cargo.toml 依存、crate サイズ | T1 | 依頼書「必ず確認すること 4」 |
| T6 | done | medium | maturity | SDK 成熟度・採用実績・ドキュメント品質・リスク | downloads、dependents、設計 doc 言語、破壊的変更履歴 | T2 | 依頼書「必ず確認すること 5,6」 |
| T7 | done | high | evaluation | cheaper 要件適合表 + 代替候補(rig 等)との位置付け | 要件チェックリスト vs 収集証拠 | T2-T6 | 依頼書「必ず確認すること 7」 |

全タスク完了。監査(audit.md)で引用済み・鮮度・ギャップを確認済み。
