---
id: "SPEC-034Zu8Xj0Y5x6ii9aDOGuS"
type: "spec"
status: "approved"
title: "cheaper-workflow-create skill for issue 204"
created: "2026-11-18"
updated: "2026-11-18"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/204"
    - "https://github.com/scarecrow173/cheaper/issues/200"
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
    - "workflow-file-interface.md"
    - "workflow-contract-compiler.md"
    - "workflow-catalog-issue-201.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/workflow-create-skill-issue-204.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# cheaper-workflow-create skill for issue 204

## What Should Be Built

`packages/apm/.apm/skills/cheaper-workflow-create/SKILL.md` を新設し、
親 Agent が Workflow JSON v1 の設計・生成・静的検証・catalog 登録・
利用例提示を、現行 Rust 実装が受理する contract のみで一貫して
行えるようにする。skill は定義 file の保存までを担い、実行
(`workflow run`)・grant/policy/Route の変更は責務としない。

## Why Now

Epic #200 の3層導線(L0 判定 → L1 discover #203 → L2 create #204)の
最終層。#201 で catalog manifest と `workflow list` が実装済みで、
登録先の file 契約が確定した。discover の完成は必須ではなく、
明示的な作成依頼でも独立利用できる必要がある。

## Users and Value

親 Coding Agent が、user 承認の範囲内で Workflow JSON v1 を
設計し、`workflow interface` で検証し、catalog へ reviewable な
差分として登録できる。利用者は承認なしに定義 file・catalog・
権限・実行が変化しないことを保証される。

## Scope

### In Scope

- `cheaper-workflow-create/SKILL.md` — 発動条件・承認境界・
  authoring 手順・schema 要点・template・良い/悪い例・検証手順・
  catalog 登録手順・invocation 例・出力契約。
- 公開 input/output 契約の確定 → catalog 既存候補との比較 →
  JSON v1 組立 → 配置 path 決定 → `workflow interface` 検証 →
  catalog 登録 → invocation 例の一連の手順。
- 実例 5 形態(最小 worker・tool+worker chain・fan-in・inputs
  defaults・`when` と skip)の検証可能な template。
- `docs/guides/workflow-usage.md` と
  `docs/guides/parent-agent-integration.md` への導線(最小限)。

### Out of Scope

- 新しい Workflow schema・YAML runtime・CLI `workflow create`
  generator 本体・interactive daemon・nested workflow・自己更新
  Workflow の実装。
- `workflow run` の起動・副作用を伴う smoke(command/process/
  file write/LLM 課金)。
- grant/policy/Route の自動改変、catalog 書込み CLI。
- `cheaper-workflow-discover`(#203)の実装(独立利用可能だが依存では
  ない)。

## Requirements

- 発動は明示的な作成依頼、または #203 提案への user 承認。
  候補発見だけでは file を永続化しない。定義保存と実行は別で、
  `workflow run` や副作用 smoke は user の明示承認時のみ。
- binding は構造化 `$input`/`$from`+RFC 6901 pointer/`$literal`。
  string interpolation や不正 wrapper、架空の job/tool/worker を
  禁止。`--input '{...}'` の inline JSON は記述しない
  (`--input <file>|-` 規約)。
- skill 本文の例は現行 CLI と schema で検証済みの形のみ。
  `run` に実在しない `--model`、stdin 定義、YAML 読込を提案しない。
- catalog 登録は `{id,path,summary,tags}` の追記で、id 重複・
  manifest 競合・既存定義上書きを黙って処理せず人が review できる
  差分にする。
- 静的検証は `cheaper workflow interface <path>` が accept する
  こと。失敗時は修正して再検証し、invalid definition を完成品と
  しない。

## Acceptance Criteria

- [ ] AC1: `cheaper-workflow-create/SKILL.md` が APM 配置と相互
  skill 参照規約(`cheaper-usage` 等)に合う。
- [ ] AC2: 最小 worker、tool+worker chain、fan-in、inputs
  defaults、`when` と skip を含む実例について JSON v1 生成が可能。
- [ ] AC3: 生成物に対する `workflow interface` の静的検証が pass
  し、invalid case が明示的な失敗として扱われる。
- [ ] AC4: catalog の重複判定・競合処理・保存/登録が独立した
  revision として追跡できる。
- [ ] AC5: 承認がない場合は定義 file・catalog・grant・Route を
  書き換えない。
- [ ] AC6: user 承認のない `workflow run` や副作用付き smoke を
  しない。
- [ ] AC7: `--input <file>|-`、`--cwd`、public interface など
  CLI 実装と整合する。
- [ ] AC8: skill 本文に良い/悪い例・権限/変更範囲の注意・検証
  可能な template を含む。

## Deferred Design Concerns

- catalog 書込み CLI(`workflow register` 等) | #201 で deferred、
  file 契約で登録可能 | skill 運用で手順の誤り率が実害化した段階 |
  なしでも file 契約で完結する。
- `cheaper-workflow-discover`(#203)との相互参照 | #203 未実装で
  リンク先が未定 | #203 が skill として着地した時点で追記 |
  片方向参照のみで不整合は起きない。

## Sources and Relations

Issue #204(親 Epic #200、依存 #201)が product source。schema 正本:
`crates/cheaper-workflow/src/compiler.rs`・`schema.rs`、spec 正本:
`workflow-contract-compiler.md`・`workflow-file-interface.md`、
catalog 契約: `workflow-catalog-issue-201.md`・
`docs/guides/workflow-usage.md`。discovery:
`../discovery/workflow-create-skill-issue-204.md`。
