---
id: "TASK-034KvxFptiaMDpvi8xe9dM"
type: "task"
status: "done"
title: "起票 issue 5: substitute-exec route mode"
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
    - "docs/tasks/file-issue-route-grant-set-and-record-split.md"
  blocks:
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
    - "docs/adr/defer-non-exec-routing-to-permission-model-redesign.md"
  defers: []
  deferred-by: []
---

# 起票 issue 5: substitute-exec route mode

## Work

PLAN-034KvwX4x7oaZlaECqhHc1 の T5。DESIGN-034KuDkWOmNXiBddEQ4XWK
「§実装 issue 分解」の issue 5 を `gh` CLI で
`scarecrow173/cheaper` へ起票し、発行 URL を本 task の
「起票結果」欄へ記録する。

issue title / body は下記を**そのまま**使う — design 決定を
再解釈・再導出しない。body 内の `#<ISSUE-4>` placeholder は、
T4(`docs/tasks/file-issue-route-grant-set-and-record-split.md`)
の起票結果に記録された実番号で置換してから起票する。

### 手順

1. `gh issue list --repo scarecrow173/cheaper --search "権限モデル再設計 5/7" --state all --json number,title,url`
   で重複確認。既に存在すれば作成を skip し、その URL を
   起票結果へ記録して完了とする。
2. T4 task doc の起票結果から issue 4 の番号を取得し、下記 body
   内の `#<ISSUE-4>` を `#<実番号>` へ置換する。
3. 置換後の body を temp file へ書き出し、
   `gh issue create --repo scarecrow173/cheaper --title "権限モデル再設計 5/7: substitute-exec route mode" --body-file <temp file>`
   で起票する。
4. 発行 URL・番号を本 task の「起票結果」欄に記録する。

### issue title

`権限モデル再設計 5/7: substitute-exec route mode`

### issue body

````markdown
## 背景

親 issue #151 の実装分解 5/7。route が `substitute:{grant,argv}`
を宣言できる substitute-exec mode — 非 exec routing deferred
sketch(ADR-034KtTzOJGqmzmBW3SwGp5)の採用要素。precheck の
strict さを保ったまま「宣言通り実行する command」を検査する。

## Scope

- route `substitute:{grant,argv}` 宣言を追加する。
- viability の検査対象を `substitute.{grant,argv}` へ retarget
  する(原 argv でなく substitute を `check_command_execution`
  で検査)。
- `expected_input`(`route/run.rs:176` の
  `{name:grant,args:exec_args}`)を substitute mode では
  **`{name:substitute.grant,args:substitute.argv}` へ再 anchor**
  し、exec-input byte 一致(D11)を substitute argv へ適用する。
- exit mirror は実行した command の exit を mirror するだけで
  機構維持 — substitute 宣言あり → substitute exit、なし →
  原 command exit。route-run exit は「原 command の exit」では
  なく「実行した command の exit」(substitute mode では親
  agent が substitute の exit を原 command のものとして解釈する
  semantic substitution を trusted 層 opt-in で受容する帰結 —
  `docs/specs/pretooluse-route-resolver-and-workflow-delegation.md`
  R15 は再記述済み、要件 ID は維持)。
- grant anchor 規約(substitute mode): `expected_input` の `name`
  = `substitute.grant`、lifecycle bound(deadline/timeout 等)は
  `substitute.grant` の宣言値、usage record の route 帰属は
  `route.grant` anchor、`{command, executed}` の `executed`
  field が substitute argv を記録する。
- **Option E(exec-less route — exec job 不要、`outputs.exitCode`
  の author 宣言値 mirror)は本 issue では実装しない**(open
  事項 — product 需要が未確認。author 宣言 exit を信用する
  設計が前提になり、未決のまま実装すると成功偽装の hole を
  開ける。route `grant` field は firing gate として留保される
  設計だけ確定)。

## Acceptance Criteria

- [ ] route `substitute:{grant,argv}` 宣言が schema に存在する
- [ ] viability の検査対象が原 argv でなく substitute ペアへ
      retarget される
- [ ] `expected_input` が `{substitute.grant, substitute.argv}`
      に anchor され、exec-input byte 一致(D11)が substitute
      argv に適用される
- [ ] exit mirror が「実行した command の exit」を mirror する
      (substitute 宣言あり → substitute exit)
- [ ] usage record が route 帰属 `route.grant` + `executed` =
      substitute argv を記録する
- [ ] Option E が本 issue の範囲外(open 事項)であることが
      本文またはコメントで明示されている

## Dependencies

- depends on #<ISSUE-4>(grant-set 投影・record split・viability
  条件付き弱化の形が見えてから)

## 参照

- 親 issue: https://github.com/scarecrow173/cheaper/issues/151
- design: `docs/designs/permission-model-responsibility-redesign.md`
  (DESIGN-034KuDkWOmNXiBddEQ4XWK)
- spec: `docs/specs/permission-model-responsibility-redesign.md`
  (SPEC-034KuqPkTK2b4mHsykH9A5)
- ADR: `docs/adr/enforce-declared-contracts-over-content-audit.md`
  (ADR-034KuqZelggTIb8Z8K9QB8)
- deferred input:
  `docs/adr/defer-non-exec-routing-to-permission-model-redesign.md`
  (ADR-034KtTzOJGqmzmBW3SwGp5)
- spec R15 再記述先:
  `docs/specs/pretooluse-route-resolver-and-workflow-delegation.md`
  (SPEC-034J7SxWvGL3oSbeBpo8a3)
- discovery:
  `docs/discovery/permission-model-responsibility-redesign.md`
  (DISC-034KuleA9g82Ir4xWfmTYq)
````

### 起票結果

- URL: https://github.com/scarecrow173/cheaper/issues/167
- 番号: #167

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/file-permission-model-redesign-implementation-issues.md`
- `docs/designs/permission-model-responsibility-redesign.md`
- `docs/specs/permission-model-responsibility-redesign.md`
- `docs/adr/enforce-declared-contracts-over-content-audit.md`
- `docs/adr/defer-non-exec-routing-to-permission-model-redesign.md`
  (deferred sketch の input — 別 chain のため references 止まり)

## Dependencies

- `docs/tasks/file-issue-route-grant-set-and-record-split.md`
  (T4 — issue 4 の番号を deps 記述へ引用)

## Affected Files

- (GitHub) `scarecrow173/cheaper` の新規 issue ×1
- `docs/tasks/file-issue-substitute-exec-route-mode.md`
  (起票結果欄の記録)

## Done When

- [x] issue が `scarecrow173/cheaper` に起票済み(または重複確認で
      既存を検出し skip)
- [x] issue body が `## Scope`・`## Acceptance Criteria`・
      `## Dependencies`・`## 参照` の 4 節を持ち、deps が
      `#<ISSUE-4>` placeholder ではなく実番号で記述されている
- [x] body が substitute anchor 規約・D11 維持・「実行した
      command の exit」・Option E の open 扱いを含む
      (TSPEC-034Kvx6yD4sfTj8KFkFIF3 の issue 5 Rule)
- [x] 起票 URL・番号が本 task の起票結果欄に記録されている

## Verification

- [ ] `gh issue list --repo scarecrow173/cheaper --search "権限モデル再設計 5/7" --state all --json number,title,url`
      がちょうど 1 件を返す
- [ ] `gh issue list --repo scarecrow173/cheaper --search "権限モデル再設計 5/7" --state all --json body --jq ".[0].body"`
      の出力が `## Scope`・`## Acceptance Criteria`・
      `## Dependencies`・`## 参照` を全て含み、`<ISSUE` を含まない
