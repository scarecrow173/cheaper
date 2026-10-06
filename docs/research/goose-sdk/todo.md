# Research Tasks: Goose SDK as Pi SDK replacement

| ID | Status | Priority | Domain | Task | Evidence target | Dependency | Provenance |
|----|--------|----------|--------|------|-----------------|------------|------------|
| T1 | completed | high | Architecture | Goose の構成(crates 分割)とライブラリ埋め込み可否を確認 | GitHub block/goose repo構造、crates.io、Cargo.toml | - | 依頓要件#1 |
| T2 | completed | high | Providers | provider 対応範囲と追加機構 | 公式docs providers ページ、ソースの providers モジュール | T1 | 依頓要件#2 |
| T3 | completed | high | Agent loop | agent loop / tool calling 形態(MCP vs 直接登録)、usage/metrics 取得 | ソース(agents/)、docs、issue | T1 | 依頓要件#3 |
| T4 | completed | high | Distribution | バイナリサイズ・GUI 分離・ビルド重量 | GitHub Releases assets、Cargo deps | T1 | 依頓要件#4 |
| T5 | completed | medium | Headless | 非対話実行(goose run, JSON出力, subprocess適性) | docs CLIリファレンス | - | 依頓要件#5 |
| T6 | completed | medium | Governance | ライセンス・メンテ状況・API安定性 | LICENSE、リリース頻度、issue「library/embedding」系 | T1 | 依頓要件#6 |
| T7 | completed | high | Synthesis | cheaper 要件適合表の作成と結論 | T1-T6 全証拠 | T1-T6 | 依頓要件#7 |
