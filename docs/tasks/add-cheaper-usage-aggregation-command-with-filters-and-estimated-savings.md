---
id: "TASK-034rzRuCuCgZ08BmHTCqgz"
type: "task"
status: "todo"
title: "Add cheaper usage aggregation command with filters and estimated savings"
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
    - "docs/tasks/extend-usage-jsonl-run-record-with-savings-section.md"
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

# Add cheaper usage aggregation command with filters and estimated savings

## Work

`cheaper usage` 集計コマンドを実装する。usage.jsonl を
read-only で読み、filter・集計し、pricing metadata があれば
estimated savings を別枠で算出する。

### 1. `args.ts`

- `UsageCliArgs { kind:"usage", since?, worker?, provider?,
  model?, compareModel?, format, quiet }` を追加。
- option: `--since <duration>`（parseDuration 形式）・`--worker`
  ・`--provider`・`--model <provider/id>`・`--compare-model
  <provider/id>`。run/auth/count-tokens 専用 option の排他を
  既存ルールに倣って検証する。

### 2. `pricing.ts`（新規）

- `<userConfigDir>/pricing.json` を読む minimal contract:
  `{ "models": { "<provider>/<id>": { inputPerMillionUsd,
  outputPerMillionUsd, checkedAt, source } } }`。
- file 不在・parse 失敗・model 欠損は「pricing 不明」として
  undefined を返す（throw しない）。
- stale 判定: `checkedAt` が 30 日超または欠損/parse 不可なら
  stale。

### 3. `usage-report.ts`（新規）

- `readUsageLog(userConfigDir)`: per-line parse、破損行 skip。
- `filterRecords(records, {since, worker, provider, model})`:
  kind==="run" のみ集計対象。`--since` は `ts` を
  `now - duration` と比較。`--worker` は `worker` 完全一致。
  `--provider`/`--model` は `model.provider` / `provider/id`。
- `aggregate(records)`: runs（ok 別内訳）・usage 合計
  （input/output/cacheRead/cacheWrite/totalTokens/costUsd）・
  turns/toolCalls/durationMs・savings 合計（savings を持つ
  record のみ母数）。
- `estimate(agg, pricing, compareModel)`: entry が存在し非 stale
  のときのみ `{ estimatedParentEquivalentCostUsd,
  estimatedSavingsUsd, estimatedSavingsPercent, pricing:
  { source, checkedAt } }` を返す。式:
  `parentEquiv = (sourceTokens * input + returnedTokens * output)
  / 1e6`、`savings = parentEquiv - workerCostUsd`、
  `percent = parentEquiv > 0 ? savings / parentEquiv * 100 : null`。

### 4. `cli.ts` + `text-format.ts`

- `parsed.kind === "usage"` 分岐を追加。utility envelope
  （`worker: null`）の `result` に集計 JSON を入れて返す。
  `cheaper usage` 自体は recordUsage の対象外（utility command）。
- text 表示: Runs・Worker tokens・Avoided parent context・
  Worker cost を表示し、estimated 行は `Estimated` prefix 付きで
  算出時のみ出す。

### 5. unit test

- readUsageLog: 破損行 skip・旧 record（savings なし）混在・
  file 不在で空集計。
- filter: since/worker/provider/model。
- aggregate: usage・savings 合計、ok:false の扱い。
- estimate: pricing 不在・model 不明・stale で非出力、正常系で
  3 field + pricing provenance。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/usage-savings-telemetry-implementation-plan.md`
- `docs/designs/usage-savings-telemetry.md`
- `docs/specs/usage-and-savings-telemetry-aggregation.md`

## Dependencies

- `docs/tasks/extend-usage-jsonl-run-record-with-savings-section.md`
  （record の `savings` field shape）

## Affected Files

- `packages/cli/src/args.ts`（modify）
- `packages/cli/src/pricing.ts`（add）
- `packages/cli/src/usage-report.ts`（add）
- `packages/cli/src/cli.ts`（modify）
- `packages/cli/src/text-format.ts`（modify）
- `packages/cli/test/usage-report.test.ts`・`pricing.test.ts`（add）

## Done When

- [ ] `cheaper usage` が filter 付きで集計を出力する
- [ ] estimated は 3 条件（compare-model・pricing entry・非
      stale）揃ったときのみ出力される
- [ ] unit test pass・typecheck・lint pass

## Verification

- [ ] `pnpm --filter @cheaper/cli test` pass
- [ ] `pnpm typecheck`・`pnpm lint` pass
