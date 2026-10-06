# 依頼者意図モデル — Rig 調査

## 目的
cheaper(TypeScript pnpm モノレpo、`@earendil-works/pi-ai` / `@earendil-works/pi-coding-agent` 0.85.1 依存、Node.js SEA 配布)の worker runtime 層について、Pi SDK + TypeScript + Node.js 依存自体を排除しネイティブバイナリ化する代替として、Rust 製 LLM agent framework「Rig」(0xPlaygrounds/rig) が要件を満たすかを証跡ベースで評価する。

## 必須論点
- Rig の正体・ライセンス・メンテナンス状況(star, commit activity, release 頻度, 最新 version, MSRV)
- provider 対応: Anthropic / OpenAI / OpenAI-compatible / Ollama / Gemini / Bedrock / カスタム provider 実装可否
- agent loop / tool calling API: マルチターン, max_turns, カスタム tool trait, streaming, usage 取得, structured output, abort
- ネイティブバイナリサイズの見通し(単一バイナリ可否、依存の重さ)
- WASM/組み込み利用実例、SDK 成熟度、採用実績
- リスク: API 安定性, breaking change 頻度, ドキュメント品質
- cheaper 要件適合度(ヘッドレス埋め込み、tool-call ループ計測、非対話 stdin/stdout JSON envelope 等)

## 鮮度要件
- 現時点(2025-10 前後)の最新 version と最近のリリース/commit 状況を確認。古い(1年以上前)記事は参考程度に留める。

## 品質基準
- 一次ソース(GitHub, docs.rs, crates.io, 公式 docs, CHANGELOG)優先
- 少なくとも1回の批判的検索(issue/不満/移行記事)
- 実際に確認していないソースは引用しない。推測は推測と明記
- Confidence: High(primary or 複数独立ソース)/Medium/Low

## 不明点・未確定事項
- Rig の structured output の JSON schema repair 機能の有無
- Ollama を OpenAI-compatible endpoint として使う経路があるか、専用 provider か
- abort/cancellation の API 形態
- usage token 取得の確実性(全 provider で usage が返るか)
- 最小バイナリサイズの実測データ(公開情報が無ければ依存構成からの推測)
