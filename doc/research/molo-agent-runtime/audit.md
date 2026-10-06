# audit.md — 監査チェックリスト

## カバレッジ(依頼の必須確認 1-7)
- [x] 1. 正体・言語・ライセンス・メンテ状況 → S1,S2,S3,S5,S6(star/commit/release/version 全確認)
- [x] 2. provider 対応範囲 + カスタム provider 可否 → S7,S9,S10,S13
- [x] 3. agent loop/tool API(マルチターン・カスタムツール・streaming・usage) → S4,S7,S8,S12,S16
- [x] 4. バイナリサイズ見通し → S5,S9,S10(dep 構成ベースの推定、実測なしと明記)
- [x] 5. 成熟度・採用実績・ドキュメント品質 → S5,S11,S12,S15
- [x] 6. リスク(API 安定性・保守・制約) → S3,S5,S10,S11,S13
- [x] 7. cheaper 要件適合(項目別 Yes/No/Partial+証拠) → final-report Evidence Matrix/Analysis
- [x] 名称同定 + 類似候補の扱い → Q1,Q2,Q20 + final-report Key Findings

## 一次ソース比率
- 引用 16 ソース中 primary = 13(公式 repo/API/docs.rs/raw source/Anthropic 公式)、high = 2(lib.rs、crates.io スニペット)、medium 以下 0。
- 未検証引用なし。crates.io 本体 403・GitHub code search 401 は代替ソースで補完し、query-log に記録。

## 鮮度
- [x] 最新 version 0.6.0(2026-09-29)を docs.rs で確認(S6)。README master 記述と一致。
- [x] 最終 commit/push 2026-09-29 を GitHub API で確認(S2,S3)。
- [x] 検索結果に 0.3.0/0.4.0 の古い情報が混在したが、docs.rs/lib.rs で最新を確定し矛盾なしと判定。

## 矛盾チェック
- crates.io スニペットの molo-agent「0.4.0」依存表示 vs docs.rs 0.3.0 → 版ずれは時期差であり、最新系 0.6.0 で統一確認済み。矛盾として残らない。
- GitHub 検出 license = Apache-2.0 vs crates license = MIT OR Apache-2.0 → 両記(デュアルライセンス)、矛盾なし。

## 推測の明示
- バイナリサイズ「4-8MB 程度」は dep 構成からの推定(Low-Medium confidence)。
- Anthropic native Provider の実装工数は推定(Medium-Low)。
- reqwest TLS backend(native-tls)は Cargo.toml の default features からの推定。

## 未検証・残ギャップ
- molo 実ビルドのバイナリサイズ実測、静的リンク可否(musl)。
- Provider trait で Anthropic SSE/tool_use の写像詳細(実装スパイク未実施)。
- cheaper 全面 Rust 化の総工数(スコープ外)。
- rig の詳細 API 適合(比較ポインタのみ)。
