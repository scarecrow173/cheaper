---
id: "PLAN-034rzUgf26bzSEwLr1L6Pl"
type: "plan"
status: "approved"
title: "implement model pricing catalog and curated exceptions"
created: "2026-09-19"
updated: "2026-09-19"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/45"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/model-pricing-catalog.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/pricing-metadata-layer-and-curated-exceptions.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/model-pricing-catalog.md"
    - "docs/designs/model-pricing-catalog-and-curated-exceptions.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/model-tier-classification-pipeline.md"
    - "docs/test-specs/model-exclude-and-explicit-selection.md"
    - "docs/test-specs/pricing-metadata-and-models-command.md"
  references: []
  defers: []
  deferred-by: []
---

# implement model pricing catalog and curated exceptions Implementation Plan

## Goal

normalized pricing metadata layer・curated exceptions・`exclude` /
`--model` / `cheaper models` を spec（SPEC-034rzI0xebTv8HszaX6Tvw）と
design（DESIGN-034rzP2XX0QSo5LRQDfr9B）どおりに実装する。

## Scope

単一の implementation stream。core の model 層 → runtime-pi → cli の
依存順で 1 plan にまとめる。#46 の auto tier formula 本体と #44 の
集計ロジックはこの plan の外。

## Upstream Documents

- `docs/specs/model-pricing-catalog.md`
- `docs/adr/pricing-metadata-layer-and-curated-exceptions.md`

## Design Inputs

- `docs/designs/model-pricing-catalog-and-curated-exceptions.md`

## File Map

- `packages/core/src/model/pricing.ts` — `ModelPricing` /
  `PricingSnapshot` / `resolvePricing`（新規）
- `packages/core/src/model/data/pricing-snapshot.ts` — bundled 補完
  snapshot（新規）
- `packages/core/src/model/data/model-exceptions.ts` — curated
  exceptions（新規）
- `packages/core/src/model/classify.ts` — `TierSource` /
  `Classification` を返す pipeline 化
- `packages/core/src/model/config.ts` — `exclude` schema と union merge
- `packages/core/src/model/resolve.ts` — exclude 適用・metadata 返却・
  `resolveExplicitModel`
- `packages/core/src/index.ts` — 新規 module export
- `packages/runtime-pi/src/model-runtime.ts` — `toModelInfo` 拡張・
  `resolveExplicitPiModel`
- `packages/runtime-pi/src/run.ts` — `explicitModel` deps・
  `meta.model.selection`/`tierSource`
- `packages/cli/src/args.ts` — `--model`・`models` コマンド・
  `--provider`/`--tier` filter
- `packages/cli/src/cli.ts` — run 経路の explicit 接続・`models`
  コマンド実行
- `packages/cli/src/text-format.ts` — models の text 表示
- `packages/core/test/model/*.test.ts` — pricing/exclude/classify/
  resolve の単体テスト
- `packages/cli/test/args.test.ts`・`cli` 系テスト — flag/command の
  単体テスト
- `docs/guides/configuration.md`・`docs/guides/providers.md`・
  `docs/guides/usage.md` — 利用者向け文書

## Implementation Sequence

- [ ] core: pricing contract + bundled data（snapshot/exceptions）
- [ ] core: `models.yaml` `exclude` + union merge
- [ ] core: classify pipeline + resolve（exclude・explicit・metadata）
- [ ] runtime-pi: pricing 写し・explicit 解決・meta 拡張
- [ ] cli: `--model` flag・`models` コマンド・text 表示
- [ ] guides 更新と index export の整合

## Dependencies and Risks

- `ModelInfo.cost` 拡張は optional field の追加のみで既存呼出を壊さない
- `classifyModel` の返り値型変更は package 内部 API（`resolve.ts` と
  テストのみが利用）なので影響は限定
- `exclude` の union merge は `mergePolicyLayers` と規則が異なるため
  config 層で明示合成する（design 記載の意図的例外）

## Task Breakdown

- `add-model-pricing-metadata-layer` — pricing.ts + bundled data
- `add-models-config-exclude` — `exclude` schema + merge
- `implement-classification-pipeline-and-explicit-resolve` — classify +
  resolve
- `wire-explicit-model-and-meta-in-runtime` — runtime-pi 側
- `add-cli-model-flag-and-models-command` — cli 側
- `update-guides-for-pricing-catalog` — docs/guides

## Verification

- [ ] `pnpm -r test`（vitest）で全単体テスト pass
- [ ] `pnpm -r build` で typecheck pass
- [ ] `mise run check` pass

## Open Questions / Gaps

- bundled snapshot の初版 entry は少数（ollama/local 等）に留める。
  拡充は #44/#46 の消費要件が固まってから。
- `cheaper models` の text 表示列は issue の表示候補をそのまま採用。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution
- Independent work streams: none（core → runtime-pi → cli は直列依存）
- User approval required before dispatch: no（Orchestrator 委任による
  自律実行）

## Review Handoff

- interim auto classifier は #46 で差し替わる前提の暫定実装。
  review では seam の分離が保たれているかを確認。
