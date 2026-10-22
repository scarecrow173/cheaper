---
id: "TASK-034KvxFX6oCd4TlEowM1kd"
type: "task"
status: "todo"
title: "起票 issue 6: trusted route flag 発行主体の再設計 (D4)"
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
    - "docs/tasks/file-issue-exec-contract-content-audit-removal.md"
    - "docs/tasks/file-issue-route-grant-set-and-record-split.md"
    - "docs/tasks/file-issue-substitute-exec-route-mode.md"
  blocks:
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

# 起票 issue 6: trusted route flag 発行主体の再設計 (D4)

## Work

PLAN-034KvwX4x7oaZlaECqhHc1 の T6。DESIGN-034KuDkWOmNXiBddEQ4XWK
「§実装 issue 分解」の issue 6 を `gh` CLI で
`scarecrow173/cheaper` へ起票し、発行 URL を本 task の
「起票結果」欄へ記録する。

issue title / body は下記を**そのまま**使う — design 決定を
再解釈・再導出しない。body 内の `#<ISSUE-1>`・`#<ISSUE-4>`・
`#<ISSUE-5>` placeholder は、T1・T4・T5 task doc の起票結果に
記録された実番号で置換してから起票する。

### 手順

1. `gh issue list --repo scarecrow173/cheaper --search "権限モデル再設計 6/7" --state all --json number,title,url`
   で重複確認。既に存在すれば作成を skip し、その URL を
   起票結果へ記録して完了とする。
2. T1・T4・T5 task doc の起票結果から issue 1・4・5 の番号を
   取得し、下記 body 内の `#<ISSUE-1>`・`#<ISSUE-4>`・
   `#<ISSUE-5>` を実番号で置換する。
3. 置換後の body を temp file へ書き出し、
   `gh issue create --repo scarecrow173/cheaper --title "権限モデル再設計 6/7: trusted route flag 発行主体の再設計 (D4)" --body-file <temp file>`
   で起票する。
4. 発行 URL・番号を本 task の「起票結果」欄に記録する。

### issue title

`権限モデル再設計 6/7: trusted route flag 発行主体の再設計 (D4)`

### issue body

````markdown
## 背景

親 issue #151 の実装分解 6/7。`trusted:true` を project 自身が
宣言できる現状の再設計(D4)— trust 判定主体を operator/user
層へ移す方式の決定と実装。route の trust model は grant 参照の
scope(誰の宣言を route が使えるか — grant-set #<ISSUE-4>・
substitute #<ISSUE-5> の形)と直に交差するため、それらの形が
見えてから決める。

## Scope

- `trusted` route flag の発行主体を operator/user 層へ移す方式を
  決定し実装する。project 自身が `trusted:true` を宣言できる
  現状を解消する。
- route の trust model を grant 参照 scope(trusted-scoped grant
  のみを解決 — M3 / #<ISSUE-4>)と整合させて設計する。
- 残存リスクの扱い: 本 issue 完了までは現行と同じ曝露(project
  が自ら `trusted` を宣言できる)が残る — 悪化はしないが放置も
  しない。

## Acceptance Criteria

- [ ] `trusted` flag の発行主体が project 自身ではなく
      operator/user 層で判定される設計が決定・記録されている
- [ ] project 層が自ら `trusted` を宣言しても route の trust
      判定を変えられないことがテストで確認できる
- [ ] grant-set・substitute の grant 参照 scope(trusted-scoped
      のみ解決)との整合が文書化されている

## Dependencies

- depends on #<ISSUE-1>, #<ISSUE-4>, #<ISSUE-5>(route 信頼モデル
  は grant 参照 scope の形が見えてから決める)

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

- `docs/tasks/file-issue-exec-contract-content-audit-removal.md`
  (T1 — issue 1 の番号を deps 記述へ引用)
- `docs/tasks/file-issue-route-grant-set-and-record-split.md`
  (T4 — issue 4 の番号を deps 記述へ引用)
- `docs/tasks/file-issue-substitute-exec-route-mode.md`
  (T5 — issue 5 の番号を deps 記述へ引用)

## Affected Files

- (GitHub) `scarecrow173/cheaper` の新規 issue ×1
- `docs/tasks/file-issue-trusted-flag-issuer-redesign.md`
  (起票結果欄の記録)

## Done When

- [ ] issue が `scarecrow173/cheaper` に起票済み(または重複確認で
      既存を検出し skip)
- [ ] issue body が `## Scope`・`## Acceptance Criteria`・
      `## Dependencies`・`## 参照` の 4 節を持ち、deps が
      placeholder ではなく実番号で記述されている
- [ ] 起票 URL・番号が本 task の起票結果欄に記録されている

## Verification

- [ ] `gh issue list --repo scarecrow173/cheaper --search "権限モデル再設計 6/7" --state all --json number,title,url`
      がちょうど 1 件を返す
- [ ] `gh issue list --repo scarecrow173/cheaper --search "権限モデル再設計 6/7" --state all --json body --jq ".[0].body"`
      の出力が `## Scope`・`## Acceptance Criteria`・
      `## Dependencies`・`## 参照` を全て含み、`<ISSUE` を含まない
