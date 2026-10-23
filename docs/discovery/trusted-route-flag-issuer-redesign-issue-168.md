---
id: "DISC-034L44O8jYRDhvhM16gDiM"
type: "discovery"
status: "active"
title: "trusted route flag issuer redesign issue-168"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
relations:
  source: []
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
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# trusted route flag issuer redesign issue-168

## Exploration Goal

GitHub issue #168（権限モデル再設計 6/7: trusted route flag 発行主体の再設計 D4）の実装検討。
`trusted:true` を project 自身が宣言できる現状の再設計 — trust 判定主体を
operator/user 層へ移す方式の決定と実装。route trust model は grant 参照 scope
（trusted-scoped grant のみ解決 — M3/#166・substitute #167）と直に交差するため
依存完了後の着手。depends on #163/#166/#167（全完了済み）。

## Key Issues and Assumptions

- 現状: `routes.rs` — project route は `trusted: true` 宣言時のみ下位層（builtin/user）を shadow できる。project 自己申告で trust 昇格するのが問題
- `trusted` は現在 shadowing gate のみで使用（判別子として使えないことは #166/#167 で確立済み — `grants`/`substitute` は layer provenance で判定する先例）
- 発行主体を operator/user 層へ: trust は宣言値ではなく宣言層 provenance で判定する設計が本命（project 層の `trusted` 宣言は無効化 or reject）
- grant 参照 scope（route が解決できるのは trusted-scoped grant のみ）との整合を文書化
- project が `trusted` を宣言しても route の trust 判定を変えられないことを test で確認
- builtin/user 層 route は引き続き上位として扱われるか（builtin が最下位か最上位かの precedence 確認要）

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| <!-- option --> | <!-- pro --> | <!-- con --> | <!-- yes/no/maybe --> |

## Tentative Conclusions and Hypotheses

<!-- Current best guess before committing to a spec or ADR.
     Mark each item as "hypothesis" or "confirmed" and explain supporting evidence. -->

## Open Questions

- <!-- question blocking resolution — who must answer it and by when -->

## Promotion Candidates

- [ ] spec-doc needed: <!-- what product intent needs to be formalized -->
- [ ] adr-doc needed: <!-- what architecture decision needs to be recorded -->

## Sources and Relations

<!-- List external research, issue links, user interviews, and upstream discovery artifacts. -->
