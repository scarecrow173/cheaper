---
id: "IMPL-034zQZYpiexj1ILiP9NuIH"
type: "impl"
status: "completed"
title: "Add launcher acquisition contract tests"
created: "2026-10-04"
updated: "2026-10-04"
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
  implements:
    - "../../../docs/tasks/add-launcher-acquisition-contract-tests.md"
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
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Acquisition contract test harness
## Summary
Node-free sh/PE fixtureとHTTP Release stubの新規46casesを追加。
## Context
承認plan/TSPECをtest-firstで実行。
## Implementation
### Changes
launcher-acquisition.test.ts、launcher-fixture.cs。全cache変数とPATHをfixtureに隔離。
### Decisions During Implementation
Git BashのdirnameをPATHへ追加しsuite timeout35秒を明示。fixture probeはstdinを読むためnull接続の欠陥を捉える。
## Related Experiments
### Adopted
なし。
### Rejected
なし。
## Validation
focused -t 'acquires the pinned pair'：2failed/44skipped/exit1。sh/pwsh双方がsibling不在の旧launcher errorで失敗（harness問題は修正済み）。Duration15.64s。
## Risks
全caseのgreenは後続taskで確認。
## Follow-ups
native launcher taskでgreenへ進む。
