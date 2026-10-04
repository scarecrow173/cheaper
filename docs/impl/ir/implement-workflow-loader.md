---
id: "IMPL-034zTuWWp7bpifS4FoAiG2"
type: "impl"
status: "completed"
title: "implement-workflow-loader"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: ["packages/workflow/src/loader.ts", "packages/workflow/test/loader.test.ts"]
    modified: ["packages/workflow/src/compiler.ts", "packages/workflow/src/index.ts"]
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
実装前に IR と RED テストを作成し、workspace boundary/deny/symlink/regular-file/JSON の I/O 境界と compiler 委譲を実装した。validateInputs は compile と同じ validateDefinition/invocationInputs を使用し、barrel から loader と型を公開した。

## Validation and Evidence
baseline: workflow suite は 4 files / 75 tests GREEN。新規 loader test は未実装 export/API により RED。実装後 mise exec -- pnpm --filter @cheaper/workflow test は 5 files / 84 tests GREEN、mise exec -- pnpm typecheck は exit 0。差分 review で独自 schema validator がなく、同一 canonical compiler を返し、parse error に private JSON 本文が含まれないことを確認した（P0/P1 なし）。コード commit: 8417f2b。

## Risks or Follow-ups
run/resource inference と bundle は downstream/coordinator の責務。
