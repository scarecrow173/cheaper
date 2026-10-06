# Audit Checklist

## カバレッジ
- [x] T1 構成・ライブラリ埋め込み可否 — S1,S2,S3,S5,S6,S12,S13
- [x] T2 provider 範囲・追加機構 — S7,S18(native 10 種 + declarative ~30+ + Provider trait 自前実装)
- [x] T3 agent loop / tool / usage / structured output — S6,S11,S15
- [x] T4 配布サイズ — S8,S16,S17
- [x] T5 ヘッドレス実行 — S9,S10,S20,S14
- [x] T6 ライセンス・メンテ・安定性 — S1(Apache-2.0),S5-S7(alpha 版),S8(頻繁リリース),S12,S14
- [x] T7 適合表 — final-report に記載

## 検索ルール遵守
- [x] 一次ソース検索: GitHub repo/docs.rs/lib.rs/公式 docs/リリース(Q1-Q10)
- [x] 批判的検索: Issue #1726(library 公開の議論), #3914(バイナリサイズ問題)(Q13,Q15)
- [x] 多様なクエリ: 直接キーワード、registry、issue、docs、release notes
- [x] ≥2 イテレーション: 3 イテレーション実施
- [x] 未確認ソースの引用なし・推測は明記

## 矛盾・注意点
- crates.io `goose`/`goose-cli` の名前衝突(一次確認済み、S4,S6 経由)
- docs の `cargo install goose-cli` は誤解を招く → 要注記
- 日付の鮮度: lib.rs/docs.rs の日付は表示系。リリース頻度は GitHub Releases(v1.45→v1.53 が短期間に連続)で裏付け
- 「Goose GDK」表記: 依頼者の「GDK」は正確 — goose-sdk crate の正式名称が "goose Development Kit (GDK)"

## 引用監査
- 全 Source ID は実際に webfetch/websearch で内容を確認したもののみ
- 数値(バイナリサイズ、SLoC、リリース version)はソース表示値を転記
