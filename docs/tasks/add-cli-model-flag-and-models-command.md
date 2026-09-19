---
id: "TASK-034rzWT9V9bHwEIcrmbTor"
type: "task"
status: "done"
title: "Add CLI --model flag and models command"
created: "2026-09-19"
updated: "2026-09-19"
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
    - "docs/plans/implement-model-pricing-catalog.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-model-pricing-catalog.md"
    - "docs/tasks/wire-explicit-model-in-runtime.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/pricing-metadata-and-models-command.md"
  references: []
  defers: []
  deferred-by: []
---

# Add CLI --model flag and models command

## Work

`args.ts` に `--model <provider>/<id>`（run/alias 専用）と
`models` コマンド（`--provider`/`--tier` filter）を追加する。
`cli.ts` で run 経路へ `explicitModel` を渡し、`models` 実行を
実装する。`text-format.ts` に models の text 表示を追加する。
HELP を更新する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-model-pricing-catalog.md`

## Dependencies

- `docs/tasks/wire-explicit-model-in-runtime.md`

## Affected Files

- `packages/cli/src/args.ts`
- `packages/cli/src/cli.ts`
- `packages/cli/src/text-format.ts`
- `packages/cli/test/args.test.ts` ほか cli テスト

## Done When

- [ ] `cheaper run <worker> --model p/m` と alias で直接指定できる
- [ ] `cheaper models` が provider/model・available/excluded・
      pricing・tier・tierSource・pricing source/freshness を出す
- [ ] `--provider`/`--tier` filter が機能する

## Verification

- [ ] `pnpm -F @cheaper/cli test` pass
