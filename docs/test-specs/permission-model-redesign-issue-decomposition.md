---
id: "TSPEC-034Kvx6yD4sfTj8KFkFIF3"
type: "test-spec"
status: "draft"
title: "権限モデル再設計 issue 分解の成果物契約 (issue #151)"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/file-permission-model-redesign-implementation-issues.md"
    - "docs/discovery/permission-model-responsibility-redesign.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/permission-model-responsibility-redesign.md"
    - "docs/designs/permission-model-responsibility-redesign.md"
    - "docs/adr/enforce-declared-contracts-over-content-audit.md"
  derived-by: []
  verifies:
    - "docs/specs/permission-model-responsibility-redesign.md"
    - "docs/designs/permission-model-responsibility-redesign.md"
  verified-by:
    - "docs/tasks/file-issue-exec-contract-content-audit-removal.md"
    - "docs/tasks/file-issue-opt-in-exec-deny-patterns.md"
    - "docs/tasks/file-issue-unified-exec-grant-model.md"
    - "docs/tasks/file-issue-route-grant-set-and-record-split.md"
    - "docs/tasks/file-issue-substitute-exec-route-mode.md"
    - "docs/tasks/file-issue-trusted-flag-issuer-redesign.md"
    - "docs/tasks/file-issue-docs-honesty-and-migration-guide.md"
    - "docs/tasks/record-issue-urls-and-verify-investigation-criteria.md"
  references:
    - "docs/adr/defer-non-exec-routing-to-permission-model-redesign.md"
  defers: []
  deferred-by: []
---

# 権限モデル再設計 issue 分解の成果物契約 (issue #151)

## Purpose

issue #151 は調査・再設計 issue であり、その「実装」は
DESIGN-034KuDkWOmNXiBddEQ4XWK が確定した 7 つの実装 issue の
起票と成果物の記録に等しい。本 test-spec は、起票される GitHub
issue 群が design の決定事項を**欠落なく保持する**ことを保証する
意図を固定する — issue body が後続実装の唯一の authoritative
入力になるため、起票時の情報脱落はそのまま実装の誤りになる。

本 spec が守るべき意図は 3 つ。(1) **分解の忠実性**: 7 issue の
title/scope/deps が design「§実装 issue 分解」と一致する。
(2) **決定の保存性**: M1-M5(merge fidelity)・disposition
(remove 7 + opt-in 1)・handoff 表の採否・open 事項(Option E)が
issue 本文に残る。(3) **正直性の記述**: 「cheaper は OS
sandbox ではない」「deny list は deterministic deny-list であり
containment ではない」等の限定表現が、関連 issue・docs 修正指示に
含まれる。

退役条件: 7 issue 全てが close され、かつ各 issue の実装 AC が
後続 chain の test-spec へ引き継がれた時点(本 spec は起票成果物
だけを見る)。

## Feature

