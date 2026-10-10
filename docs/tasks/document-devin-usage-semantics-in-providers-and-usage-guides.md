---
id: "TASK-034DlFJnTT9v0YcqV9Uine"
type: "task"
status: "done"
title: "Document devin usage semantics in providers and usage guides"
created: "2026-10-10"
updated: "2026-10-10"
owners: []
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
    - "docs/plans/devin-usage-accounting-contract-test-and-docs.md"
  implemented-by:
    - "docs/impl/ir/devin-usage-accounting-contract-pin-tests-and-guide-notes.md"
  depends-on:
    - "docs/plans/devin-usage-accounting-contract-test-and-docs.md"
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

# Document devin usage semantics in providers and usage guides

## Work

Add the devin usage-accounting note (uncached `input`, `cacheRead`
carries the hit, `totalTokens = input + output`, full prompt volume
= `input + cacheRead`, `costUsd` 0 on all-zero resolved price) to
`docs/guides/providers.md` Devin Local section and a parallel
sentence to `docs/guides/usage.md` next to the `meta.usage` field
list.

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/devin-usage-accounting-contract-test-and-docs.md`
- `docs/specs/devin-provider-usage-accounting-contract.md`

## Dependencies

- none (independent of the stream-tests task)

## Affected Files

- `docs/guides/providers.md` (Devin Local section, before
  `model catalog` bullet)
- `docs/guides/usage.md` (`meta.usage` field list, ~lines 61-62)

## Done When

- [x] `providers.md` Devin Local section documents uncached
      `input` / `cacheRead` split, `totalTokens = input + output`,
      `input + cacheRead` full-prompt-volume guidance, and all-zero
      resolved price → `costUsd: 0`
- [x] `usage.md` meta.usage field docs carry the same semantic in
      one or two sentences

## Verification

- [x] Both notes present and consistent with spec R5; Japanese
      prose matches guide style
