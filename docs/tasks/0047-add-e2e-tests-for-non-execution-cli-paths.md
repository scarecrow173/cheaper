---
id: "TASK-0047"
type: "task"
status: "done"
title: "Add e2e tests for non-execution CLI paths"
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
    - "docs/tasks/0046-implement-cli-main-orchestration.md"
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

# Add e2e tests for non-execution CLI paths

## Work

PLAN-0003 step 9。`node bin/cheaper.js` を `execFile` で起動する e2e（非実行系）: `--help`/`--version` exit 0、stdin 無し → `INVALID_REQUEST` exit 2 で stdout が envelope 1 行のみ、不正 worker 名 → exit 3、`--timeout` 拡張 → `POLICY_VIOLATION` exit 4、未認証環境 → `MODEL_TIER_UNAVAILABLE` exit 5。stdout を `JSON.parse` して 1 オブジェクト + 末尾改行 1 つを assert。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0003-implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md`
- `docs/designs/0005-cli-contract-envelope-exit-codes.md`
- `docs/specs/0001-cheaper-v1-worker-runtime-cli.md`

## Dependencies

- `docs/tasks/0046-implement-cli-main-orchestration.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/cli/test/e2e/cli.test.ts`

## Done When

- [x] 失敗するテスト（または scaffold 検証コマンド）を書く
- [x] 対象テストを実行し red を確認する
- [x] 最小実装を行う
- [x] 再実行で green を確認する
- [x] `pnpm biome check` と `tsc -b` が pass
- [x] 上記全ケースの e2e が green

## Verification

- [x] `mise exec -- pnpm --filter cheaper test -- test/e2e/cli.test.ts` exits 0
- [x] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- 実行系 e2e（fake stream 注入）は TASK-0048。
