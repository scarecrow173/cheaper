---
id: "TASK-034r05iHQeQpDpQjb4veqT"
type: "task"
status: "done"
title: "Implement core Token Counter with js-tiktoken"
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
    - "docs/plans/canonical-token-counter.md"
  implemented-by: []
  depends-on:
    - "docs/plans/canonical-token-counter.md"
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

# Implement core Token Counter with js-tiktoken

## Work

`@cheaper/core` に `js-tiktoken@1.0.21`（pin）を追加し、`src/token/count.ts` に
`countFileTokens`・`TokenizerIdentity`・`FileTokenCount` を実装する。
`resolveWorkspacePath` で boundary/symlink policy を適用し、utf8 lossy decode →
`o200k_base` encode で決定的に count する。`src/index.ts` から export。
`TOOL_NAMES` に `count_tokens` を追加し、version 定数と導入 version の一致を
test する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/designs/token-counter-contract.md`
- `docs/specs/canonical-token-counter.md`
- `docs/adr/canonical-tokenizer-js-tiktoken.md`

## Dependencies

- なし

## Affected Files

- `packages/core/package.json`（js-tiktoken 追加）
- `packages/core/src/token/count.ts`（新規）
- `packages/core/src/index.ts`
- `packages/core/src/tools/vocabulary.ts`
- `packages/core/test/token/count.test.ts`（新規）

## Done When

- [ ] `countFileTokens` が boundary 適用済みの決定的 count を返す
- [ ] `tokenizer` 識別（name/implementation/version）が結果に含まれる
- [ ] `count_tokens` が `TOOL_NAMES` に存在する

## Verification

- [ ] `mise exec -- pnpm -F @cheaper/core test` が green
