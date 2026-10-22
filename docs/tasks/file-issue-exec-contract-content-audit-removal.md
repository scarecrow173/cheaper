---
id: "TASK-034KvxjG6fUOLjtn1cOFTF"
type: "task"
status: "todo"
title: "起票 issue 1: exec-contract cleanup — 内容審査 7 項目の mandatory 層除去"
created: "2026-10-22"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/151"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/file-permission-model-redesign-implementation-issues.md"
  implemented-by: []
  depends-on:
    - "docs/plans/file-permission-model-redesign-implementation-issues.md"
  blocks:
    - "docs/tasks/file-issue-opt-in-exec-deny-patterns.md"
    - "docs/tasks/file-issue-unified-exec-grant-model.md"
    - "docs/tasks/file-issue-route-grant-set-and-record-split.md"
    - "docs/tasks/file-issue-trusted-flag-issuer-redesign.md"
    - "docs/tasks/file-issue-docs-honesty-and-migration-guide.md"
    - "docs/tasks/record-issue-urls-and-verify-investigation-criteria.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/file-permission-model-redesign-implementation-issues.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/permission-model-redesign-issue-decomposition.md"
  references:
    - "docs/designs/permission-model-responsibility-redesign.md"
    - "docs/specs/permission-model-responsibility-redesign.md"
    - "docs/adr/enforce-declared-contracts-over-content-audit.md"
  defers: []
  deferred-by: []
---

# 起票 issue 1: exec-contract cleanup — 内容審査 7 項目の mandatory 層除去

## Work

PLAN-034KvwX4x7oaZlaECqhHc1 の T1。DESIGN-034KuDkWOmNXiBddEQ4XWK
「§実装 issue 分解」の issue 1 を `gh` CLI で
`scarecrow173/cheaper` へ起票し、発行 URL を本 task の
「起票結果」欄へ記録する。

issue title / body は下記を**そのまま**使う — design 決定を
再解釈・再導出しない。本 issue は deps なしの起点であり、
placeholder 置換は不要。

### 手順

1. `gh issue list --repo scarecrow173/cheaper --search "権限モデル再設計 1/7" --state all --json number,title,url`
   で重複確認。既に存在すれば作成を skip し、その URL を
   起票結果へ記録して完了とする。
2. 下記「issue body」の fenced block 内容を temp file へそのまま
   書き出す(例: `$(mktemp --suffix=.md)`)。
3. `gh issue create --repo scarecrow173/cheaper --title "権限モデル再設計 1/7: exec-contract cleanup — 内容審査 7 項目を mandatory 層から除去" --body-file <temp file>`
   で起票し、出力された issue URL を控える。
4. 発行 URL・番号を本 task の「起票結果」欄に記録する。

### issue title

`権限モデル再設計 1/7: exec-contract cleanup — 内容審査 7 項目を mandatory 層から除去`

### issue body

````markdown
## 背景

親 issue #151(権限・Runtime Policy・Tool 実行制限の責務ベース
再設計)の実装分解 1/7。
`docs/designs/permission-model-responsibility-redesign.md`「§実装
issue 分解」に従い、「宣言された exec の内容を審査して reject」
する制限(content-audit クラス)を mandatory 層から除去する。
cheaper の責務は宣言された制約の強制と実行契約の正しさに限る
(ADR-034KuqZelggTIb8Z8K9QB8 Option B — ratification 済み)。

## Scope

以下 7 項目の検査分岐と呼出しを `check_command_execution`・pin・
executable resolve・task resolve から除去する。

| # | 制限 | 出所 | 除去範囲 |
|---|------|------|----------|
| A8 | grant executable の shell/shim 判定 | `policy/command.rs:372` + `process.rs:117-148` | 検査分岐と呼出し |
| A10 | `argv_shell_re` メタ文字 reject | `process.rs:134-137,278`、`command/mod.rs:467-475`、`command.rs:249` | `command.rs:249` は `ArgPattern::compile` が宣言済み argv pattern へ適用する同種審査として同時除去 |
| B3 | `native()` magic-byte 検査 | `executable.rs:74-117` | 検査分岐と呼出し |
| B4 | workspace-resident executable reject | `command/mod.rs:214-244,273-278` | pin 時 2 段検査 + `verify_pinned_executable` の workspace 再検査。**pin-equality 検査(261-266)は B2 の TOCTOU 対策として残す** |
| B5 | bare `node` reject | `executable.rs:163-173` | 検査分岐と呼出し |
| B6 | `.cmd/.bat/.ps1/.sh` 拡張子 reject | `process.rs:127-130` | 検査分岐と呼出し |
| F3 | process task の native/shim 検査 | `process.rs:117-148` + `executable.rs` | B3-B6 と同種の内容審査として同時除去 |

