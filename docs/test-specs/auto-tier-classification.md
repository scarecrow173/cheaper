---
id: "TSPEC-CcB52hP0U8aHcOIhg0D0ge"
type: "test-spec"
status: "approved"
title: "auto tier classification by effectiveCost percentile"
created: "2026-09-20"
updated: "2026-09-20"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/auto-tier-classification.md"
  derived-by: []
  verifies:
    - "docs/specs/auto-tier-classification.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# auto tier classification by effectiveCost percentile

## Purpose

Worker workload（`outputPerInput`）と normalized pricing から算出した
effectiveCost の母集団相対 ranking が、issue #46 の contract どおり
deterministic に tier を返すことを保証する。分類自体が absolute
threshold へ戻る・同値を恣意的に分割する・pricing 欠落 model を
推測分類するような退化を防ぐ。

## Feature

Worker-specific effectiveCost percentile による model tier 自動分類

## Rules

- `effectiveCost = pricing.input + pricing.output * outputPerInput`
- percentile は midrank: `p = (countBelow + countEqual / 2) / n`
- `p < 1/3 → low`、`p < 2/3 → medium`、それ以外 `high`
- 母集団は exclude 適用後の利用可能 model のうち pricing を持つ
  全体（override/curated 持ちも順位には含める）
- `input = 0 && output = 0`・pricing 欠落の model は母集団にも
  auto 分類対象にもならず unclassified
- 同一 effectiveCost の model は常に同一 percentile・同一 tier
- precedence: `explicit-override > curated-exception > auto >
  unclassified`
- `outputPerInput` 未指定 worker は `DEFAULT_OUTPUT_PER_INPUT`
- `--model` は tier candidate selection を bypass するが exclude・
  availability・maxTier policy は bypass しない

## Examples

- 母集団 3 件・全員異なる effectiveCost → 最安 low / 中央 medium /
  最高 high
- 母集団 2 件・異なる effectiveCost → low / high（medium 無し）
- 母集団 1 件または全員同値 → 全員 `p = 0.5 → medium`（auto）
- 母集団 6 件・全員異なる effectiveCost → low 2 / medium 2 / high 2
- outputPerInput=0 の worker では output price が ranking に影響
  しない（effectiveCost = input のみ）
- 同一 model set でも outputPerInput が異なる worker で tier が
  変わりうる
- `ollama/*` curated exception は pricing に関わらず low /
  curated-exception
- pricing を持つ model が `overrides` で high 指定 → auto より
  explicit-override が優先
- input=0, output=0 の model → unclassified（low にしない）

## Guarantees

- 同一入力・同一 workload で常に同一 tier（deterministic）
- 価格水準が一様に変動しても相対 ranking が保存される
- 分類根拠（effectiveCost・percentile・workload）が
  `meta.model` / `cheaper models` から追跡できる

## Non-goals

- built-in `outputPerInput` 初期値の実測妥当性（#44 follow-up）
- percentile boundary の数値自体の最適性
- cache pricing を含む effectiveCost

## Risk

- ranking 母集団の定義ミス（exclude 済み・unpriced が混入）は
  tier 分布全体を歪める
