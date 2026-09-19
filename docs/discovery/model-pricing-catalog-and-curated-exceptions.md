---
id: "DISC-034rzGKaCRO8vRQUpNJvWd"
type: "discovery"
status: "resolved"
title: "model-pricing-catalog-and-curated-exceptions"
created: "2026-09-19"
updated: "2026-09-20"
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
  implements: []
  implemented-by: []
  depends-on:
    - "../adr/model-tier-model-resolver.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/model-tier-model-resolver.md"
    - "../designs/model-resolver.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/model-pricing-catalog.md"
    - "../adr/pricing-metadata-layer-and-curated-exceptions.md"
  verifies: []
  verified-by: []
  references:
    - "https://models.dev"
  defers: []
  deferred-by: []
---

# model-pricing-catalog-and-curated-exceptions

## Exploration Goal

issue #45「Model pricing catalog と curated exceptions を導入する」の実装方針を
確定する。具体的には、(a) 利用可能 model の normalized pricing metadata を
どこから取得しどう表現するか、(b) 現行の regex ベース built-in 分類表を
curated exceptions へどう置き換えるか、(c) exclude / explicit override /
`--model` 直接指定 / `cheaper models` コマンドをどの層に置くか、を探る。

## Key Issues and Assumptions

- 現行実装（`packages/core/src/model/classify.ts`）は `BUILTIN_TABLE` の
  regex パターン表 + `cost.output` threshold heuristic で tier を分類する。
  issue の方針転換により、手管理の exhaustive 分類は廃止し curated
  exceptions へ縮小する必要がある。
- Pi (`@earendil-works/pi-ai` 0.85.1) の `Model.cost` は既に
  `{ input, output, cacheRead, cacheWrite, tiers? }`（USD / 1M tokens）を
  持ち、`dist/providers/data/*.json` として models.dev 由来の generated
  catalog が version pin されて bundle されている。runtime refresh は
  `models-store.json` に cache されるため run ごとの network 必須ではない。
  → 「pricing metadata source の bundled snapshot」は実質的に Pi が既に
  提供している。cheaper 側で別途全件 snapshot を持つ必然性は薄い。
- `ModelInfo`（core の最小 shape）は現在 `cost.input/output` のみを持ち、
  cache price・pricing source・freshness を欠く。#44（cost/savings
  推定）と #46（price-based classifier）が消費する normalized metadata は
  ここを拡張する必要がある。
- 分類 priority は issue 指定で `exclude > overrides > curated exception >
  auto classifier(#46) > unclassified`。#46 未実装の間の auto stage を
  どうするかが設計論点だった（候補: 現行 cost heuristic を interim
  classifier として温存 / unclassified → medium 扱い）が、issue #45
  追記で「自動分類の採否に依存させない」と明示され absolute USD
  threshold は不採用。unclassified → medium 扱いに確定した。
- `exclude` は tier ではなく eligibility の責務。resolver の candidate set
  形成と `--model` 直接指定の双方に効かせる必要がある。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| pricing source: Pi runtime catalog の `Model.cost` を正準 source とし、cheaper 側は normalized view + 補完用 bundled snapshot を持つ | network 不要・models.dev 追従が Pi の更新に乗る・実装が薄い | Pi catalog に無い provider/plugin model は別途補完が要る | yes |
| pricing source: LiteLLM/models.dev の全件 snapshot を cheaper が独自 bundle | Pi 非依存で provider 中立 | 全件データの保守・生成 pipeline・更新手順が要り、#45 の規模に対し過剰 | no（補完 snapshot のみ採用） |
| pricing source: runtime に都度 fetch | 常に最新 | issue が明示する「実行ごとに network 必須にしない」に反する | no |
| auto stage: 現行 cost.output heuristic を interim auto classifier として残し、#46 が差し替え | unclassified 増加による tier 退化を防げる | absolute USD threshold は #46 で不採用の方向のため rejected design を production に入れてしまう | no（issue #45 追記・review 指摘で不採用） |
| auto stage: #46 まで全て unclassified | 実装最小・#45 が #46 の採否に依存しない | 未分類 → medium 扱いで low tier には overrides/exceptions の pin が要る | yes |
| built-in regex 表: curated exceptions へ移行して削除 | issue の意図に忠実・表現 drift 解消 | 既知 model の名前一貫性が例外 entry の整備に依存 | yes |

## Tentative Conclusions and Hypotheses

- (hypothesis→確認済) pricing metadata layer は `packages/core` に
  normalized `ModelPricing` / `PricingCatalog` contract として置き、
  runtime catalog join + bundled 補完 snapshot の 2 source を吸収する。
- (hypothesis) classification の結果は tier だけでなく `tierSource`
  （`explicit-override` / `curated-exception` / `auto` / `unclassified`）を
  返す構造にし、`cheaper models` と selection metadata が同じ情報を
  表示・記録できるようにする。
- (hypothesis) `exclude` は `ModelConfig` に `exclude: string[]` として
  追加し、resolver candidate set からの除外と `--model` 指定時の deny
  の両方に適用する。
- (hypothesis) `--model <provider>/<id>` は resolver を bypass するが、
  availability（runtime catalog に存在・credential 状態で利用可能）・
  exclude・policy deny は bypass しない。`meta.model.selection` に
  `"explicit"` を記録する。

## Open Questions

- bundled 補完 snapshot の生成・更新手順（どの source から、どの頻度で）
  → ADR で「初版は手編集の小規模 YAML/JSON + 形式のみ固定」とし、
  自動生成 pipeline は後続課題として扱うか判断する。
- stale pricing（freshness 閾値超過）を warning に留めるか分類から除外
  するか → design で確定（基本は記録・表示のみで fail-open）。
- local / zero-price model（Ollama 等）の既定扱い → curated exception で
  `low` を明示する方針（issue の非目標「price=0 だけで low にしない」に
  整合）。

## Promotion Candidates

- [x] spec-doc needed: pricing metadata layer・exclude/override・`--model`・
  `cheaper models` の要件と acceptance criteria を formalize
- [x] adr-doc needed: pricing source 構成（runtime catalog 正準 + bundled
  補完 snapshot）と分類 pipeline への移行を記録

## Sources and Relations

- issue #45 本文（comment 含む）: https://github.com/scarecrow173/cheaper/issues/45
- 現行分類: `packages/core/src/model/classify.ts` / `resolve.ts` / `config.ts`
- Pi model metadata: `@earendil-works/pi-ai` `dist/types.d.ts`
  （`Model.cost` = `ModelCostRates + tiers`）、`dist/providers/data/*.json`
  （models.dev generated catalog）
- 上位決定: `docs/adr/model-tier-model-resolver.md`（tier 抽象と resolver）
