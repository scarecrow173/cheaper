---
id: "SPEC-034rzI0xebTv8HszaX6Tvw"
type: "spec"
status: "approved"
title: "Model pricing catalog and curated exceptions"
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
    - "./cheaper-v1-worker-runtime-cli.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/model-tier-model-resolver.md"
    - "../adr/pricing-metadata-layer-and-curated-exceptions.md"
    - "../adr/usage-jsonl-contract-canonical.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/model-pricing-catalog-and-curated-exceptions.md"
  derived-by:
    - "../designs/model-pricing-catalog-and-curated-exceptions.md"
  verifies: []
  verified-by:
    - "../test-specs/model-tier-classification-pipeline.md"
    - "../test-specs/model-exclude-and-explicit-selection.md"
    - "../test-specs/pricing-metadata-and-models-command.md"
  references: []
  defers: []
  deferred-by: []
---

# Model pricing catalog and curated exceptions

## What Should Be Built

利用可能 model の normalized pricing metadata を resolver・telemetry・
auto classifier へ供給する pricing metadata layer と、price-based
分類で扱えない例外だけを管理する built-in curated exceptions、
user/project の `exclude`・`overrides`、tier resolver を bypass する
`--model` 直接指定、metadata を確認する `cheaper models` コマンドを
導入する。

## Why Now

現行の tier 分類は `BUILTIN_TABLE` の regex パターンと `cost.output`
heuristic に依存し、全 current model の網羅的管理を cheaper が手作業で
抱える構造になっている。issue #45 の議論で「全 model tier の手管理は
しない」「pricing metadata から分類できる model は #46 の price-based
classifier に任せる」「built-in は例外のみ」と方針転換された。
#44（usage/savings telemetry）と #46（auto tier classifier）が共通で
消費する pricing metadata layer を先に基盤として提供する必要がある。

## Audience

- cheaper CLI の利用者（不要 model の exclude、tier override、
  `--model` 直接指定、`cheaper models` での確認）
- upstream Coding Agent（run 結果の selection metadata を参照）
- #44 / #46 の実装者（同一 pricing metadata layer を API として利用）

## In Scope

- normalized `ModelPricing` metadata（input/output/cache read・write
  price、pricing unit/currency、source、freshness）の contract と、
  runtime provider catalog との join
- bundled pricing snapshot（runtime catalog に pricing が無い model の
  補完用・versioned・offline で読める）
- built-in curated exceptions（pricing だけでは扱えない model/alias の
  tier・適用理由・provenance を持つ entry）
- `models.yaml` への `exclude` 追加と既存 `overrides` の維持
- 分類結果への tier source 記録（`explicit-override` /
  `curated-exception` / `auto` / `unclassified`）
- `--model <provider>/<id>` による tier resolver bypass（ただし
  exclude・availability・policy deny は bypass しない）
- `cheaper models [--provider <id>] [--tier <tier>]` で
  available/excluded・effective pricing・tier・tier source・pricing
  source/freshness を表示
- `meta.model.selection`（`"tier"` / `"explicit"`）の記録
- auto classifier の seam（#46 が差し込む interface）。#45 では
  `auto` を生成する経路を持たない

## Out of Scope

- 全 current model の tier を built-in に列挙すること
- #46 の price-based auto tier formula 本体（workload profile 連携を含む）
- #44 の usage/savings telemetry 集計ロジック本体
- model benchmark / 性能ランキング
- subscription の実請求額の再現
- local model を price=0 という理由だけで自動 low 分類すること
- runtime 実行ごとの network fetch を必須とする仕組み

## Behavior

### Classification pipeline

```text
project/user exclude          → eligibility（tier ではない）
project/user tier override    → tierSource = explicit-override
built-in curated exception    → tierSource = curated-exception
auto classifier（#46 seam）   → tierSource = auto（#45 では生成しない）
上記いずれも非該当            → tierSource = unclassified
```

- `exclude` に一致する model は resolver の candidate set から除かれ、
  `--model` で明示指定されても拒否される（`POLICY_VIOLATION` 系 error）。
- `overrides` は built-in exception より優先する。
- auto classifier は #46 の scope。#45 では `tierSource: "auto"` を
  生成する経路を持たず、override・exception のいずれにも該当しない
  model はすべて unclassified になる。`TierSource` union の `"auto"`
  は #46 が差し込む seam として残す。
- unclassified model は既存不変条件どおり resolver 上 `medium` として
  扱うが、tier source 表示では `unclassified` と区別できる。
- `ignore` を tier に混ぜない。除外は `exclude` のみで表現する。

### Pricing metadata

- 各 model について normalized pricing（input/output/cache read/write、
  unit/currency、source、freshness）を参照できる。
- 正準 source は runtime/provider catalog（Pi `Model.cost`、models.dev
  由来）。catalog に pricing が無い model は bundled snapshot で補完する。
- 実行ごとの network 必須にしない。bundled snapshot と runtime catalog
  cache で offline 動作する。
- stale / missing pricing は error ではなく freshness metadata として
  露出する（fail-open、#44/#46 が解釈を決める）。

### Direct model selection

- `cheaper run <worker> --model <provider>/<id>` および worker alias
  （`bulk-read` 等）で model を直接指定できる。
- `--model` 指定時は tier classification と candidate selection を
  bypass するが、provider/auth readiness・model availability・
  `exclude`・policy deny・runtime compatibility は bypass しない。
- `meta.model.selection` は `"explicit"`、tier resolver 経由は `"tier"`
  を記録する。

### `cheaper models` コマンド

- 利用可能 model 集合を provider/model・available/excluded・effective
  pricing・tier・tier source・pricing source/freshness と共に表示する。
- `--provider` / `--tier` で filter できる。

## Acceptance Criteria

- [ ] runtime catalog の model 集合が resolver の candidate 母集団として
      使われ、`exclude` 適用後の集合が tier 分類の対象になる
- [ ] `models.yaml` の `exclude` が tier resolver の候補と `--model`
      指定の双方から model を除外する
- [ ] `overrides` が built-in exception より優先される
- [ ] built-in exception の entry が reason/source/checkedAt を持ち、
      overrides に次ぐ priority で適用される
- [ ] override・exception のいずれにも該当しない model が
      unclassified（effective tier `medium`）になり、`tierSource:
      "auto"` が #45 では生成されない
- [ ] runtime catalog に pricing が無い model が bundled snapshot で
      補完される
- [ ] pricing がどこにも無い model は unpriced として扱われ error に
      ならない
- [ ] `--model` 指定が tier resolver を bypass し `selection:
      "explicit"` が meta に記録される
- [ ] `--model` が `exclude`・availability・policy を突破しない
- [ ] `cheaper models` が tier source と pricing source/freshness を
      表示できる
- [ ] `mise run check` が pass する

## Sources

- issue #45: https://github.com/scarecrow173/cheaper/issues/45
- 上位 spec: `docs/specs/cheaper-v1-worker-runtime-cli.md`
- 上位 ADR: `docs/adr/model-tier-model-resolver.md`
- discovery: `docs/discovery/model-pricing-catalog-and-curated-exceptions.md`