維持するもの: `tokenize_shell`(D1 — hook 文字列の解析
correctness、拒否ではなく viability 分類)、pin + spawn 前再検証
(B1/B2)、pin-equality 検査、全 lifecycle/fs/writeScope/manifest
契約。

churn 対象(fixture/test): golden fixture
`crates/cheaper-core/tests/golden/fixtures/policy/process.json` の
shell/script/metachar 系 case、
`crates/cheaper-tools/tests/fixtures/process.json`、
`run_dev_seam.rs` の deny-list case
(`run_fake_stream_grant_deny_listed_executable_rejected` 等)。

観測差分: `\0` を含む argv は現行 `POLICY_VIOLATION` で事前拒否
されるが、A10 除去後は spawn 層の OS/NUL エラーへ縮退する(拒否
自体は維持、error 分類が変わる軽微な差分)。

## Acceptance Criteria

- [ ] A8/A10/B3/B4/B5/B6/F3 の検査分岐と呼出しが mandatory 層から
  除去されている(discovery 分類表との照合)
- [ ] `python`/`ssh`/`tar`/`.sh`/workspace binary を宣言した grant
  が(opt-in deny 未指定で)拒否されない統合テスト
- [ ] pin-equality(B2)・`tokenize_shell` の解析用途(D1)・
  pin + spawn 前再検証(B1)が維持されている
- [ ] 既存契約テスト(pin・env_clear・stdin null・lifecycle・fs
  boundary・writeScope)が回帰なく pass
- [ ] 上記 churn 対象の golden fixture・dev-seam case が更新または
  削除されている

## Dependencies

- なし(起点)。ADR-034KuqZelggTIb8Z8K9QB8 は accepted 済みのため
  着手前提を満たす。

## 参照

- 親 issue: https://github.com/scarecrow173/cheaper/issues/151
- design: `docs/designs/permission-model-responsibility-redesign.md`
  (DESIGN-034KuDkWOmNXiBddEQ4XWK)
- spec: `docs/specs/permission-model-responsibility-redesign.md`
  (SPEC-034KuqPkTK2b4mHsykH9A5)
- ADR: `docs/adr/enforce-declared-contracts-over-content-audit.md`
  (ADR-034KuqZelggTIb8Z8K9QB8)
- discovery:
  `docs/discovery/permission-model-responsibility-redesign.md`
  (DISC-034KuleA9g82Ir4xWfmTYq)
````

### 起票結果

- URL: (未起票 — T8 完了時点で記録済みであること)
- 番号: (未起票)

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/file-permission-model-redesign-implementation-issues.md`
- `docs/designs/permission-model-responsibility-redesign.md`
- `docs/specs/permission-model-responsibility-redesign.md`
- `docs/adr/enforce-declared-contracts-over-content-audit.md`

## Dependencies

- なし(起点 task)。後続 T2/T3/T4/T6/T7 が本 issue の番号を deps
  記述に引用するため、起票結果の記録が必須。

## Affected Files

- (GitHub) `scarecrow173/cheaper` の新規 issue ×1
- `docs/tasks/file-issue-exec-contract-content-audit-removal.md`
  (起票結果欄の記録)

## Done When

- [ ] issue が `scarecrow173/cheaper` に起票済み(または重複確認で
      既存を検出し skip)
- [ ] issue body が `## Scope`・`## Acceptance Criteria`・
      `## Dependencies`・`## 参照` の 4 節を持つ
- [ ] 起票 URL・番号が本 task の起票結果欄に記録されている

## Verification

- [ ] `gh issue list --repo scarecrow173/cheaper --search "権限モデル再設計 1/7" --state all --json number,title,url`
      がちょうど 1 件を返す
- [ ] `gh issue list --repo scarecrow173/cheaper --search "権限モデル再設計 1/7" --state all --json body --jq ".[0].body"`
      の出力が `## Scope`・`## Acceptance Criteria`・
      `## Dependencies`・`## 参照` を全て含む
