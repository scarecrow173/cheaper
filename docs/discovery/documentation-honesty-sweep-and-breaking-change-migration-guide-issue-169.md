---
id: "DISC-034L67agpaqLudLiaNI822"
type: "discovery"
status: "draft"
title: "documentation honesty sweep and breaking-change migration guide issue-169"
created: "2026-10-23"
updated: "2026-10-23"
owners: ["scarecrow173"]
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
  depends-on:
    - "../designs/permission-model-responsibility-redesign.md"
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

# documentation honesty sweep and breaking-change migration guide issue-169

## Exploration Goal

issue #169 — 権限モデル再設計(#151)分解の最終 slice(7/7)。
「boundary/sandbox/protection」表現と実強制範囲の整合
(claim ≤ 実強制)を guides・refine 対象 ADR/spec から訂正し、
#163–#168 の破壊的変更一覧と移行手順を migration guide に
列挙する。全実装依存(#163–#168)が完了したため着手可能。

実測 inventory(briefing で詳細化):

- 「OS sandbox ではない」「子プロセス fs 副作用は強制不能」
  の明記対象 — canonical file tool の workspace boundary
  (E1/E2)は cheaper 所有 surface で強制可能だが spawn 子
  プロセスの副作用は OS sandbox なしでは強制不能
- spawn API 層の shell-free 記述と ENOEXEC→`/bin/sh` fallback
  (libc 既定動作・保証対象外)の区別
- `security.execDenyPatterns` = deterministic executable
  deny-list の命名規約(「保護」「sandbox」「containment」
  とは呼ばない)
- refine 対象 16 doc(ADR 7 件・spec/design 9 件 — issue 本文
  の §Supersede/Refine 一覧。supersede なし・全件 refine)
- migration guide 収録項: executable deny list 削除・内容
  審査 7 項目除去・統一 grant 形状・**#168 の project 層
  `trusted` key presence 拒否**(修復 = key 削除 1 行 or
  userConfigDir へ vendor — ゼロ編集で shadowing 復活)

## Key Issues and Assumptions

- 「claim ≤ 実強制」の判定には各表現の実装照合が必要
  (掃き出し対象表現の grep inventory + 強制境界の実測)
- refine は文書訂正のみでコード変更を伴わない(コード側の
  不整合が見つかった場合は別 issue 化して本 sweep では
  文書の正直性を優先する)
- migration guide の置き場所・形式は briefing で確定
  (docs/guides/ 配下 or CHANGELOG 系の既存規約に従う)

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
