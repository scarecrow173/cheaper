---
id: "TASK-034rzRugER0y7cQ2tlWjPp"
type: "task"
status: "todo"
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
    - "docs/plans/usage-savings-telemetry-implementation-plan.md"
  implemented-by: []
  depends-on:
    - "docs/plans/usage-savings-telemetry-implementation-plan.md"
    - "docs/tasks/measure-tool-output-tokens-and-expose-meta-savings.md"
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

# Extend usage.jsonl run record with savings section

## Work

`usage.jsonl` の run record に optional `savings` section を追加し、
envelope `meta.savings` を転写する。記録対象条件・
`schemaVersion`・既存 field は不変（ADR-034rzGnXqru47VgHNL16cS
option i）。

### 1. `usage-log.ts`

- `UsageLogRecord` に `savings?: { sourceTokens: number;
  returnedTokens: number; avoidedContextTokens: number }` を追加。
- `buildUsageRecord` で run record（success + usage-bearing
  error）に限り `env.meta.savings` を転写。count record には
  付けない。
- `meta.savings` が無い envelope（旧 runtime 等）では field を
  出さない。

### 2. unit test（`test/usage-log.test.ts`）

- `meta.savings` 付き success envelope → record.savings 転写。
- error envelope（meta.savings 付き）→ record.savings 転写。
- `meta.savings` なし → field なし。
- count record → savings なし。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/usage-savings-telemetry-implementation-plan.md`
- `docs/designs/usage-savings-telemetry.md`
- `docs/specs/usage-and-savings-telemetry-aggregation.md`
- `docs/adr/usage-jsonl-contract-canonical.md`

## Dependencies

- `docs/tasks/measure-tool-output-tokens-and-expose-meta-savings.md`
  （`meta.savings` の供給元）

## Affected Files

- `packages/cli/src/usage-log.ts`（modify）
- `packages/cli/test/usage-log.test.ts`（modify）

## Done When

- [ ] run record に `savings` が optional field として記録される
- [ ] 既存 record field・記録対象条件が不変
- [ ] unit test pass

## Verification

- [ ] `vitest run test/usage-log.test.ts` pass
- [ ] `pnpm typecheck`・`pnpm lint` pass
