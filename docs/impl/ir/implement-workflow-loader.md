---
id: "IMPL-034zTuWWp7bpifS4FoAiG2"
type: "impl"
status: "in-progress"
title: "implement-workflow-loader"
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
    - "docs/tasks/implement-workflow-loader.md"
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

# Workflow loader と invocation API

## Summary
承認済み Task A を開始。dispatch-specific override: supervised worker の明示 skill 指示に従い meta profile 探索を省略、inline RED/GREEN・focused verification・code review・Conventional Commit を実行する。既知の compiler adapter なので exp は不要。

## Changes Made
実装前にこの IR を開いた。loader/API テストを先に追加する。

## Validation and Evidence
baseline: mise exec -- pnpm --filter @cheaper/workflow test は 4 files / 75 tests GREEN。

## Risks or Follow-ups
run/resource inference と bundle は downstream/coordinator の責務。
