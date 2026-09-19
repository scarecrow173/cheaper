---
id: "IMPL-034rArrzNyFlCc39TNLU1X"
type: "impl"
status: "completed"
title: "Extend usage.jsonl run record with savings section"
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
    - "../../tasks/extend-usage-jsonl-run-record-with-savings-section.md"
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
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Extend usage.jsonl run record with savings section

## Summary

`usage.jsonl` の run record に optional `savings` 節を追加し、
error envelope にも nullable `savings` を伝播させた。

## Context

TASK-034rzRugER0y7cQ2tlWjPp。DESIGN-034rzNZ2BZGTPuOwHmyqh4 に
従い、worker run の measured savings を永続化層へ橋渡しする。

## Implementation

### Changes

- `packages/cli/src/envelope.ts`: error envelope の meta に
  `savings: WorkerSavings | null` を追加し、worker error の
  `meta.savings` を伝播。
- `packages/cli/src/usage-log.ts`: run record 型に optional
  `savings` を追加。success / usage-bearing failure の両経路で
  `meta.savings` を record へ転写。count record には付与しない。
- `packages/cli/test/usage-log.test.ts`: savings 転写・旧 record
  互換・count 非付与・best-effort 書き込み失敗時の挙動を検証。

### Decisions During Implementation

- `savings` は optional のままとし、savings を持たない旧 record
  の読み込み互換を維持した。
- 書き込みは引き続き best-effort で、失敗しても stdout / exit
  code に影響しない既存契約を保持した。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `vitest run test/usage-log.test.ts`（packages/cli）pass。
- `pnpm typecheck` / `pnpm lint` pass。

## Risks

なし（schema は additive で後方互換）。

## Follow-ups

なし。
