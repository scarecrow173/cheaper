---
id: "TASK-034r05iHQeQpDqc1Novy9f"
type: "task"
status: "done"
title: "Core byte cap for countFileTokens"
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
    - "docs/plans/huge-file-byte-cap-guard-and-fail-closed-hook.md"
  implemented-by: []
  depends-on: []
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

# Core byte cap for countFileTokens

## Work

`countFileTokens` に `maxBytes` option（default 16MB）を追加し、
`resolveWorkspacePath` 後・`readFile` 前に `stat` で size を確認、
超過時は `FileTooLargeError` を投げる。`errors.ts` に
`FILE_TOO_LARGE: {exitCode: 2, retryable: false}` を追加し、
`FileTooLargeError` を package export 面へ出す。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/huge-file-byte-cap-guard-and-fail-closed-hook.md`
- `docs/designs/huge-file-byte-cap-guard-and-fail-closed-hook.md`

## Dependencies

- なし（chain の先頭 task）

## Affected Files

- `packages/core/src/token/count.ts`
- `packages/core/src/errors.ts`
- `packages/core/src/index.ts`（export 面）
- `packages/core/test/token/count.test.ts`

## Done When

- [ ] `size > maxBytes` で `FileTooLargeError`（`{path, bytes, maxBytes}`
      保持）が投げられ `readFile`/`encode` が実行されない
- [ ] `maxBytes` 省略時は `DEFAULT_COUNT_MAX_BYTES`（16MB）が使われる
- [ ] cap 未満 file の count 結果が従来と一致する

## Verification

- [ ] `pnpm --filter @cheaper/core test` が pass
- [ ] 小さな `maxBytes` で cap 発火を unit test で検証
