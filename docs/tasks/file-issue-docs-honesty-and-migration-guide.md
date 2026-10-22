---
id: "TASK-034KvxGAbTTWEsA1RbX7cn"
type: "task"
status: "done"
title: "起票 issue 7: 文書正直性 sweep + breaking-change migration guide"
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
    - "docs/tasks/file-issue-opt-in-exec-deny-patterns.md"
    - "docs/tasks/file-issue-unified-exec-grant-model.md"
    - "docs/tasks/file-issue-route-grant-set-and-record-split.md"
    - "docs/tasks/file-issue-substitute-exec-route-mode.md"
    - "docs/tasks/file-issue-trusted-flag-issuer-redesign.md"
  blocks:
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

# 起票 issue 7: 文書正直性 sweep + breaking-change migration guide

## Work

PLAN-034KvwX4x7oaZlaECqhHc1 の T7。DESIGN-034KuDkWOmNXiBddEQ4XWK
「§実装 issue 分解」の issue 7 を `gh` CLI で
`scarecrow173/cheaper` へ起票し、発行 URL を本 task の
「起票結果」欄へ記録する。

issue title / body は下記を**そのまま**使う — design 決定を
再解釈・再導出しない。body 内の `#<ISSUE-1>` 〜 `#<ISSUE-6>`
placeholder は、T1-T6 各 task doc の起票結果に記録された実番号で
置換してから起票する。

### 手順

1. `gh issue list --repo scarecrow173/cheaper --search "権限モデル再設計 7/7" --state all --json number,title,url`
   で重複確認。既に存在すれば作成を skip し、その URL を
   起票結果へ記録して完了とする。
2. T1-T6 各 task doc の起票結果から issue 1-6 の番号を取得し、
   下記 body 内の `#<ISSUE-1>` 〜 `#<ISSUE-6>` を実番号で置換する。
3. 置換後の body を temp file へ書き出し、
   `gh issue create --repo scarecrow173/cheaper --title "権限モデル再設計 7/7: 文書正直性 sweep + breaking-change migration guide" --body-file <temp file>`
   で起票する。
4. 発行 URL・番号を本 task の「起票結果」欄に記録する。

### issue title

`権限モデル再設計 7/7: 文書正直性 sweep + breaking-change migration guide`

### issue body

````markdown
## 背景

親 issue #151 の実装分解 7/7。「boundary/sandbox/protection」
表現と実強制範囲の整合(guides・refine 対象 ADR/spec)と、
破壊的変更の告知を行う。原則は **claim ≤ 実強制** — 提供しない
保護を文書で誇張しない。

## Scope

- 文書正直性 sweep — 「cheaper が workspace に閉じ込める」
  「deny list が保護」等の実強制を超える表現を guides・対象
  ADR/spec から訂正する。
- 「**cheaper は OS sandbox ではない**」ことを明記する —
  canonical file tool の workspace boundary(E1/E2)は cheaper
  所有 surface で強制できるが、spawn した子プロセスの fs
  副作用は OS sandbox なしでは強制不能。
- spawn API 層の shell-free 記述と ENOEXEC→`/bin/sh` fallback の
  区別を明記する — cheaper が使う spawn API は shell を介さず
  argv を直接渡す。ただし POSIX で shebang を持たない +X の
  text file 実行が ENOEXEC となった際に libc が `/bin/sh`
  fallback するのは OS/libc 既定動作であり cheaper の保証対象外。
- deny profile の表現規約 — `security.execDenyPatterns` は
  「deterministic executable deny-list」と呼び、「保護」
  「sandbox」「containment」とは呼ばない。
