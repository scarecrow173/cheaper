---
id: "IDEA-034Aqy9KktpNVENeRoICf7"
type: "idea"
status: "promoted"
title: "One-shot workflow engine and run CLI"
created: "2026-10-05"
updated: "2026-10-05"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/62"
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
  derived-by: ["../discovery/one-shot-workflow-execution-boundaries.md"]
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# One-shot workflow engine and run CLI

## Summary

親 agent が jobs と dataflow を一度指定し、cheaper が実行を完結して公開 outputs を一つの envelope で返す。

## Problem and Motivation

- compiler・resource safety・artifact store は導入済みだが、それらを使う one-shot engine と workflow run CLI はまだ無い。

## Expected Value

- 親 Coding Agent が fan-out/fan-in の polling と中間コンテキスト往復を省ける。

## Open Questions

- 共有 invocation、resource admission、bounded cancellation、artifact consumer をどの境界で結合するか。

## Next Action

- [x] discovery に昇格し、実装境界を調査する。
