---
id: "TASK-0048"
type: "task"
status: "done"
title: "Add fake stream hook and e2e execution tests"
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
    - "docs/tasks/0047-add-e2e-tests-for-non-execution-cli-paths.md"
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

# Add fake stream hook and e2e execution tests

## Work

PLAN-0003 step 10。runtime-pi の `stream` seam を環境変数 `CHEAPER_TEST_FAKE_STREAM=<fixture path>` で注入する test-only フックを実装（`NODE_ENV=test` 以外では無効化）。bulk-read の成功 envelope と cheap-write の `meta.changes` 実測を e2e 化。非 test 環境で env var を与えても無視されることを assert。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0003-implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md`
- `docs/designs/0004-pi-execution-engine.md`
- `docs/designs/0005-cli-contract-envelope-exit-codes.md`

## Dependencies

- `docs/tasks/0047-add-e2e-tests-for-non-execution-cli-paths.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/cli/test/e2e/cli.test.ts`
- `packages/runtime-pi/src/run.ts`
- `packages/cli/test/fixtures/`

## Done When

- [x] 失敗するテスト（または scaffold 検証コマンド）を書く
- [x] 対象テストを実行し red を確認する
- [x] 最小実装を行う
- [x] 再実行で green を確認する
- [x] `pnpm biome check` と `tsc -b` が pass
- [x] fake stream 経由の成功 envelope・meta.changes・NODE_ENV ガードの e2e が green

## Verification

- [x] `mise exec -- pnpm --filter cheaper test -- test/e2e/cli.test.ts` exits 0
- [x] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- 実 provider 統合は認証環境での手動手順（PLAN-0002 Review Handoff）。