- refine 対象一覧(design「§Supersede / Refine 一覧」—
  supersede する doc は無し、すべて refine で扱う):
  - ADR(7 件): `command-grant-trust-boundary-and-managed-process-
    reuse`(ADR-034EMgaatFhsXjR97kH14c)・`worker-definition-
    runtime-policy-runtime-invariants-capability`
    (ADR-034r05iHlWosDpJ3DpG3XW)・`workspace-filesystem-boundary-
    cheaper-sandboxed-tool`(ADR-034r05iHlWosDpO699OwYF — 「子
    プロセスには及ばない」を明記)・`pretooluse-auto-routing-via-
    route-resolver-and-workflow-delegation`
    (ADR-034J8eZHNPqARi25HLFOL6 — C9 single-grant → grant-set
    投影へ一般化)・`per-target-hook-rewrite-enablement-with-
    policy-gated-approval-bypass`(ADR-034KrYGRHZuvOIx1d0S0nE —
    表現の軽微な訂正)・`defer-non-exec-routing-to-permission-
    model-redesign`(ADR-034KtTzOJGqmzmBW3SwGp5 — deferred
    sketch 採否の refine 注記)・`fix-tool-runtime-and-workflow-
    package-topology-and-shared-contracts-upfront`
    (ADR-034zGZpjoQlppwkkOs5gur — shell-free 記述の再記述)
  - spec/design(9 件): `specs/policy-gated-command-tool`
    (SPEC-034EMbEGBOWM4CAM6YSWm6)・`specs/invocation-scoped-
    command-grants`(SPEC-034FlOz4hiN9m0nQW7lBN2)・
    `specs/pretooluse-route-resolver-and-workflow-delegation`
    (SPEC-034J7SxWvGL3oSbeBpo8a3 — grant-set 前提 + R15 再記述
    済み)・`specs/managed-process-canonical-tool`
    (SPEC-034A8q6kmxcBsNF8zS9Cwn)・`specs/workflow-resource-
    inference`(SPEC-034zSWvgiB9gHQpjguNJ3M)・
    `designs/policy-gated-command-tool`
    (DESIGN-034EMWNJbQpMB0N3ZTXM5A — deny list 部分のみ
    置き換わる)・`designs/workflow-resource-inference`
    (DESIGN-034zSWw1LYrRvcB471oQm2)・`designs/rust-rig-runtime-
    migration-architecture`(DESIGN-034Bbq1qY8WFBdL26tUdBw —
    B5 由来記述の訂正)・`designs/pretooluse-route-resolver-and-
    delegated-workflow-execution`(DESIGN-034JbqoCWm85pKC033fgfd —
    viability 分類の変化)
- 破壊的変更一覧と移行手順の告知(migration guide):
  - executable deny list 削除(旧 builtin 一覧 →
    `security.execDenyPatterns` の seed 例を提供)
  - 内容審査 7 項目の除去(従来拒否されていた宣言 exec が受理
    される挙動変化)
  - 統一 grant 形状(`process` 節・`tool:"process"` は loader
    写像 + alias で存続 — 既存 policy.yaml・workflow は即死
    しないが resolved model は統一 grant へ)

## Acceptance Criteria

- [ ] 「cheaper は OS sandbox ではない」・「子プロセスの fs
      副作用は強制不能」が docs に明記されている
- [ ] 「sandbox/boundary/protection」表現と実強制の整合が
      guides・refine 対象 ADR/spec で取れている
- [ ] spawn API 層の shell-free と ENOEXEC→`/bin/sh` fallback
      (OS/libc 既定動作・保証対象外)の区別が記述されている
- [ ] design §Supersede/Refine 一覧の全対象 doc に refine が
      反映されている
- [ ] 破壊的変更一覧(deny list 削除・内容審査除去・grant
      形状)と移行手順が migration guide に列挙されている

## Dependencies

- depends on #<ISSUE-1>, #<ISSUE-2>, #<ISSUE-3>, #<ISSUE-4>,
  #<ISSUE-5>, #<ISSUE-6>(全実装変更の告知が対象のため最後)

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

- URL: https://github.com/scarecrow173/cheaper/issues/169
- 番号: #169

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/file-permission-model-redesign-implementation-issues.md`
- `docs/designs/permission-model-responsibility-redesign.md`
- `docs/specs/permission-model-responsibility-redesign.md`
- `docs/adr/enforce-declared-contracts-over-content-audit.md`

## Dependencies

- T1-T6 全 task(issue 1-6 の番号を deps 記述へ引用するため最後)

## Affected Files

- (GitHub) `scarecrow173/cheaper` の新規 issue ×1
- `docs/tasks/file-issue-docs-honesty-and-migration-guide.md`
  (起票結果欄の記録)

## Done When

- [x] issue が `scarecrow173/cheaper` に起票済み(または重複確認で
      既存を検出し skip)
- [x] issue body が `## Scope`・`## Acceptance Criteria`・
      `## Dependencies`・`## 参照` の 4 節を持ち、deps が
      placeholder ではなく実番号で記述されている
- [x] body が「cheaper は OS sandbox ではない」・spawn API 層の
      shell-free と ENOEXEC fallback の区別・refine 対象一覧・
      破壊的変更一覧を含む(TSPEC-034Kvx6yD4sfTj8KFkFIF3 の
      issue 7 Rule)
- [x] 起票 URL・番号が本 task の起票結果欄に記録されている

## Verification

- [ ] `gh issue list --repo scarecrow173/cheaper --search "権限モデル再設計 7/7" --state all --json number,title,url`
      がちょうど 1 件を返す
- [ ] `gh issue list --repo scarecrow173/cheaper --search "権限モデル再設計 7/7" --state all --json body --jq ".[0].body"`
      の出力が `## Scope`・`## Acceptance Criteria`・
      `## Dependencies`・`## 参照` を全て含み、`<ISSUE` を含まない
