# persona.md — 依頼者意図モデル

## 依頼者
cheaper プロジェクト(親 Coding Agent から安価モデルへ処理を委譲する Worker Runtime CLI)の開発者/委任元エージェント。

## 目的
- 現行の Pi SDK (`@earendil-works/pi-ai` / `pi-coding-agent` 0.85.1) + TypeScript + Node.js SEA という構成を、「molo」と呼ばれるランタイム/SDK で置き換え可能か評価したい。
- 真の動機は Pi SDK 単体の置換ではなく **TypeScript/Node ランタイム依存そのものの排除とネイティブバイナリ化による配布サイズ削減**。したがって対象は Rust/Go/C++ 等のネイティブコンパイル可能なエージェントライブラリである可能性が高いと依頼書自身が示唆。

## 必須論点
1. 名称「molo」の同定(同名・類似名プロジェクトの切り分け)
2. ライブラリとして埋め込み可能か(フル製品ではなく headless worker runtime)
3. エージェントループ(マルチターン tool-call、turn/tool call 計測)
4. カスタムツール定義(sandboxed read/grep/find/write 相当)
5. 複数 provider(Anthropic / OpenAI / OpenAI-compatible / プラグイン機構)
6. usage 取得、streaming/abort、structured output(repair 含む)
7. 非対話 stdin/stdout JSON envelope CLI としての適合
8. 小さいネイティブバイナリへのコンパイル可能性
9. 成熟度・メンテ状況・リスク

## 鮮度要件
- 最新 version・直近 commit・最新 release 日を明示すること。ソフトウェア調査のため現時点(2026-10 前後と推定)での状況が必須。

## 品質基準
- 一次ソース(公式リポジトリ、docs.rs、crates.io 系)優先。実際に確認したソースのみ引用。推測は推測と明記。
- 要件ごとの Yes/No/Partial 判定に証拠(Source ID)を紐付ける。

## 不明点・残課題
- 「molo」が単語だけの指定のため同定リスクがあった → 初回検索で Silwings-git/molo(Rust agent runtime, "Model Loop")が確度高くヒット。文脈(ネイティブ化可能な LLM エージェント SDK)と完全一致するため主対象とした。
- 依頼者は cheaper 側の全面 Rust 書き換えコスト(count-tokens、APM、usage.jsonl、pricing 等の周辺機能)をどこまで織り込む想定か不明 → final-report で留意事項として言及。
