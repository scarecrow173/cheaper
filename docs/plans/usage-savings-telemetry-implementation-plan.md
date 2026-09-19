---
id: "PLAN-034rzPZHq5JFQ3CVZCVLJo"
type: "plan"
status: "in-progress"
title: "Usage savings telemetry — implementation plan"
test-spec-skip: "Verified by unit tests and fake-stream e2e assertions inside the implementation tasks; no separate test-spec artifact needed."
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
    - "docs/specs/usage-and-savings-telemetry-aggregation.md"
  implemented-by:
    - "../tasks/measure-tool-output-tokens-and-expose-meta-savings.md"
    - "../tasks/extend-usage-jsonl-run-record-with-savings-section.md"
    - "../tasks/add-cheaper-usage-aggregation-command-with-filters-and-estimated-savings.md"
    - "../tasks/add-e2e-coverage-docs-bundle-regen-and-final-verification.md"
  depends-on:
    - "../adr/measured-offload-tool-output-token-estimated-savings-pricing-seam.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/usage-and-savings-telemetry-aggregation.md"
    - "docs/designs/usage-savings-telemetry.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Usage savings telemetry — implementation plan Implementation Plan

## Goal

worker run の measured offload（tool output token 和 vs 返却
result token 和）を `meta.savings` と `usage.jsonl` record に
実測記録し、`cheaper usage` 集計コマンドと pricing metadata
経由の estimated savings を追加する。

## Scope

単一の implementation stream。core → runtime-pi → cli の
依存方向に沿って 4 task に分割する。pricing metadata の
生成機構（#45）は scope 外で、consumer contract
（`pricing.json`）の読み取りのみ実装する。

## Upstream Documents

- `docs/specs/usage-and-savings-telemetry-aggregation.md`
- `docs/adr/measured-offload-tool-output-token-estimated-savings-pricing-seam.md`
- `docs/discovery/usage-and-savings-telemetry-for-delegation-reduction.md`

## Design Inputs

- `docs/designs/usage-savings-telemetry.md`

## File Map

| Path | Change | Reason |
|---|---|---|
| `packages/core/src/token/count.ts` | modify | `countTextTokens(text)` を公開 |
| `packages/runtime-pi/src/tools/meter.ts` | add | tool output token/byte 集計 wrap |
| `packages/runtime-pi/src/tools/registry.ts` | modify | meter を wrap 層に挟む |
| `packages/runtime-pi/src/usage.ts` | modify | savings 型 |
| `packages/runtime-pi/src/run.ts` | modify | `meta.savings` 組み立て |
| `packages/runtime-pi/test/*` | add/modify | meter・meta の unit test |
| `packages/cli/src/usage-log.ts` | modify | record に `savings` 転写 |
| `packages/cli/src/pricing.ts` | add | `pricing.json` 読み取り + stale 判定 |
| `packages/cli/src/usage-report.ts` | add | usage.jsonl 集計ロジック |
| `packages/cli/src/args.ts` | modify | `usage` command + filter option |
| `packages/cli/src/cli.ts` | modify | `usage` 分岐 |
| `packages/cli/src/text-format.ts` | modify | 集計の text 表示 |
| `packages/cli/test/*` | add/modify | record・集計・pricing の unit test |
| `packages/cli/test/e2e/*` | modify/add | fake-stream savings・usage e2e |
| `docs/guides/usage.md` | modify | 指標定義・`usage` command |
| `AGENTS.md` | modify | savings/usage command の記載 |
| `packages/apm/.apm/hooks/cli/bin/cheaper.mjs` | generated | bundle 再生成 |

## Implementation Sequence

- [ ] TASK-034rzRtRHEEtyWvIINcDnL: `countTextTokens` + tool meter + `meta.savings`（unit test 含む）
- [ ] TASK-034rzRugER0y7cQ2tlWjPp: run record への `savings` 転写（unit test 含む）
- [ ] TASK-034rzRuCuCgZ08BmHTCqgz: `cheaper usage` command（args・report・pricing・text format、unit test 含む）
- [ ] TASK-034rzRv1K2fmi2Igbdu5HG: e2e・docs・bundle 再生成・最終検証

## Dependencies and Risks

- TASK-034rzRugER0y7cQ2tlWjPp は TASK-034rzRtRHEEtyWvIINcDnL の `meta.savings` に依存（型・値の供給元）。
- TASK-034rzRuCuCgZ08BmHTCqgz は独立だが、savings field の集計は TASK-034rzRugER0y7cQ2tlWjPp の record
  shape に依存。
- tool output の encode コスト: tool output は bounded（read は
  50KB 上限等）だが、grep/find の大量 output で per-call encode
  が増える。tokenizer は共有 instance で実害は小さいと判断。
- fake-stream fixture が tool call を行わない場合、savings が
  0/欠損になる — e2e は tool call を含む scripted fixture で
  検証する。

## Task Breakdown

- 下記 4 task を `docs/tasks/` に作成し `implements` で本 plan に
  紐付ける。

## Verification

- [ ] `pnpm typecheck`・`pnpm lint`・`pnpm test` pass
- [ ] fake-stream e2e で `meta.savings` と record `savings` を
      assert
- [ ] `cheaper usage` の filter・集計・estimated 非出力条件を
      unit test で確認
- [ ] `mise run check` pass（bundle freshness 含む）

## Open Questions / Gaps

- `pricing.json` の正式 schema は #45 側で確定し得る — 本 plan
  は minimal contract（`models` map + per-entry
  `inputPerMillionUsd`/`outputPerMillionUsd`/`checkedAt`/`source`）
  のみを実装する。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（worker agent が task graph
  順に実装）。
- Independent work streams: なし（依存 chain のため順次実行）。
- User approval required before dispatch: no（Orchestrator 委任
  により自律実行）。
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- `meta.savings` の field 名（sourceTokens/returnedTokens/
  avoidedContextTokens）は spec 固定。error run では
  `returnedTokens: 0`。
- estimated は `--compare-model` + pricing entry + 非 stale の
  3 条件が揃ったときのみ出力する。
