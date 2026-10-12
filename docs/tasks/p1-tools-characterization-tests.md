---
id: "TASK-034EDQoBRwn5Bqlb09xwx1"
type: "task"
status: "done"
title: "T07: cheaper-tools characterization tests"
created: "2026-10-12"
updated: "2026-10-12"
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
  implements:
    - "docs/plans/execute-post-feature-structural-refactoring.md"
  implemented-by: []
  depends-on:
    - "docs/plans/execute-post-feature-structural-refactoring.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/post-feature-structural-refactoring-parity.md"
  references: []
  defers: []
  deferred-by: []
---

# T07: cheaper-tools characterization tests

## Work

tools の構造分割に先行して characterization test を追加し、現挙動を pin
する（spec Phase 1・先行必須条件）。

- access observation/support authorization
- operation の state transition・cancellation settlement
- process lifecycle（spawn/monitor/cleanup 順序・drain 分類）
- grep の stop reason
- resource lease の親子 counter

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/execute-post-feature-structural-refactoring.md`
- `docs/specs/post-feature-structural-refactoring-scope.md`
- `docs/test-specs/post-feature-structural-refactoring-parity.md`

## Dependencies

- なし（tools 分割 task T11 の必須先行条件）

## Affected Files

- `crates/cheaper-tools/tests/`（characterization test 追加）

## Done When

- [ ] 対象挙動を pin する test が追加され green
- [ ] 追加のみで既存 src の変更を含まない

## Verification

- [ ] `mise exec -- cargo test -p cheaper-tools` が pass
