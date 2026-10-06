# Audit Checklist — Rig 調査

## カバレッジ
- [x] T1 正体・言語・ライセンス・メンテ状況: MIT / Rust / 8.8k stars / 0.43.0 / 2-4週間リリース / MSRV 1.95 — primary sources
- [x] T2 provider 対応範囲 + カスタム provider 可否: providers module 全列挙、dialect、Wire/Decoder 設計、docs.guide — primary
- [x] T3 agent loop / tool API: AgentBuilder, PromptResponse, Usage, Tool traits, hooks, streaming, cancel, extractor — primary
- [x] T4 バイナリサイズ見通し: 依存構成(Cargo.toml)+ release profile。実測値なし → 推定と明記
- [x] T5 WASM/組み込み実例・成熟度・採用実績: README 採用一覧, VT Code 詳細, WASM matrix — primary + high
- [x] T6 リスク: README warning, issue #628, MIGRATING.md, FAUN/dev.co review, docs lag 実例 — primary + medium
- [x] T7 cheaper 要件適合度マトリクス: final-report Analysis に集約

## 矛盾チェック
- docs.rig.rs の custom provider ガイド(CompletionModel trait 前提) vs 0.43 API(CompletionModel 不存在, Wire/Decoder 設計): 矛盾確認済み → 「docs が古い」として報告に明記。docs.rs API を正とする。
- README「20+ providers」vs providers module 実数 26 + companion crates: README は概数、矛盾なし。
- crates.io dependents 数値(検索時点 205 vs 211): 取得タイミング差、~210 と記載。
- S19 issue #628 の maintainer 回答本文はフェッチで途切れた → 「closed as not planned」の事実のみ引用。

## 鮮度チェック
- [x] 最新 version 0.43.0 (2026-09-30) を基準に評価
- [x] API 参照は全て 0.43.0 / latest の docs.rs
- [x] CHANGELOG/MIGRATING は 0.43 まで網羅
- [x] 批判ソース: issue #628(2025-07)は1年以上前だが現行方針と整合(README warning + 0.43 changelog が裏付け)

## 引用監査
- [x] 全 claim は実際に fetch/search したソースに限定
- [x] 実測していない数値(バイナリサイズ)は「推測」と明記
- [x] 旧版 API(docs.rig.rs ガイド)は「現行版と不一致の可能性」を明記
- [x] Confidence: primary=High、複数独立=High、単一中次=Medium、推測=Low

## 未解決(許容)
- 実測バイナリサイズ → PoC で実測推奨(推定根拠は依存構成)
- devin provider 相当の実装経路 → 設計上の選択肢提示に留める
