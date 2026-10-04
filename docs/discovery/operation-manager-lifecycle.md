---
id: "DISC-034zSONEtz7Sfeq2UvbW3a"
type: "discovery"
status: "resolved"
title: "OperationManager の cancel と settle の境界"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/56"
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
  derives-from: ["../ideas/operation-manager-lifecycle.md"]
  derived-by: ["../specs/operation-manager-lifecycle.md", "../adr/operation-manager-settlement.md"]
  verifies: []
  verified-by: []
  references: ["../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
  defers: []
  deferred-by: []
---

# OperationManager の cancel と settle の境界

## Exploration Goal

bounded drain と、cancel 要求だけでは実行停止を証明しないという D3 を同時に満たす境界を調べる。
Entry Decision: A-4。issue と D1/D3/D7 が目的・契約を固定し、未 settle の分類のみ詳細化する。
Dispatch-specific override: 明示された chain を直接実行し、profile routing と sub-worker dispatch は使わない。
Active stack: idea-doc → discovery-doc → spec-doc → design-doc。既存 accepted ADR を再利用する。

## Key Issues and Assumptions

- 確認済み: registry.ts の drain は grace 超過で pending を clear し、遅延 settle を停止と区別する record がない。
- 確認済み: invocation.ts は scope の signal と root abort を合成し、borrowed scope disposal は sibling を止めない。
- 確認済み: run.ts は timeout/signal/maxTurns/maxToolCalls を first-cause で分類するが signal.reason を渡していない。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| cancel 直後に terminal | 単純 | 実停止を偽装し D3 違反 | 却下 |
| 全 settle を無期限に待つ | terminal が正確 | #63 finite return 違反 | 却下 |
| grace 超過を明示分類して runtime ownership を手放す | bounded return と実停止観測を両立 | detached な外部処理の副作用は止まらない | 推奨候補 |

## Tentative Conclusions and Hypotheses

仮説: running state を実 settle まで維持し、grace 超過は settlement=unsettled と owned=false で分類できる。遅延 settle で初めて terminal を確定する。
共有 ADR D1/D3/D7 の契約変更は不要。graph が ADR artifact を要求するため、coordinator 許可に基づき局所的な表現の細化を ADR に記録する。

## Open Questions

- coordinator が spec/design approval で未 settle 分類を承認済み。実装前の未解決要件はない。

## Promotion Candidates

- [x] spec-doc: 操作状態・cancel・drain の観測契約。
- [x] scoped ADR: cancel/settle/ownership の表現。共有 accepted ADR は references で参照する。

## Sources and Relations

#56 Triage 2026-10-04、共有 ADR D1/D3/D7、packages/tool-runtime/src/{registry,invocation}.ts、packages/runtime-pi/src/run.ts。
