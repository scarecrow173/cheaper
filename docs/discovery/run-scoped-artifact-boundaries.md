---
id: "DISC-034zSQKiU63uXPcMSjjodn"
type: "discovery"
status: "resolved"
title: "Run-scoped artifact boundaries"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/66"]
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: ["IDEA-034zSPC3SGPGIHidq3x7sp"]
  derived-by: ["SPEC-034zSQRLxNXgJDxjmUBBB8", "ADR-034zTe2Gt1o8oAkX43ilEm"]
  verifies: []
  verified-by: []
  references: ["ADR-034zGZpjoQlppwkkOs5gur"]
  defers: []
  deferred-by: []
---

# Run-scoped artifact boundaries

## Exploration Goal

invocation-local の容量制限と borrowed reader の破棄を、filesystem と混同せず提供する境界を調べる。

## Context and Constraints

Entry Decision A-4: brief の acceptance は具体的。dispatch 固有の skill 指示を優先し briefing meta skill の profile/delegation は省略、idea/discovery/spec/design の document skill を直接実行する。
既存 workflow は tool-runtime に依存し、bindings は JsonValue を clone する。RuntimePolicy に artifact cap は無い。共有 ADR D4/D5/D7 が namespace、dependency inference、所有者を定める。

## Alternatives

- in-memory store: filesystem の permission と切り離せる。明示容量 cap が必要。
- temp-file store:大容量には有利だが、host path・cleanup・権限の責務が増える。今回の non-goal と合わない。

## Tentative Conclusions

仮説: in-memory store、opaque run/id、tool-runtime 側の独立 reader port により逆向き依存なしで要件を満たせる。

## Open Questions

実装前 blocker は無し。engine/CLI/adapter 全面配線、#65-B の dependency inference は後続 issue が所有する。

## Promotion Candidates

SPEC-034zSQRLxNXgJDxjmUBBB8 へ要件を昇格。architecture は既存 accepted ADR D4/D5/D7 の互換的細化。2026-10-04: graph の local chain.adr 要求を満たすため coordinator 承認で ADR-034zTe2Gt1o8oAkX43ilEm に選択を記録した。

## Sources

issue #66 の Triage と worker brief、packages/workflow/src/types.ts、packages/tool-runtime/src/read.ts を調査した。
