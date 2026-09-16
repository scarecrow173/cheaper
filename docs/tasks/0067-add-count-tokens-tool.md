---
id: "TASK-0067"
type: "task"
status: "done"
title: "Add count_tokens tool to runtime-pi registry and built-in workers"
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
    - "docs/plans/0007-canonical-token-counter.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/0066-implement-core-token-counter.md"
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

# Add count_tokens tool to runtime-pi registry and built-in workers

## Work

runtime-pi `tools/count-tokens.ts` に `{ path }` parameters を持つ
`count_tokens` ToolDefinition を実装し、`createSandboxedTools` の
`{ root, symlinkPolicy }` で core `countFileTokens` を呼ぶ。registry の switch に
case を追加し、builtin `policy.yaml` の `tools.allow` と 3 つの built-in Worker
frontmatter（bulk-reader・code-writer・doc-writer）の `tools:` に
`count_tokens` を追加する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/designs/0012-token-counter-contract.md`

## Dependencies

- TASK-0066

## Affected Files

- `packages/runtime-pi/src/tools/count-tokens.ts`（新規）
- `packages/runtime-pi/src/tools/registry.ts`
- `packages/runtime-pi/test/tools/count-tokens.test.ts`（新規）
- `packages/cli/policy.yaml`
- `packages/cli/workers/{bulk-reader,code-writer,doc-writer}.md`

## Done When

- [ ] `count_tokens` tool が `{ path }` を受け `{ path, tokens, bytes, tokenizer }` を返す
- [ ] capability 交差で builtin worker から tool が有効になる
- [ ] workspace escape が tool error になる

## Verification

- [ ] `mise exec -- pnpm -F @cheaper/runtime-pi test` が green
