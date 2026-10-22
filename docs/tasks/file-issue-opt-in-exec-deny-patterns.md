---
id: "TASK-034KvxvHeO6yOVxflkxTPO"
type: "task"
status: "todo"
title: "起票 issue 2: opt-in security.execDenyPatterns profile (A9 降格)"
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

# 起票 issue 2: opt-in security.execDenyPatterns profile (A9 降格)

## Work

PLAN-034KvwX4x7oaZlaECqhHc1 の T2。DESIGN-034KuDkWOmNXiBddEQ4XWK
「§実装 issue 分解」の issue 2 を `gh` CLI で
`scarecrow173/cheaper` へ起票し、発行 URL を本 task の
「起票結果」欄へ記録する。

issue title / body は下記を**そのまま**使う — design 決定を
再解釈・再導出しない。body 内の `#<ISSUE-1>` placeholder は、
T1(`docs/tasks/file-issue-exec-contract-content-audit-removal.md`)
の起票結果に記録された実番号で置換してから起票する。

### 手順

1. `gh issue list --repo scarecrow173/cheaper --search "権限モデル再設計 2/7" --state all --json number,title,url`
   で重複確認。既に存在すれば作成を skip し、その URL を
   起票結果へ記録して完了とする。
2. T1 task doc の起票結果から issue 1 の番号を取得し、下記 body
   内の `#<ISSUE-1>` を `#<実番号>` へ置換する。
3. 置換後の body を temp file へ書き出し、
   `gh issue create --repo scarecrow173/cheaper --title "権限モデル再設計 2/7: opt-in security.execDenyPatterns profile (A9 降格)" --body-file <temp file>`
   で起票する。
4. 発行 URL・番号を本 task の「起票結果」欄に記録する。

### issue title

`権限モデル再設計 2/7: opt-in security.execDenyPatterns profile (A9 降格)`

### issue body

````markdown
## 背景

親 issue #151 の実装分解 2/7。
executable deny list(A9 `command_denied_executable_re`)を builtin
定数の mandatory 審査から、operator opt-in の deterministic deny
へ降格する。「既知の危険名一覧」は containment ではなく
(改名・別ツール・子プロセス経由で構造的に bypass 無限)、
mandatory 層には残さない。

## Scope

- policy schema に `security` 節を新設し `execDenyPatterns` を
  追加する:

```yaml
security:
  execDenyPatterns: []   # 既定: 空。operator が regex 名リストを宣言すると
                         # grant executable の deterministic deny として効く
```

- authoring は trusted 層(builtin/user)のみ — invocation 層は
  keys ⊆ `{version, command.grants}` の構造 guard
  (`load.rs:119-150`)で `security` を書けず、project partial
  schema も同様に受け付けない。
- deny の効果は invocation 合成 grant(`--grant` flag 経路)にも
  等しく及ぶ — deny 評価は `resolve_command_policy`(declared
  basename)・`pin_command_grant`(resolved basename)・
  `verify_pinned_executable`(spawn 前再検証)の既存適用点で、
  layer 由来情報を持たない resolved `command.grants` map 全体に
  対して効く(operator deny は invocation grant を exempt
  しない)。
- 既定値は空 — builtin 一覧を mandatory には残さない。
- 現行の builtin 一覧は `docs` に「旧既定値」として記録し、
  必要な operator が自前で seed できる migration 例を提供する。
- 表現は「deterministic executable deny-list」。「保護」
  「sandbox」「containment」とは呼ばない。

## Acceptance Criteria

- [ ] policy schema に `security.execDenyPatterns` が存在し、
      既定値が空である
- [ ] trusted 層のみが authoring でき、invocation 層・project
      partial が `security` を受け付けない構造になっている
- [ ] operator が `execDenyPatterns` を宣言した場合のみ、
      declared/resolved basename・spawn 前再検証の既存 3 適用点で
      deterministic deny が効き、invocation 合成 grant も対象に
      なる
- [ ] 既定(空)では `python`/`ssh`/`tar` 等を宣言した grant が
      拒否されない
- [ ] 旧 builtin 一覧と seed 用 migration 例が docs に記録されて
      いる

## Dependencies

- depends on #<ISSUE-1>(deny list の mandatory 除去後に opt-in
  profile として再配置する)

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
  (T1 — issue 1 の番号を deps 記述へ引用するため)

## Affected Files

- (GitHub) `scarecrow173/cheaper` の新規 issue ×1
- `docs/tasks/file-issue-opt-in-exec-deny-patterns.md`
  (起票結果欄の記録)

## Done When

- [ ] issue が `scarecrow173/cheaper` に起票済み(または重複確認で
      既存を検出し skip)
- [ ] issue body が `## Scope`・`## Acceptance Criteria`・
      `## Dependencies`・`## 参照` の 4 節を持ち、deps が
      `#<ISSUE-1>` placeholder ではなく実番号で記述されている
- [ ] body が既定=空・trusted 層のみ authoring・invocation grant
      にも効く・「deterministic executable deny-list」の表現規約を
      含む(TSPEC-034Kvx6yD4sfTj8KFkFIF3 の issue 2 Rule)
- [ ] 起票 URL・番号が本 task の起票結果欄に記録されている

## Verification

- [ ] `gh issue list --repo scarecrow173/cheaper --search "権限モデル再設計 2/7" --state all --json number,title,url`
      がちょうど 1 件を返す
- [ ] `gh issue list --repo scarecrow173/cheaper --search "権限モデル再設計 2/7" --state all --json body --jq ".[0].body"`
      の出力が `## Scope`・`## Acceptance Criteria`・
      `## Dependencies`・`## 参照` を全て含み、`<ISSUE` を含まない
