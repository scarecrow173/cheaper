---
id: "TASK-034r6roOdgb1mKbE0LLAv3"
type: "task"
status: "done"
title: "inject generous eval budget into bigTool factory"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/39"
  changes:
    added: []
    modified:
      - type: "file"
        path: "packages/runtime-pi/test/tools/grep.test.ts"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/grep-eval-budget-test-stability.md"
  implemented-by: []
  depends-on:
    - "docs/plans/grep-eval-budget-test-stability.md"
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

# inject generous eval budget into bigTool factory

## Work

`packages/runtime-pi/test/tools/grep.test.ts` の `bigTool()` factory
の `createGrepTool` 呼び出しに、実時間で到達し得ない eval budget
（`evalPerFileMs` / `evalTotalMs`、vitest timeout 120s を上回る
有限値）を `...opts` の前に既定値として追加する。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/grep-eval-budget-test-stability.md
- docs/designs/grep-eval-budget-test-stability.md
- docs/specs/grep-eval-budget-test-stability.md
- docs/adr/grep-eval-budget-test-stability-budget-injection.md

## Dependencies

- なし

## Affected Files

- `packages/runtime-pi/test/tools/grep.test.ts`

## Done When

- [x] `bigTool()` 経由で生成される tool が明示注入された eval
  budget を使い、caller の `opts` で override 可能
- [x] production 既定値・`fakeClock()` テスト・fixture は不変

## Verification

- [x] `mise exec -- pnpm --filter @cheaper/runtime-pi test` で
  grep test を含む suite が pass
- [x] 大規模 fixture テストの出力に `[skipped: timeout]` が無い
- [x] `mise run check` が pass
