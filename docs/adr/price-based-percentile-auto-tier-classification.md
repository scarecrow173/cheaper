---
id: "ADR-P0zbksjMxOkaP5fE5ZkVfX"
type: "adr"
status: "accepted"
title: "Price-based percentile auto tier classification"
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
  implements:
    - "../specs/auto-tier-classification.md"
  implemented-by: []
  depends-on:
    - "model-tier-model-resolver.md"
    - "pricing-metadata-layer-and-curated-exceptions.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "pricing-metadata-layer-and-curated-exceptions.md"
    - "model-tier-model-resolver.md"
    - "../specs/auto-tier-classification.md"
    - "../designs/auto-tier-classification.md"
    - "../plans/implement-auto-tier-classification.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/auto-tier-classification-effective-cost.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# Price-based percentile auto tier classification

## Context and Problem Statement

#45 で導入した分類 pipeline の `auto` stage は interim の
`cost.output` 絶対 threshold heuristic であり、issue #46 が要求する
「ユーザーの利用可能 model set 内での相対コスト帯」を表現できない。
auto 分類は model 単体ではなく母集団の相対順位に依存するため、
現行の per-model `classifyModel` API では母集団を扱えず、batch 分類
と percentile 境界・tie・small set の deterministic rule が必要。

## Decision Drivers

- reference model / absolute USD threshold の定期メンテナンスを
  持ち込まない（issue 非目標）
- 同一 effectiveCost の model を恣意的に別 tier に分けない
- 1 件・2 件・全員同値の small set でも deterministic
- explicit / curated tier を auto が上書きしない precedence の維持
- 価格ゼロ（opaque billing の観測）を「無料＝low」と推測しない

## Considered Options

- Option 1: midrank percentile `p = (countBelow + countEqual/2) / n`、
  境界 1/3・2/3 の built-in 定数
- Option 2: rank-index percentile `p = countBelow / n`、同値は
  sort 安定順で分割
- Option 3: 絶対 USD threshold を interim heuristic のまま維持し、
  母集団補正（中央値比等）だけ追加

## Decision Outcome

Option 1 を採用。

- midrank は同値 group を同一 percentile に置くため tie が自然に
  同一 tier になり、「同値を別 tier に分けない」を rule として
  追加せず式の性質で担保できる。
- 全員同値・母集団 1 件は `p = 0.5 → medium` に自動的に落ち、
  「相対順位が存在しない」ことを中立 tier で表現できる。
  unclassified → medium の既存不変条件と整合する。
- 境界 1/3 / 2/3 は built-in 定数 `AUTO_TIER_BOUNDARIES` とし、
  config 化は YAGNI で初版は持たない。
- Option 2 は同値 model が順序依存で tier を跨ぎ得る（恣意的分割に
  抵触）のと、母集団 1 件・全員同値が `p = 0 → low` に落ちて
  「順位情報なし」を表現できないため不採用。
- Option 3 は絶対 threshold の維持という issue 非目標に反するため
  不採用。

### 母集団の定義

auto 分類の ranking 母集団は「exclude 適用後の利用可能 candidate
のうち pricing を持つ model 全体」とし、explicit override・curated
exception 済み model も順位には含める（tier 割当のみ auto を
スキップ）。override の追加で分布が揺れず、順位は実際の価格観測
全体に基づくため安定する。`input = 0 && output = 0` の model は
母集団からも除外する（価格ゼロは API 価格の観測ではない）。

### Consequences

- `classifyModel` の per-model API に加え、母集団を受け取る batch
  分類（`classifyModels` / auto 分類 context）が必要になる。
- tier が Worker workload 相対になるため、`cheaper models` は基準
  workload（`--worker` または built-in default）を必要とする。
- percentile 境界を変えると全 model の tier が再配分される。
  boundary 変更はコード変更扱いとする。

### Confirmation

- `packages/core/test/model/` の unit test が effectiveCost・
  percentile 境界・tie・small set・precedence を網羅する。
- `mise run check`（typecheck / lint / test）で検証する。

## Implementation Plan

`docs/plans/implement-auto-tier-classification.md` に従う。
core model 層（workload schema + classifier）→ resolver 統合
→ runtime-pi / cli の順で薄く実装する。

## Verification

- built-in worker 3 種で fixture model set に対する分類結果を unit
  test で確認する。
- `cheaper models --worker` と `meta.model` の explainability field を
  e2e / unit test で確認する。

## Pros and Cons of the Options

### Option 1: midrank percentile

- Good: tie・単一要素・全員同値を式の性質で処理。順位情報なしが
  medium に落ちる保守性。実装は countBelow/countEqual の数え上げのみ。
- Bad: 母集団 2 件で medium が出ない（low/high に二分）。分布が
  極端に偏ると medium が空になりうる。

### Option 2: rank-index percentile

- Good: 実装が最も単純。
- Bad: 同値分割が arbitrary order 依存、単一 model が low に落ちる。

### Option 3: interim heuristic + 母集団補正

- Good: 既存コードを残せる。
- Bad: 絶対 threshold の定期メンテが残り issue 非目標に抵触。

## More Information

- issue #46: https://github.com/scarecrow173/cheaper/issues/46
- spec: `docs/specs/auto-tier-classification.md`
- design: `docs/designs/auto-tier-classification.md`
- 前提 ADR: `docs/adr/pricing-metadata-layer-and-curated-exceptions.md`
