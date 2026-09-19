---
id: "ADR-034rzI0Yqw3L8AFM9diZct"
type: "adr"
status: "accepted"
title: "Pricing metadata layer と curated exceptions"
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
    - "../specs/model-pricing-catalog.md"
  implemented-by: []
  depends-on:
    - "model-tier-model-resolver.md"
    - "agent-runtime-pi-sdk-earendil-works-pi-coding-agent.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "model-tier-model-resolver.md"
    - "usage-jsonl-contract-canonical.md"
    - "../specs/model-pricing-catalog.md"
    - "../designs/model-pricing-catalog-and-curated-exceptions.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/model-pricing-catalog-and-curated-exceptions.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://models.dev"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# Pricing metadata layer と curated exceptions

## Context and Problem Statement

issue #45 で model tier 管理の方針が転換された。全 current model を
cheaper が手分類する exhaustive catalog は持たず、(1) runtime/provider
catalog から利用可能 model 集合を取得し、(2) pricing metadata source
で pricing を補完し、(3) price で分類できる model は #46 の auto
classifier に任せ、(4) built-in は価格だけでは扱えない例外のみを
curated exceptions として持つ、という構造に移行する。

現行実装は `BUILTIN_TABLE` regex 分類 + `cost.output` heuristic で、
normalized pricing metadata の contract も tier source の記録もない。
#44（savings 推定）と #46（auto classifier）が同じ pricing metadata
を必要とするため、基盤となる layer をここで決める。

## Decision Drivers

* issue #45: built-in は exhaustive list ではなく curated exceptions
* issue #45: runtime 実行ごとの network 必須にしない
* Pi `Model.cost` が既に models.dev 由来の normalized pricing
  （input/output/cacheRead/cacheWrite、USD/1M tokens）を持ち、
  generated catalog が package に version pin されて bundle 済み
* #44 / #46 が同一 pricing metadata layer を消費する
* `exclude` は tier ではなく eligibility の責務として分離する
* unclassified model は medium 扱い（既存不変条件を維持）

## Considered Options

* (1) Pi runtime catalog の `Model.cost` を正準 pricing source とし、
  cheaper 側は normalized view + 補完用 bundled snapshot のみ持つ
* (2) LiteLLM / models.dev の全件 snapshot を cheaper が独自に bundle
  し正準 source とする
* (3) runtime が実行時に pricing source へ fetch する

## Decision Outcome

Chosen option: "(1) runtime catalog 正準 + bundled 補完 snapshot",
because Pi が既に models.dev の versioned generated catalog を bundle
しており `Model.cost` に normalized pricing を載せているため、cheaper が
全件 snapshot を独自管理する必要はなく、runtime catalog に pricing が
無い model だけを小規模な bundled snapshot で補完すれば offline 動作と
reproducibility を両立できるから。

あわせて以下を決定する。

* built-in regex 分類表（`BUILTIN_TABLE`）は廃止し、bundled
  curated exceptions（`provider/model` → `{ tier, reason, source,
  checkedAt }`）へ置き換える
* 分類 priority は `exclude（eligibility）> user override > curated
  exception > auto classifier > unclassified`
* auto classifier は #46 が実装する pluggable seam とし、#45 では既存
  cost heuristic を interim 実装として `tierSource: "auto"` で維持する
* `exclude` は `models.yaml` の `exclude: string[]` で表現し、
  resolver candidate set と `--model` 直接指定の双方に適用する
* `--model <provider>/<id>` は tier resolver を bypass するが
  exclude/availability/policy は bypass せず、`meta.model.selection`
  に `"explicit"` を記録する
* `cheaper models` で available/excluded・effective pricing・tier・
  tier source・pricing source/freshness を露出する

### Consequences

* Good, because pricing の freshness・更新が Pi の catalog 更新と
  models.dev 追従に乗り、cheaper 側の手作業は exception entry のみに
  縮小する。
* Good, because #44/#46 が `ModelPricing` / tier source を同じ
  contract から取得でき、表現 drift を避けられる。
* Good, because `exclude` が eligibility layer に分離され、tier 値に
  `ignore` を混ぜない設計になる。
* Bad, because Pi catalog に存在しない provider/plugin model は
  bundled snapshot か exception で個別補完が必要。
* Bad, because interim cost heuristic と #46 の正式 formula が一時的に
  二重に存在し得る（seam として隔離して軽減）。

### Confirmation

* 分類 priority・exclude・`--model` bypass・snapshot 補完・
  unpriced/stale 扱いが `packages/core` と `packages/cli` の
  単体テストで確認されること。
* `cheaper models` が tier source / pricing source / freshness を
  出力すること。

## Implementation Plan

* Affected paths: `packages/core/src/model/`（pricing・exceptions・
  classify・resolve・config）、`packages/cli/src/`（`--model`、
  `models` command）、`packages/runtime-pi/src/model-runtime.ts`
  （Model → ModelPricing 変換、explicit selection）
* Patterns to follow: `models.yaml` の layer merge（`mergePolicyLayers`
  共有）、`ModelInfo` の構造型 seam、`meta.model` envelope 記録
* Patterns to avoid: 全件 pricing snapshot の独自管理、tier への
  `ignore` 混入、run ごとの network fetch
* Dependencies or configuration: 新規依存なし。bundled snapshot /
  exceptions は `packages/core` 内の versioned data file

## Verification

* [ ] exclude/override/exception/auto/unclassified の priority と
      `tierSource` が単体テストで確認される
* [ ] bundled snapshot が runtime catalog の欠損 pricing を補完する
      テスト
* [ ] `--model` が exclude/availability/policy を bypass しないテスト
* [ ] `cheaper models` の出力が tier source・pricing source・
      freshness を含むテスト
* [ ] `mise run check` pass

## Pros and Cons of the Options

### Option 1: runtime catalog 正準 + bundled 補完

* Good, because Pi が bundle する models.dev generated catalog をそのまま
  正準 source にでき、offline/reproducible を満たす。
* Good, because snapshot は欠損補完用の小規模データで済む。
* Bad, because Pi の pricing schema 変更に追随が必要。

### Option 2: cheaper 独自の全件 snapshot

* Good, because provider 中立で schema を cheaper が完全管理できる。
* Bad, because 全件データの生成・更新 pipeline が要り、#45 の scope に
  対して過剰。Pi catalog と内容が重複する。

### Option 3: runtime fetch

* Good, because 常に最新 pricing を参照できる。
* Bad, because 実行ごとの network 必須になり issue の要件に反する。
  offline で分類不能になる。

## More Information

* issue #45: https://github.com/scarecrow173/cheaper/issues/45
* discovery: `docs/discovery/model-pricing-catalog-and-curated-exceptions.md`
* spec: `docs/specs/model-pricing-catalog.md`
* Pi `Model.cost`: `@earendil-works/pi-ai` `dist/types.d.ts` の
  `ModelCost`（`ModelCostRates + tiers`）、generated catalog は
  `dist/providers/data/*.json`