issue #151 の調査・再設計成果物として起票される 7 実装 issue と
記録物(URL 記録・#151 への comment・IR)の内容契約。

## Rules

- Rule: 7 つの GitHub issue が `scarecrow173/cheaper` に存在し、
  title が design「§実装 issue 分解」の 1-7 に対応する
  (exec-contract cleanup / opt-in execDenyPatterns / 統一 grant
  統合 / route 審査整理+grant-set+record split / substitute-exec /
  trusted flag 発行主体 / 文書正直性+migration guide)。
- Rule: 各 issue body に `## Scope`・`## Acceptance Criteria`・
  `## Dependencies`・`## 参照` の 4 節が存在し、`## 参照` が
  issue #151・DESIGN-034KuDkWOmNXiBddEQ4XWK・
  ADR-034KuqZelggTIb8Z8K9QB8 を指す。
- Rule: `## Dependencies` の内容が design の依存グラフと一致する
  — 1: なし / 2: {1} / 3: {1} / 4: {1,3} / 5: {4} /
  6: {1,4,5} / 7: {1,2,3,4,5,6}。引用は起票された実 issue
  番号で行われる。
- Rule: issue 2 の本文が opt-in deny profile の要件を保持する
  — `security.execDenyPatterns` schema 新節・既定=空・
  authoring は trusted 層のみ・deny の効果は invocation 合成
  grant を含む resolved grant map 全体・「deterministic
  executable deny-list」と呼び「保護」「sandbox」「containment」と
  呼ばない表現規約・旧 builtin 一覧の migration 例。
- Rule: issue 3 の本文が merge fidelity 決定 M1-M5 を全て保持する
  — M1(optional inputs/outputs)・M2(`fixed: true` marker)・
  M3(`process` 節の project-authorable 維持 + project-scoped
  grant namespace 写像 + route grant 参照は trusted-scoped のみ)・
  M4(writeScope 下は `fixed` かつ `outputs ⊆ writeScope` の
  per-call coverage)・M5(`tool:"process"` alias 維持)。
- Rule: issue 4 の本文が条件付き viability 弱化と record split を
  保持する — byte-matched mode の argv precheck 維持・set 内非
  exec grant は存在+pin 解決可能性のみ・`kind:"route"` record の
  `{command, executed}` 分離・attribution は `route.grant` anchor。
- Rule: issue 5 の本文が substitute mode の anchor 規約と Option E
  の open 扱いを保持する — `expected_input` の
  `{substitute.grant, substitute.argv}` 再 anchor・D11 mirror 機構
  維持・exit は「実行した command の exit」・Option E は実装
  しない(open 事項として明記)。
- Rule: issue 7 の本文が文書正直性要件を保持する — 「cheaper は
  OS sandbox ではない」「spawn API 層の shell-free と ENOEXEC 時の
  libc `/bin/sh` fallback は OS/libc 既定動作で cheaper の保証
  対象外」の区別・refine 対象 ADR/spec/design 一覧・破壊的変更
  一覧(deny list 削除・内容審査除去・grant 形状)。
- Rule: issue #151 の成果物基準が確認される — (1) 現状調査表
  (discovery 分類表)・(2) trust/permission モデル提案(spec+ADR)・
  (3) ADR/spec 改訂案(accepted ADR + approved spec/design)・
  (4) 実装分割と依存関係の記録(7 issue + deps)・(5) 検証計画
  (各 issue の AC + design Verification Notes)。
- Rule: issue URL が plan の Task Breakdown 記録欄・各起票 task・
  `docs/impl/ir/file-permission-model-redesign-implementation-issues.md`
  の IR に記録される。

## Examples

- Example: issue 3 が M1-M5 を欠落させると、実装者は task の
  inputs/outputs 情報を持たない単純畳み込みを行い、manifest・
  writeScope・ClaimMode の semantics を壊す — 本 spec の M1-M5
  保持 Rule がこれを防ぐ。
- Example: issue 2 の本文から「既定=空」「trusted 層のみ
  authoring」が抜けると、実装者は builtin deny list を mandatory
  で復活させるか、project 層から deny  authoring を許す設計に
  なる — 責務モデル(ADR-034KuqZelggTIb8Z8K9QB8 Option B)に反する。
- Example: 「cheaper が workspace に閉じ込める」等の誇張表現が
  issue 7 の sweep 対象から漏れると、claim と実強制の乖離が残り
  spec R5/正直性 AC を満たさない。
- Example: T8 が deps 記述の実番号引用を確認しないまま完了すると、
  「deps: 1」のままの issue が残り、後続実装の着手順が解決不能に
  なる。

## Guarantees

- 起票 worker が design・spec を再解釈しなくても、task 本文の
  issue body 転記だけで上記 Rules を全て満たす issue が作れる。
- T8 の検証手順が、Rules の各項目を `gh issue view`/`gh issue list`
  の出力と突き合わせる機械的確認として機能する。
- issue #151 の成果物基準 5 項目それぞれに対応する証跡
  (discovery/spec/ADR/design/7 issue/検証記述)が指名される。

## Non-goals

- 起票した issue の実装内容そのものの正しさ(後続 issue chain の
  test-spec が担う)。
- issue body の文体・表記揺れの検証(内容の有無のみ)。
- Option E exec-less route の採否そのもの(open 事項として
  「記録されていること」を検証し、判断はしない)。
- GitHub label・milestone・project 紐付けの有無。

## Risk

- M1-M5 が issue 3 から脱落すると、統合時に task の resource 宣言
  (inputs/outputs→produces・ClaimMode)が失われ、manifest・
  writeScope の semantics が静かに壊れる。
- 「正直性」記述が issue 7 から漏れると、docs が実強制を超える
  保証を主張し続け、spec R5 と ADR の責務決定が形骸化する。
- deps 記述が番号引用で確定されないと、design が定めた
  依存順(1→{2,3}→4→5→6→7)が追跡不能になり、後続実装が
  前提未了の issue から着手する。
- issue #151 の成果物基準の検証を省略すると、分解漏れ(例:
  refine 対象一覧の未起票)が発覚しないまま調査 issue が close
  される。
