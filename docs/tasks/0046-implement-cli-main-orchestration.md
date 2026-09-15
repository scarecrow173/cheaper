---
id: "TASK-0046"
type: "task"
status: "todo"
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
    - "docs/plans/0003-implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0003-implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md"
    - "docs/tasks/0040-implement-cli-argument-parsing.md"
    - "docs/tasks/0041-implement-input-reading-from-stdin-or-file.md"
    - "docs/tasks/0042-implement-result-and-error-envelope-output.md"
    - "docs/tasks/0043-implement-human-readable-text-format-output.md"
    - "docs/tasks/0045-add-built-in-worker-definitions.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/0003-implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Implement CLI main orchestration

## Work

PLAN-0003 step 8。`main.ts` オーケストレーション: args → input → core resolve（Worker/policy/model）→ runtime-pi `runWorker` → envelope → exit code。timeout タイマー（`setTimeout(effectiveTimeoutMs)` → abort）、SIGINT/SIGTERM → abort、未捕捉例外 → `INTERNAL_ERROR` 70 envelope。stderr に model 選択行（`--quiet` で抑止）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0003-implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md`
- `docs/designs/0005-cli-contract-envelope-exit-codes.md`
- `docs/designs/0004-pi-execution-engine.md`

## Dependencies

- `docs/tasks/0040-implement-cli-argument-parsing.md` — 依存先（Dependency Graph）
- `docs/tasks/0041-implement-input-reading-from-stdin-or-file.md` — 依存先（Dependency Graph）
- `docs/tasks/0042-implement-result-and-error-envelope-output.md` — 依存先（Dependency Graph）
- `docs/tasks/0043-implement-human-readable-text-format-output.md` — 依存先（Dependency Graph）
- `docs/tasks/0045-add-built-in-worker-definitions.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/cli/src/main.ts`
- `packages/cli/test/main.test.ts`

## Done When

- [ ] 失敗するテスト（または scaffold 検証コマンド）を書く
- [ ] 対象テストを実行し red を確認する
- [ ] 最小実装を行う
- [ ] 再実行で green を確認する
- [ ] `pnpm biome check` と `tsc -b` が pass
- [ ] 正常系配線・タイマー・signal・未捕捉例外の各テストが green

## Verification

- [ ] `mise exec -- pnpm --filter cheaper test -- test/main.test.ts` exits 0
- [ ] `mise exec -- pnpm biome check .` exits 0
- [ ] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- bin 経由の e2e は TASK-0047/0048。
