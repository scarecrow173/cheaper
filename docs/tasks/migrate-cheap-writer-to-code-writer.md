---
id: "TASK-034r05iHQeQpDpMK3ZMC7C"
type: "task"
status: "done"
title: "Migrate cheap-writer to code-writer"
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
    - "docs/plans/restructure-built-in-writer-workers.md"
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

# Migrate cheap-writer to code-writer

## Work

`cheap-writer` を `code-writer` へ直接置き換える。worker md・input/output schema
ファイルを rename し内容を code-writer 向けに更新、`ALIAS_WORKERS` を
`code-write` へ変更、HELP・fixture・全 test を新名称へ移行する。
`cheap-write` alias・`cheap-writer` fallback は残さない。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/designs/writer-worker-contracts.md`
- `docs/specs/code-writer-and-doc-writer-built-in-workers.md`

## Dependencies

- なし（本チェーンの先頭タスク）

## Affected Files

- `packages/cli/workers/cheap-writer.md` → `code-writer.md`
- `packages/cli/workers/schemas/cheap-writer.input.schema.json` → `code-writer.input.schema.json`
- `packages/cli/workers/schemas/cheap-writer.output.schema.json` → `code-writer.output.schema.json`
- `packages/cli/src/args.ts`、`packages/cli/src/cli.ts`（HELP）
- `packages/cli/test/e2e/fixtures/cheap-write-fake.js` → `code-write-fake.js`
- `packages/cli/test/{args,workers,envelope,text-format}.test.ts`、`test/e2e/exec.test.ts`

## Done When

- [ ] `code-write` alias が `code-writer` worker を解決する
- [ ] `cheap-write` が `INVALID_REQUEST`、`run cheap-writer` が worker 解決失敗になる
- [ ] 旧名称のファイル・文字列が packages/ と README から除去されている

## Verification

- [ ] `mise exec -- pnpm -F cheaper test` が green
