# Persona: 依頼者意図モデル

## 依頼者
cheaper プロジェクトの親エージェント(Devin)。TypeScript pnpm モノレポの Worker Runtime CLI を開発中。

## 目的
- `packages/runtime-pi` が依存する Pi SDK(`@earendil-works/pi-ai` / `pi-coding-agent` 0.85.1)と、TypeScript/Node.js ランタイム依存そのものを、Block 社の Rust 製エージェント Goose(およびその SDK)で代替できるか評価したい。
- 動機: Node.js SEA 配布の実行ファイルサイズ削減、ネイティブバイナリ化。

## 必須論点(決定的)
- **ライブラリとして埋め込めるか**(crates.io 公開、内部 crate を依存として使えるか)vs フルアプリの subprocess 呼び出しに留まるか。
- カスタムツールを Rust コードとして直接登録できるか(MCP server 経由のみか)。
- provider 範囲: Anthropic / OpenAI / OpenAI-compatible(Ollama) / plugin 追加可否。
- usage(token)・turn/tool call 計測・streaming/abort・structured output。
- 非対話 stdin/stdout JSON envelope CLI として使えるか。
- バイナリサイズ実測。

## 鮮度要件
- Goose は活発に開発中(2024-2025)。最新リリースと main ブランチ状態を確認。API 安定性は特に注意(内部 API の breaking change リスク)。

## 品質基準
- 一次ソース(GitHub repo、公式 docs、crates.io、releases)優先。実際に確認したソースのみ引用。
- Yes/No/Partial + 証拠 の適合表を要求。

## 不明点・注意
- 「Goose GDK」はおそらく Goose SDK の誤記。Goose に公式「SDK」があるかも確認対象(goose-rs SDK? OpenAPI goose-server?)。
