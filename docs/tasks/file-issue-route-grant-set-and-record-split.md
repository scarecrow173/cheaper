---
id: "TASK-034KvxwOy3rcRuUnlZu4Ej"
type: "task"
status: "todo"
title: "起票 issue 4: route/hook 審査依存整理 + grant-set 投影 + record split"
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
    - "docs/tasks/file-issue-unified-exec-grant-model.md"
  blocks:
    - "docs/tasks/file-issue-substitute-exec-route-mode.md"
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

# 起票 issue 4: route/hook 審査依存整理 + grant-set 投影 + record split

## Work

PLAN-034KvwX4x7oaZlaECqhHc1 の T4。DESIGN-034KuDkWOmNXiBddEQ4XWK
「§実装 issue 分解」の issue 4 を `gh` CLI で
`scarecrow173/cheaper` へ起票し、発行 URL を本 task の
「起票結果」欄へ記録する。

issue title / body は下記を**そのまま**使う — design 決定を
再解釈・再導出しない。body 内の `#<ISSUE-1>`・`#<ISSUE-3>`
placeholder は、T1・T3 task doc の起票結果に記録された実番号で
置換してから起票する。

### 手順

1. `gh issue list --repo scarecrow173/cheaper --search "権限モデル再設計 4/7" --state all --json number,title,url`
   で重複確認。既に存在すれば作成を skip し、その URL を
   起票結果へ記録して完了とする。
2. T1・T3 task doc の起票結果から issue 1・3 の番号を取得し、
   下記 body 内の `#<ISSUE-1>`・`#<ISSUE-3>` を実番号で置換する。
3. 置換後の body を temp file へ書き出し、
   `gh issue create --repo scarecrow173/cheaper --title "権限モデル再設計 4/7: route/hook 審査依存整理 + grant-set 投影 + record split" --body-file <temp file>`
   で起票する。
4. 発行 URL・番号を本 task の「起票結果」欄に記録する。

### issue title

`権限モデル再設計 4/7: route/hook 審査依存整理 + grant-set 投影 + record split`

### issue body

````markdown
## 背景

親 issue #151 の実装分解 4/7。route/hook 側で内容審査に依存して
いる分岐を整理し、非 exec routing deferred sketch
(ADR-034KtTzOJGqmzmBW3SwGp5)の採用要素 — grant-set 投影・
条件付き viability 弱化・record split — を実装する。
routing ≠ authorization(D2)は維持する。

## Scope

- viability の unsafe 分岐が内容審査を呼ぶ経路の整理 — viability
  は authorization(grant/allowlist 検査 =
  `check_command_execution` 委譲)を呼ぶが内容審査を呼ばない
  構造へ。
- `project_grant` → route 宣言 `grants:[name,…]` の set 投影へ
  一般化(統一 grant モデル上 — #<ISSUE-3>)。grant 参照は
  trusted-scoped grant のみを解決する(M3 — project-scoped
  grant は route の firing/authorization を満たせない)。
  confused-deputy 半径は declared grant argv union に明示的に
  閉じる。
- viability の条件付き弱化: byte-matched mode では原 argv =
  実行 argv(D11)なので `check_command_execution` による argv
  precheck(`resolve.rs:304-335`)は**維持**する — 無条件に
  弱めると argv-mismatch の現行挙動(viability=`no-grant` →
  route 不発 → 原 command が直接実行)が「route 発火 → exec
  job 失敗(原 command は走らない)」へ変わる後退になる。
  弱めるのは set 内の非 exec grant のみ — 存在・pin 解決可能
  性のみ viability で見て、argv 照合は job 実行時へ委譲する
  (direct `workflow run --grant` と同一)。
- `kind:"route"` usage record を `{command, executed}` に拡張 —
  「実行されなかった command」を command attribution にしない。
  record の route 帰属(route × firing gate の認可 grant)は
  `route.grant` に anchor する(grant set は実行範囲の宣言で
  あり attribution anchor を構成しない)。`executed` field が実
  exec argv(byte-matched では原 argv)を記録する。
- grant anchor 規約: `expected_input` の `name` は exec job が
  実際に走らせる grant(byte-matched/grant-set mode では
  `route.grant`)。deadline/timeout 等の lifecycle bound は
  `expected_input` が指す実行 grant の宣言値を使う
  (`command::execute` が named grant の `timeout`/`killGrace`/
  `exitCodes` を読む既存規約)。set 内の非 exec grant はそれを
  呼ぶ追加 job で自身の宣言 bound を使う。

## Acceptance Criteria

- [ ] viability が authorization(grant/allowlist)を呼ぶが内容審査
      を呼ばない構造になっている
- [ ] route 宣言 `grants:[name,…]` の set 投影が trusted-scoped
      grant のみを解決する
- [ ] byte-matched mode の argv precheck が維持され、
      argv-mismatch 入力が route 不発 → 原 command 直接実行の
      fallback を保つ(条件付き弱化)
- [ ] set 内の非 exec grant が存在 + pin 解決可能性のみ
      viability で検査される
- [ ] `kind:"route"` record が `{command, executed}` を持ち、
      attribution が `route.grant` に anchor される

## Dependencies

- depends on #<ISSUE-1>, #<ISSUE-3>(統一 grant モデル上での set
  投影のため)

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
- `docs/adr/defer-non-exec-routing-to-permission-model-redesign.md`
  (deferred sketch の input — 別 chain のため references 止まり)

## Dependencies

- `docs/tasks/file-issue-exec-contract-content-audit-removal.md`
  (T1 — issue 1 の番号を deps 記述へ引用)
- `docs/tasks/file-issue-unified-exec-grant-model.md`
  (T3 — issue 3 の番号を deps 記述へ引用)

## Affected Files

- (GitHub) `scarecrow173/cheaper` の新規 issue ×1
- `docs/tasks/file-issue-route-grant-set-and-record-split.md`
  (起票結果欄の記録)

## Done When

- [ ] issue が `scarecrow173/cheaper` に起票済み(または重複確認で
      既存を検出し skip)
- [ ] issue body が `## Scope`・`## Acceptance Criteria`・
      `## Dependencies`・`## 参照` の 4 節を持ち、deps が
      placeholder ではなく実番号で記述されている
- [ ] body が条件付き viability 弱化・`{command, executed}`
      record split・`route.grant` attribution anchor を含む
      (TSPEC-034Kvx6yD4sfTj8KFkFIF3 の issue 4 Rule)
- [ ] 起票 URL・番号が本 task の起票結果欄に記録されている

## Verification

- [ ] `gh issue list --repo scarecrow173/cheaper --search "権限モデル再設計 4/7" --state all --json number,title,url`
      がちょうど 1 件を返す
- [ ] `gh issue list --repo scarecrow173/cheaper --search "権限モデル再設計 4/7" --state all --json body --jq ".[0].body"`
      の出力が `## Scope`・`## Acceptance Criteria`・
      `## Dependencies`・`## 参照` を全て含み、`<ISSUE` を含まない
