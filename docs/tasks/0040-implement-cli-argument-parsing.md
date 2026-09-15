---
id: "TASK-0040"
type: "task"
status: "todo"
title: "Implement CLI argument parsing"
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
    - "docs/tasks/0039-scaffold-cheaper-cli-package-with-bin-entry.md"
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

# Implement CLI argument parsing

## Work

PLAN-0003 step 2。`util.parseArgs`: commands `run <worker>`/`bulk-read`/`cheap-write`（後者は run のエイリアス）、options `--input`(既定 -)/`--worker-file`（run 専用・worker と併用不可）/`--cwd`/`--timeout`/`--format json|text`/`--quiet`/`--help`/`--version`。path 風 name は `INVALID_WORKER_NAME`。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0003-implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md`
- `docs/designs/0005-cli-contract-envelope-exit-codes.md`

## Dependencies

- `docs/tasks/0039-scaffold-cheaper-cli-package-with-bin-entry.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/cli/src/args.ts`
- `packages/cli/test/args.test.ts`

## Done When

- [ ] 失敗するテスト（または scaffold 検証コマンド）を書く
- [ ] 対象テストを実行し red を確認する
- [ ] 最小実装を行う
- [ ] 再実行で green を確認する
- [ ] `pnpm biome check` と `tsc -b` が pass
- [ ] 各 option・併用不可・path 風 name のテストが green

## Verification

- [ ] `mise exec -- pnpm --filter cheaper test -- test/args.test.ts` exits 0
- [ ] `mise exec -- pnpm biome check .` exits 0
- [ ] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- stdin/ファイル読取は TASK-0041。
