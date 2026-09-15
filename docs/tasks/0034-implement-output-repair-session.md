---
id: "TASK-0034"
type: "task"
status: "done"
title: "Implement output repair session"
created: "2026-09-15"
updated: "2026-09-15"
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
    - "docs/plans/0002-implement-cheaper-runtime-pi-execution-engine-and-sandboxed-tools.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0002-implement-cheaper-runtime-pi-execution-engine-and-sandboxed-tools.md"
    - "docs/tasks/0033-implement-final-output-extraction-and-schema-validation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/0002-implement-cheaper-runtime-pi-execution-engine-and-sandboxed-tools.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Implement output repair session

## Work

PLAN-0002 step 13。repair session: `noTools: "all"`・customTools なし・同 model・systemPrompt = schema 適合 JSON のみ返す指示 + schema。user prompt = 前回 raw output + validation errors。`repairAttempts` 回まで、尽きれば `INVALID_OUTPUT`（details: Ajv errors 先頭 20 件・rawOutput 先頭 2000 文字）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0002-implement-cheaper-runtime-pi-execution-engine-and-sandboxed-tools.md`
- `docs/designs/0004-pi-execution-engine.md`

## Dependencies

- `docs/tasks/0033-implement-final-output-extraction-and-schema-validation.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/runtime-pi/src/repair.ts`
- `packages/runtime-pi/test/repair.test.ts`

## Done When

- [x] 失敗するテスト（または scaffold 検証コマンド）を書く
- [x] 対象テストを実行し red を確認する
- [x] 最小実装を行う
- [x] 再実行で green を確認する
- [x] `pnpm biome check` と `tsc -b` が pass
- [x] tool なし session であること・試行回数・INVALID_OUTPUT details のテストが green

## Verification

- [x] `mise exec -- pnpm --filter @cheaper/runtime-pi test -- test/repair.test.ts` exits 0
- [x] `mise exec -- pnpm biome check .` exits 0
- [x] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- repair の起動条件と meta.repairs 計上は run（TASK-0037）。
