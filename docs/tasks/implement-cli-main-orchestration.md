---
id: "TASK-034r05iHQeQpDpro7GvlKQ"
type: "task"
status: "done"
title: "Implement CLI main orchestration"
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
    - "docs/plans/implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md"
    - "docs/tasks/implement-cli-argument-parsing.md"
    - "docs/tasks/implement-input-reading-from-stdin-or-file.md"
    - "docs/tasks/implement-result-and-error-envelope-output.md"
    - "docs/tasks/implement-human-readable-text-format-output.md"
    - "docs/tasks/add-built-in-worker-definitions.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Implement CLI main orchestration

## Work

PLAN-034r05iHy2gdnpJFiMWhkY step 8。`main.ts` オーケストレーション: args → input → core resolve（Worker/policy/model）→ runtime-pi `runWorker` → envelope → exit code。timeout タイマー（`setTimeout(effectiveTimeoutMs)` → abort）、SIGINT/SIGTERM → abort、未捕捉例外 → `INTERNAL_ERROR` 70 envelope。stderr に model 選択行（`--quiet` で抑止）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md`
- `docs/designs/cli-contract-envelope-exit-codes.md`
- `docs/designs/pi-execution-engine.md`

## Dependencies

- `docs/tasks/implement-cli-argument-parsing.md` — 依存先（Dependency Graph）
- `docs/tasks/implement-input-reading-from-stdin-or-file.md` — 依存先（Dependency Graph）
- `docs/tasks/implement-result-and-error-envelope-output.md` — 依存先（Dependency Graph）
- `docs/tasks/implement-human-readable-text-format-output.md` — 依存先（Dependency Graph）
- `docs/tasks/add-built-in-worker-definitions.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/cli/src/main.ts`
- `packages/cli/test/main.test.ts`

## Done When

- [x] 失敗するテスト（または scaffold 検証コマンド）を書く
- [x] 対象テストを実行し red を確認する
- [x] 最小実装を行う
- [x] 再実行で green を確認する
- [x] `pnpm biome check` と `tsc -b` が pass
- [x] 正常系配線・タイマー・signal・未捕捉例外の各テストが green

## Verification

- [x] `mise exec -- pnpm --filter cheaper test -- test/main.test.ts` exits 0
- [x] `mise exec -- pnpm biome check .` exits 0
- [x] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- bin 経由の e2e は TASK-034r05iHQeQpDpsYVkvsPX/0048。
