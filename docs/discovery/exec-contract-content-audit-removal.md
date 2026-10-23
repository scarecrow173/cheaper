---
id: "DISC-JLHFz6x4ZuhzYZrpfrG2QV"
type: "discovery"
status: "capturing"
title: "exec-contract cleanup — 内容審査 7 項目を mandatory 層から除去（#163）"
created: "2026-10-23"
updated: "2026-10-23"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/163"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "permission-model-responsibility-redesign.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../designs/permission-model-responsibility-redesign.md"
    - "../specs/permission-model-responsibility-redesign.md"
    - "../adr/enforce-declared-contracts-over-content-audit.md"
  defers: []
  deferred-by: []
---

# exec-contract cleanup — 内容審査 7 項目を mandatory 層から除去

## Seed（briefing 前の最小記録）

Issue #163（#151 実装分解 1/7・起点 issue）。「宣言された exec の
内容を審査して reject」する制限（content-audit クラス）を
mandatory 層から除去する。設計判断は親 chain（#151）で確定済み
（ADR-034KuqZelggTIb8Z8K9QB8 accepted・design approved・
review-protocol 通過）。本 chain の作業は除去実装の正確な scope
固定と回帰検証。

### 除去対象（issue 本文より）

- A8: grant executable shell/shim 判定（`policy/command.rs:372` +
  `process.rs:117-148`）
- A10: `argv_shell_re` メタ文字 reject（`process.rs:134-137,278`・
  `command/mod.rs:467-475`・`command.rs:249` の ArgPattern::compile
  宣言 pattern 審査）
- B3: `native()` magic-byte 検査（`executable.rs:74-117`）
- B4: workspace-resident executable reject（`command/mod.rs:214-244,
  273-278`。pin-equality 261-266 = B2 は残す）
- B5: bare `node` reject（`executable.rs:163-173`）
- B6: `.cmd/.bat/.ps1/.sh` 拡張子 reject（`process.rs:127-130`）
- F3: process task の native/shim 検査（`process.rs:117-148`）

### 維持するもの

`tokenize_shell`（D1 解析 correctness）、pin + spawn 前再検証
（B1）、pin-equality（B2）、lifecycle/fs/writeScope/manifest 全契約。

### 観測差分

`\0` argv が `POLICY_VIOLATION` 事前拒否 → spawn 層 OS/NUL エラーへ
縮退（拒否自体は維持、error 分類のみ変化）。

### churn 対象

golden fixture `crates/cheaper-core/tests/golden/fixtures/policy/
process.json`、`crates/cheaper-tools/tests/fixtures/process.json`、
`run_dev_seam.rs` deny-list case。

詳細は briefing で確定する。
