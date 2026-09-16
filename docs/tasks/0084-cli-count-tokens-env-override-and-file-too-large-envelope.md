---
id: "TASK-0084"
type: "task"
status: "todo"
title: "CLI count-tokens env override and FILE_TOO_LARGE envelope"
created: "2026-09-17"
updated: "2026-09-17"
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
    - "docs/plans/0012-huge-file-byte-cap-guard-and-fail-closed-hook.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/0083-core-byte-cap-for-countfiletokens.md"
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

# CLI count-tokens env override and FILE_TOO_LARGE envelope

## Work

`count-tokens` 分岐で `CHEAPER_COUNT_MAX_BYTES` env を解釈し
（正の整数のみ有効・不正値は default）、`countFileTokens` の
`maxBytes` に渡す。`FileTooLargeError` を
`CheaperError("FILE_TOO_LARGE", ..., {details: {path, bytes, maxBytes}})`
に map する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0012-huge-file-byte-cap-guard-and-fail-closed-hook.md`
- `docs/designs/0017-huge-file-byte-cap-guard-and-fail-closed-hook.md`

## Dependencies

- `docs/tasks/0083-core-byte-cap-for-countfiletokens.md`

## Affected Files

- `packages/cli/src/cli.ts`
- `packages/cli/test/`（env 解釈・envelope の test）

## Done When

- [ ] cap 超過 file で `count-tokens` が `FILE_TOO_LARGE` code の
      error envelope を返し非0 exit する
- [ ] `CHEAPER_COUNT_MAX_BYTES` の上書き・不正値フォールバックが
      動作する

## Verification

- [ ] `pnpm --filter cheaper test` が pass
- [ ] 小さな env cap 値で FILE_TOO_LARGE envelope を検証
