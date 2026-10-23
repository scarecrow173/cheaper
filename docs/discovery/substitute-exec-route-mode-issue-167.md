---
id: "DISC-034KWGjJMiSKU5wBxcznho"
type: "discovery"
status: "active"
title: "substitute-exec route mode issue-167"
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

# substitute-exec route mode issue-167

## Exploration Goal

GitHub issue #167（権限モデル再設計 5/7: substitute-exec route mode）の実装検討。
route が `substitute:{grant,argv}` を宣言できる substitute-exec mode — 非 exec
routing deferred sketch（ADR-034KtTzOJGqmzmBW3SwGp5）の採用要素。precheck の
strict さを保ったまま「宣言通り実行する command」を検査する。depends on #166
（grant-set 投影・record split・viability 分割 = 完了済み）。

## Key Issues and Assumptions

- route `substitute:{grant,argv}` 宣言の schema 追加
- viability の検査対象を原 argv でなく substitute ペアへ retarget
- `expected_input` を `{name:substitute.grant,args:substitute.argv}` へ再 anchor（D11 byte 一致は substitute argv へ）
- exit mirror は「実行した command の exit」（substitute 宣言あり → substitute exit）
- grant anchor 規約: `expected_input.name` = `substitute.grant`、lifecycle bound は `substitute.grant` 宣言値、usage record route 帰属は `route.grant` anchor、`executed` field が substitute argv を記録
- **Option E（exec-less route）は範囲外** — open 事項として明示（author 宣言 exit の信用 = 成功偽装 hole 未解消）
- trusted 層限定は #166 で確立した層境界に準拠するか要確認

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
