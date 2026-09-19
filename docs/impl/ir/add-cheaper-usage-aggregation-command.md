---
id: "IMPL-034rArrYQRvXbeSRXD6HQQ"
type: "impl"
status: "completed"
title: "Add cheaper usage aggregation command"
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
    - "../../tasks/add-cheaper-usage-aggregation-command-with-filters-and-estimated-savings.md"
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

# Add cheaper usage aggregation command

## Summary

`cheaper usage` 集計コマンドを追加した。`usage.jsonl` を
read-only で読み、run record の usage / measured savings を集計し、
`--compare-model` + pricing metadata がある場合のみ estimated
savings を出力する。

## Context

TASK-034rzRuCuCgZ08BmHTCqgz。SPEC-034rzGnCaRQc5ceI95lf5i /
DESIGN-034rzNZ2BZGTPuOwHmyqh4 / ADR-034rzGnXqru47VgHNL16cS に従う。
pricing は issue #45 の generic seam として consumer 側のみ実装。

## Implementation

### Changes

- `packages/cli/src/args.ts`: `usage` command と
  `--since` / `--worker` / `--provider` / `--model` /
  `--compare-model` / `--format` / `--quiet` の parse を追加。
- `packages/cli/src/pricing.ts`（新規）:
  `<userConfigDir>/pricing.json` の読み込み、`checkedAt` の 30 日
  stale 判定。
- `packages/cli/src/usage-report.ts`（新規）: JSONL 読み込み
  （malformed 行 skip・missing = empty）、`kind:"run"` のみ集計、
  filter、actual usage totals / turns / toolCalls / durationMs /
  measured savings totals、estimated 計算（fresh な pricing +
  compare model が揃う場合のみ、provenance 付き）。
- `packages/cli/src/cli.ts`: `usage` 分岐。utility-style envelope
  で `result` に集計値を返し、usage 自身は usage.jsonl に記録
  しない。
- `packages/cli/src/text-format.ts`: `formatUsageReport` で
  human-readable 出力。
- `packages/cli/src/usage-log.ts`: record に `turns` /
  `toolCalls` を追加して集計可能にした。
- テスト: `args.test.ts` / `usage-report.test.ts` /
  `pricing.test.ts` / `usage-log.test.ts` 更新。

### Decisions During Implementation

- estimated は report 層でのみ計算し、run telemetry には保存
  しない（actual / measured / estimated の分離）。
- stale / missing pricing では estimated section 自体を省略する。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `vitest run`（packages/cli 関連 suite）pass。
- `pnpm typecheck` / `pnpm lint` pass。

## Risks

- `pricing.json` のスキーマは #45 の実装と要整合（consumer は
  `models["<provider>/<id>"]` + per-million USD + `checkedAt` を
  期待）。

## Follow-ups

- #45 で pricing metadata の取得・更新 layer が実装された際、
  契約の相互確認。
