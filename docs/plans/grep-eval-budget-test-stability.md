---
id: "PLAN-034r6qeRCCYRZy0etnrPfy"
type: "plan"
status: "completed"
test-spec-skip: "Verified by the existing grep.test.ts suite itself (the change is test-only); no dedicated test-spec doc authored."
title: "inject explicit eval budget into large-fixture grep tests"
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
    - "docs/specs/grep-eval-budget-test-stability.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/grep-eval-budget-test-stability-budget-injection.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/grep-eval-budget-test-stability.md"
    - "docs/designs/grep-eval-budget-test-stability.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# inject explicit eval budget into large-fixture grep tests Implementation Plan

## Goal

大規模 fixture 用の `bigTool()` factory に実時間で到達し得ない
eval budget を既定として注入し、構造検証テストを環境負荷から
独立させる。

## Scope

単一の実装ストリーム。変更対象は `grep.test.ts` の `bigTool()`
factory 1箇所のみで、production code・fixture・assertion・
`fakeClock()` テスト・`tool()` factory はすべて対象外。

## Upstream Documents

- docs/specs/grep-eval-budget-test-stability.md
- docs/adr/grep-eval-budget-test-stability-budget-injection.md

## Design Inputs

- docs/designs/grep-eval-budget-test-stability.md

## File Map

- `packages/runtime-pi/test/tools/grep.test.ts` - `bigTool()`
  factory に `evalPerFileMs` / `evalTotalMs` の既定値を追加

## Implementation Sequence

- [x] `bigTool()` の `createGrepTool` 呼び出しに `evalPerFileMs`
  と `evalTotalMs` の十分な既定値を `...opts` の前に追加
- [x] runtime-pi の grep test suite を実行し、大規模 fixture
  テストが `[skipped: timeout]` なく pass することを確認
- [x] `mise run check` を実行

## Dependencies and Risks

- なし。test-only 変更で既存テストの意図を変えない

## Task Breakdown

- 単一タスク: budget 注入 + 検証

## Verification

- [x] `mise exec -- pnpm --filter @cheaper/runtime-pi test` で
  grep test を含む suite が pass
- [x] 大規模テストの出力に `[skipped: timeout]` が無い
- [x] `mise run check` が pass

## Open Questions / Gaps

- なし

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution
- Independent work streams: none
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 変更は1行〜数行の factory 注入のみ。budget 値の根拠
  （vitest timeout 120s を上回る有限値）は design の
  Components and Boundaries を参照
