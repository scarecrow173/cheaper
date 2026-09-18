---
id: "TASK-034r05iHQeQpDqJKnzydzW"
type: "task"
status: "done"
title: "Implement usage-log writer and CLI wiring with unit tests"
created: "2026-09-18"
updated: "2026-09-18"
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
    - "docs/plans/delegation-usage-measurement-log.md"
  implemented-by: []
  depends-on:
    - "docs/plans/delegation-usage-measurement-log.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/delegation-usage-measurement-log.md"
    - "docs/specs/delegation-usage-measurement-log.md"
    - "docs/adr/usage-jsonl-in-user-config-dir-best-effort-append.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/delegation-usage-measurement-log.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# TASK-034r05iHQeQpDqJKnzydzW: Implement usage-log writer and CLI wiring

## Work

DESIGN-034r05iHy2gdnoXu6h71XL に従い `packages/cli/src/usage-log.ts` を新設し、
`cli.ts` の run・count-tokens 経路から best-effort append を
呼び出す。TDD で進める。

### 1. `usage-log.ts`（新規）

- `UsageLogRecord` 型（schemaVersion/ts/kind/worker/root/ok/
  durationMs + kind 別 optional field）
- `buildUsageRecord(env, ctx)`: CliEnvelope から record を構築。
  run success・run error（meta.usage あり）・count-tokens を
  識別し、対象外は `null`
- `appendUsageLog({userConfigDir, record})`: `mkdirSync
  {recursive:true}` + `appendFileSync` で 1 行 JSONL を追記

### 2. `cli.ts` 配線

- `executeRun` が workspace `root` を out-param（`workerName` と
  同じ方式）で `runCli` へ返す
- run success・run error（catch 内）・count-tokens の 3 箇所で
  `buildUsageRecord` → `appendUsageLog` を try/catch で呼び、
  失敗時は `quiet` 以外で stderr に `usage-log:` 1 行

### 3. unit test（`test/usage-log.test.ts`）

- success run envelope → `kind:"run"` record
- error envelope（meta.usage あり）→ `ok:false` + `error.code`
- utility envelope（count-tokens 形）→ `kind:"count"` record
- 対象外 envelope → `null`
- `appendUsageLog` が 1 行 JSONL を append・2 回目は追記
- 書き込み不可 dir でも呼出し側が swallow（exit/stdout 不変は
  e2e 側でも確認）

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/delegation-usage-measurement-log.md`
- `docs/designs/delegation-usage-measurement-log.md`
- `docs/specs/delegation-usage-measurement-log.md`
- `docs/adr/usage-jsonl-in-user-config-dir-best-effort-append.md`

## Dependencies

- なし（PLAN-034r05iHE1Fbkpd5tkg6Gs の先頭 task）

## Affected Files

- `packages/cli/src/usage-log.ts`（add）
- `packages/cli/src/cli.ts`（modify）
- `packages/cli/test/usage-log.test.ts`（add）

## Done When

- [x] `usage-log.ts` が DESIGN-034r05iHy2gdnoXu6h71XL の API・record shape を実装
- [x] `cli.ts` の 3 経路（run success/run error/count-tokens）
      から記録される
- [x] unit test が全 pass・lint・typecheck が pass

## Verification

- [x] `vitest run test/usage-log.test.ts` pass（8/8）
- [x] `pnpm typecheck`・`pnpm lint` pass
- [x] 既存 cli test 群に回帰がないこと

## Verification results

- `packages/cli/src/usage-log.ts` 新設: `UsageLogRecord`・
  `buildUsageRecord`・`appendUsageLog`・`recordUsage`（swallow +
  stderr diagnostic をここに集約）。
- `cli.ts`: `executeRun` に `onRunContext` callback を追加し
  root/userConfigDir を runCli へ伝搬。run success・run error
  （catch）・count-tokens の 3 箇所で `recordUsage` を呼ぶ。
- `vitest run test/usage-log.test.ts`: 8/8 pass（record shape・
  append・swallow/quiet）。
- `vitest run test/envelope.test.ts test/main.test.ts`: 回帰なし。
- `pnpm typecheck` pass・`pnpm lint` pass（warning は既存のみ）。
- 補足: `exactOptionalPropertyTypes` のため ctx の `now` は
  `(() => Date) | undefined` で受ける。
