---
id: "DESIGN-034rzNZ2BZGTPuOwHmyqh4"
type: "design"
status: "approved"
title: "Usage savings telemetry"
created: "2026-09-19"
updated: "2026-09-19"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/usage-and-savings-telemetry-aggregation.md"
    - "../adr/measured-offload-tool-output-token-estimated-savings-pricing-seam.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/usage-and-savings-telemetry-aggregation.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../specs/usage-and-savings-telemetry-aggregation.md"
    - "../adr/measured-offload-tool-output-token-estimated-savings-pricing-seam.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Usage savings telemetry

## Context

SPEC-034rzGnCaRQc5ceI95lf5i / ADR-034rzGnXqru47VgHNL16cS の実装設計。
worker run の measured offload（tool output token 和 vs 返却
result token 和）を run 中に計測し、envelope `meta` と
`usage.jsonl` record に載せる。`cheaper usage` で usage.jsonl を
集計し、pricing metadata があれば comparison model 基準の
estimated savings を別枠表示する。

## Scope

### In Scope

- `packages/core/src/token/count.ts`: 文字列を canonical token
  化する `countTextTokens(text)` を公開（encoding は module
  scope の既存 instance を再利用）。
- `packages/runtime-pi/src/tools/meter.ts`（新規）: tool output
  の token/byte を集計する wrap。`createSandboxedTools` で
  `wrapToolBudget` と同じ層に挟む。
- `packages/runtime-pi/src/run.ts` / `usage.ts`: `WorkerRunMeta`
  に `savings` を追加。success 経路で
  `{ sourceTokens, returnedTokens, avoidedContextTokens }` を
  組み立てる。error 経路では result が無いため
  `returnedTokens: 0`・`avoidedContextTokens: sourceTokens`
  として載せる（途中まで offload した実測として意味を持つ）。
- `packages/cli/src/usage-log.ts`: run record に optional
  `savings` を転写（`meta.savings` があれば）。count record は
  対象外。
- `packages/cli/src/args.ts` / `cli.ts`: `usage` command。
  `--since <duration>`・`--worker`・`--provider`・`--model`・
  `--compare-model <provider/id>` を parse し、usage.jsonl を
  集計して envelope（`kind` なし utility envelope ではなく
  専用 result）を返す。
- `packages/cli/src/text-format.ts`: usage 集計の text 表示
  （Estimated 行は `Estimated` prefix 必須）。
- `packages/cli/src/pricing.ts`（新規）:
  `<userConfigDir>/pricing.json` を core `PricingSnapshot`
  contract の bridge として読み取る（stale 判定は
  `usage-report.ts` 側）。
- `docs/guides/usage.md`: 指標定義・限界・`usage` command。

### Out of Scope

- pricing metadata の取得・更新（#45 の scope。ここでは
  core `resolvePricing` への接続と `pricing.json` bridge のみ）。
- usage.jsonl の rotation、per-tool 内訳、記録対象の拡大。
- `count` record への savings（offload の概念がない）。

## Components and Boundaries

- **core**: `countTextTokens` — tokenizer 再利用の純粋関数。
  新規依存なし。
- **runtime-pi**: `ToolMeter`（tool output → sourceTokens 集計）、
  `run.ts` の `buildMeta` で `meta.savings` を組み立て。
  `returnedTokens` は result 確定後に result JSON を encode して
  算出（error 時は 0）。
- **cli**: `usage-log.ts` は record へ `savings` を転写するだけ。
  `usage` command は `usage-report.ts`（新規）に read/aggregate/
  format を集約し `cli.ts` は薄い分岐のみ。

## Data and Control Flow

1. run: tool execute の戻り値 `content[].text` を meter が
   encode して `sourceTokens` に加算（bytes も保持するが record
   には出さない — token のみ）。
2. run 終了時: `buildMeta` が `savings = { sourceTokens,
   returnedTokens: countTextTokens(JSON.stringify(result)),
   avoidedContextTokens: max(0, source - returned),
   directlyComparable: isDirectlyComparableWorker(worker) }` を
   返す。error 経路では `returnedTokens: 0`。
3. recordUsage: `meta.savings` を record.savings へ転写。
4. `cheaper usage`: `<userConfigDir>/usage.jsonl` を per-line
   parse（破損行 skip）→ filter → 集計 → envelope result または
   text 出力。`--compare-model` 指定時は runtime catalog の
   `model.cost` → `pricing.json` snapshot → bundled snapshot の
   順に core `resolvePricing` で価格を解決し、stale でなく
   `directlyComparable: true` の run があれば comparable subset
   のみを母数に estimated を算出。

## Non-Functional Constraints

- tool output の encode は run 中の per-call コスト（tool output
  は既に 50KB 上限等で bounded）。tokenizer は module scope の
  共有 instance を使い初期化コストを増やさない。
- usage.jsonl の読みは集計時のみで、run 経路の latency に影響
  しない。記録は従来通り best-effort。
- envelope `meta.savings` は optional field 追加 — 既存
  consumer は無視できる。

## Risks and Trade-offs

- `sourceTokens` は tool output の和であり grep listing・error
  text を含む。「親 context に投入されなかった token 数」の
  近似として文書で明示し、厳密な file 内容量とは区別する。
- error run の `avoidedContextTokens = sourceTokens` は「処理を
  途中まで offload した」実測であり、過大表示ではないが、
  guide で error run の扱いを明記する。
- `pricing.json` は core `PricingSnapshot` と同一 schema の
  user override bridge に限定する（独自 contract は持たない）。
- `directlyComparable` の判定は worker 定義（`writeScope`・
  mutating tool の有無）に基づく構造的判定であり、worker 名の
  hardcode ではない。

## Deferred Design Concerns

- per-tool / per-file の内訳 | 合計で要件を満たす | 詳細分析の
  需要 | 内訳不明のまま
- stale の閾値変更を config 化 | 30 日固定で十分 | 運用で要望が
  出た時点 | 閾値を変えられない
- usage.jsonl の rotation | 別 issue | log 肥大化時 | 長期で
  file 肥大

## Verification Notes

- [ ] fake-stream e2e で bulk-reader の `meta.savings` 3 field が
      実測される
- [ ] `cheaper usage` が旧 record 混在の log を集計できる
- [ ] pricing 不在/不明/stale で estimated 非出力
- [ ] `mise run check` pass
