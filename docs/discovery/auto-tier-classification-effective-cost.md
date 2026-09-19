---
id: "DISC-Hsh18eGWIzyBCzpfoRxUb3"
type: "discovery"
status: "resolved"
title: "auto-tier-classification-effective-cost"
created: "2026-09-20"
updated: "2026-09-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/46"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "../specs/model-pricing-catalog.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/model-pricing-catalog.md"
    - "../adr/pricing-metadata-layer-and-curated-exceptions.md"
    - "../designs/model-pricing-catalog-and-curated-exceptions.md"
    - "../designs/model-resolver.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/auto-tier-classification.md"
    - "../adr/price-based-percentile-auto-tier-classification.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# auto-tier-classification-effective-cost

## Exploration Goal

issue #46「Worker workload と effectiveCost から利用可能モデルを自動
tier 分類する」の実装方針を確定する。具体的には (a) Worker workload
profile（`outputPerInput`）の schema と default/fallback、(b)
effectiveCost ranking と percentile 境界の決定的 rule、(c) small
set / tie / pricing 欠落時の contract、(d) `classifyModel` の interim
heuristic を置き換える API 形状、を探る。

## Key Issues and Assumptions

- #45 で `classify.ts` に interim auto classifier（`cost.output`
  threshold）が `source: "auto"` の seam として実装済み。本 issue は
  この関数を ranking ベースの正式 formula に差し替える。分類は
  model 単体ではなく candidate set 全体の相対順位に依存するため、
  現行の per-model API では母集団を表現できない。batch 分類 API が
  必要になる。
- `resolveModel` / `resolveExplicitModel` / `cheaper models` の 3 箇所が
  分類を消費する。`--model` 指定時も maxTier policy 判定のため tier
  分類自体は必要（bypass するのは candidate *selection* のみ）。
- tier は Worker 相対（同一 model でも Worker により tier が変わる）。
  `cheaper models` のような worker 非指定の一覧は、何らかの基準
  workload で分類するか `--worker` を受け付ける必要がある。
- pricing source は `resolvePricing`（runtime catalog → bundled
  snapshot）で USD/1M tokens に normalized 済み。`input=0 && output=0`
  は「無料」ではなく opaque billing の観測として ranking から除外し
  unclassified を維持する（#45 の既存不変条件と整合）。
- 母集団に explicit override / curated exception 済み model を含めるか
  は issue が「価格ランキングには利用可能な priced model 全体を使う」
  を優先候補と明記。分布の安定性のためこの案を採用する。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| percentile: `countBelow/n`（厳密に安い model の割合） | 単純・直感的 | 全員同値 → 全員 low（p=0）。単一 model も low になり「相対順位なし」を表現できない | no |
| percentile: midrank `(countBelow + countEqual/2)/n` | 同値は同一 percentile で中央に寄る。全員同値・n=1 が自動的に p=0.5→medium で special case 不要 | 2 件で low/high に分かれる（medium 無し） | yes |
| percentile: `rank/(n-1)`（min=0, max=1） | 両端が必ず low/high | n=1 で 0/0 不定・要 special case、同値処理を別途定義が必要 | no |
| 母集団: eligible ∧ priced 全体（override/curated 含む） | 分布が安定・issue 優先候補 | curated model が枠を占有 | yes |
| 母集団: auto 対象 model のみ | 分類対象だけの相対順位 | override 追加で分布が揺れる | no |
| 未指定 worker の fallback: built-in default 定数 | schema 省略可能・後方互換 | default 値の根拠が推定 | yes |
| 未指定 worker は auto 分類しない | 明示主義 | 既存 custom worker が全て unclassified 化し退化 | no |

## Tentative Conclusions and Hypotheses

- (hypothesis) `effectiveCost(model, worker) = input + output *
  outputPerInput`（USD/1M tokens 単位で normalized 済み前提）。
- (hypothesis) percentile は midrank 方式 `p = (countBelow +
  countEqual/2) / n`、境界は `p < 1/3 → low`、`p < 2/3 → medium`、
  それ以外 `high`。境界は built-in 定数とし config 化は deferred。
- (hypothesis) workload schema は `cheaper.model.workload.
  outputPerInput`（number ≥ 0）。未指定は built-in default
  `DEFAULT_OUTPUT_PER_INPUT`（1.0 = 現行 totalCost 順序と同重み）。
- (hypothesis) built-in 初期値は bulk-reader=0.05 / code-writer=0.3 /
  doc-writer=1.0 の推定値。#44 telemetry による検証を follow-up とする。
- (hypothesis) explainability は `meta.model` に `effectiveCost` /
  `effectiveCostPercentile` / `workload.outputPerInput` を追加し、
  `tierSource` は既存 `"auto"` を維持（issue 例の `auto-price` は
  #45 の `TierSource` contract を壊すため採用しない）。

## Open Questions

- `cheaper models` の基準 workload → `--worker` option で worker の
  profile を使い、未指定は built-in default とする案を design で確定。
- percentile boundary の config 化 → YAGNI で初版は built-in 定数のみ。
- cache pricing（cacheReadPerInput / cacheWritePerInput）→ 初版は非
  対応、field 追加余地のみ残す。

## Promotion Candidates

- [x] spec-doc needed: workload schema・effectiveCost・percentile
  contract・small-set/tie rule・precedence・explainability
- [x] adr-doc needed: midrank percentile と母集団定義・default
  outputPerInput の根拠を記録

## Sources and Relations

- issue #46 本文: https://github.com/scarecrow173/cheaper/issues/46
- 前提実装: `packages/core/src/model/classify.ts`（interim auto seam）、
  `pricing.ts`（normalized ModelPricing）、`resolve.ts`、
  `worker/validate.ts`（`cheaper.model` schema）
- 上位決定: `docs/adr/model-tier-model-resolver.md`、
  `docs/specs/model-pricing-catalog.md`、
  `docs/adr/pricing-metadata-layer-and-curated-exceptions.md`
