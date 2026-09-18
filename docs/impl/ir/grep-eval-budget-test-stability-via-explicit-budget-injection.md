---
id: "IMPL-swcR6qnZBfKwyIdckqQxae"
type: "impl"
status: "completed"
title: "grep eval-budget test stability via explicit budget injection"
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
    - "docs/tasks/inject-eval-budget-bigtool.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/grep-eval-budget-test-stability.md"
    - "docs/adr/grep-eval-budget-test-stability-budget-injection.md"
    - "docs/designs/grep-eval-budget-test-stability.md"
    - "docs/plans/grep-eval-budget-test-stability.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/grep-eval-budget-test-stability.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# grep eval-budget test stability via explicit budget injection

## Summary

`packages/runtime-pi/test/tools/grep.test.ts` の大規模 fixture 用
`bigTool()` factory に `evalPerFileMs: 600_000` /
`evalTotalMs: 600_000` を既定値として注入し、16MB/100MB fixture
を使う構造検証テストが production の実時間 eval budget に依存
しなくなった（issue #39）。

## Context

- task: docs/tasks/inject-eval-budget-bigtool.md
- plan: docs/plans/grep-eval-budget-test-stability.md
- spec: docs/specs/grep-eval-budget-test-stability.md
- ADR: docs/adr/grep-eval-budget-test-stability-budget-injection.md
- design: docs/designs/grep-eval-budget-test-stability.md

大規模 fixture テストは production 既定の per-file 5s budget を
継承しており、高負荷環境で 100MB 走査の累積 eval 時間が 5s を
超えると file が timeout skip されて flake していた。

## Implementation

### Changes

- `bigTool(opts)` の `createGrepTool` 呼び出しに
  `evalPerFileMs: 600_000` と `evalTotalMs: 600_000` を `...opts`
  の前に追加。caller は引き続き `opts` で override 可能
- 変更は test file 内の factory 1箇所のみ。production code・
  fixture 生成・assertion は不変

### Decisions During Implementation

- budget 値は vitest test timeout（120s）を大きく上回る有限値の
  600_000ms とした。「機構の無効化」ではなく「実時間で到達
  し得ない予算」として表現し、万一の暴走は vitest timeout が
  最終防御になる
- 注入先を `tool()` ではなく `bigTool()` に限定した。`tool()`
  経由のテストには既定 budget 依存の挙動テスト・`fakeClock()`
  注入テストが混在するため

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Validation

- `mise exec -- pnpm --filter @cheaper/runtime-pi test`:
  28 file / 315 test 全 pass。`grep.test.ts` 42 test 全 pass
  （`>100MB file` tail match / `100MB file` bounded heap を含む）。
  `[skipped: timeout]` の出力なし
- `mise run check`: lint・version・doc-id・typecheck・全 package
  test・bundle freshness すべて pass

## Risks

- 大規模テストが regex eval 時間の異常増大を検出しなくなるが、
  eval budget 機構自体は `fakeClock()` 注入の決定的テストで
  カバー済みのため実害はない

## Follow-ups

- なし
