---
id: "TASK-034r05iHQeQpDpOWHdF7Wo"
type: "task"
status: "done"
title: "Add doc-writer built-in Worker"
created: "2026-09-16"
updated: "2026-09-16"
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
    - "docs/plans/restructure-built-in-writer-workers.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/migrate-cheap-writer-to-code-writer.md"
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

# Add doc-writer built-in Worker

## Work

DESIGN-034r05iHy2gdnoDFAeW1DN の contract に従い `doc-writer` を追加する。worker md・
`doc-writer.input.schema.json`（`task`/`documentType`/`targets`/`decisions`/
`facts`/`references`/`constraints`）・`doc-writer.output.schema.json`
（`summary`/`notes`/`unresolved`）を作成し、`doc-write` alias を追加。
指示は「親が決め、writer が構造化する」分担と、判断創作禁止・`unresolved` 返却を
明示する。test を追加する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/designs/writer-worker-contracts.md`
- `docs/specs/code-writer-and-doc-writer-built-in-workers.md`

## Dependencies

- TASK-034r05iHQeQpDpMK3ZMC7C（同一ファイル `args.ts`・HELP・workers test を触るため直列）

## Affected Files

- `packages/cli/workers/doc-writer.md`（新規）
- `packages/cli/workers/schemas/doc-writer.input.schema.json`（新規）
- `packages/cli/workers/schemas/doc-writer.output.schema.json`（新規）
- `packages/cli/src/args.ts`、`packages/cli/src/cli.ts`（HELP）
- `packages/cli/test/{args,workers}.test.ts`、`test/e2e/exec.test.ts`

## Done When

- [ ] `doc-write` alias が `doc-writer` worker を解決する
- [ ] input schema が DESIGN-034r05iHy2gdnoDFAeW1DN の field 定義に一致する
- [ ] worker instructions が判断創作禁止と `unresolved` 返却を明示する

## Verification

- [ ] `mise exec -- pnpm -F cheaper test` が green
