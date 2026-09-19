---
id: "SPEC-H9CuNdGONsV6HEHetvRCxh"
type: "spec"
status: "implemented"
title: "Worker workload と effectiveCost による自動 tier 分類"
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
    - "./model-pricing-catalog.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/model-tier-model-resolver.md"
    - "../adr/pricing-metadata-layer-and-curated-exceptions.md"
    - "../adr/price-based-percentile-auto-tier-classification.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/auto-tier-classification-effective-cost.md"
  derived-by:
    - "../designs/auto-tier-classification.md"
    - "../plans/implement-auto-tier-classification.md"
  verifies: []
  verified-by:
    - "../test-specs/auto-tier-classification.md"
  references: []
  defers: []
  deferred-by: []
---

# Worker workload と effectiveCost による自動 tier 分類

## What Should Be Built

#45 の pricing metadata layer が供給する normalized pricing（USD/1M
tokens）と、Worker ごとの workload profile（`outputPerInput`）から
`effectiveCost` を算出し、そのユーザーが利用可能な candidate model
set 内の相対 ranking（percentile）で `low` / `medium` / `high` を
決定的に自動分類する。分類は最下位の補完レイヤーであり、
explicit override / curated exception / exclude を上書きしない。

## Why Now

#45 で分類 pipeline の `auto` stage は interim cost heuristic のまま
残っている。母集団相対の percentile 分類に置き換えることで、
reference model・絶対 USD threshold・全 model の手管理を不要にし、
市場価格の変動や provider 追加に無メンテで追随できるようにする。

## Audience

- cheaper CLI の利用者（custom worker の workload 設定、`cheaper
  models` での分類確認、selection metadata の追跡）
- upstream Coding Agent（`meta.model` の分類根拠を参照）
- #44 の実装者（実測 input/output ratio で built-in profile を検証）

## In Scope

- Worker workload profile schema: `cheaper.model.workload.
  outputPerInput`（`number ≥ 0`、input 1 token あたりの output token
  比率。絶対 token 数は持たない）
- built-in Worker の初期 `outputPerInput`（bulk-reader / code-writer /
  doc-writer）と、未指定 worker の built-in default fallback
- `effectiveCost(model, worker) = input + output * outputPerInput`
- candidate set 内 ranking と percentile による deterministic な
  `low` / `medium` / `high` 分類（built-in boundary 定数）
- small set / tie の deterministic rule
- resolver 統合: tier-based resolution が auto 分類 tier を使い、
  `--model` 直接指定は従来どおり tier candidate selection を bypass
- explainability: `meta.model` と `cheaper models` に分類根拠
  （effectiveCost・percentile・workload）を記録
- 分類結果の deterministic 性（同一入力 → 同一 tier）

## Out of Scope

- capability score・benchmark score・provider marketing tier の推測
- reference model / absolute USD threshold の維持
- subscription 実請求額の再現
- pricing が無い model・価格ゼロ（opaque billing 観測）model の推測分類
- cache pricing（cacheReadPerInput / cacheWritePerInput）の式への
  追加（設計余地のみ残す）
- percentile boundary の user config 化（built-in default のみ）
- runtime が実測で workload profile を自動学習・変更すること
- #44 の telemetry 集計ロジック本体

## Behavior

### Workload profile

- worker frontmatter に `cheaper.model.workload.outputPerInput:
  <number>` を任意指定できる。`0` 以上の有限数のみ有効。
- 未指定 worker は built-in default `DEFAULT_OUTPUT_PER_INPUT = 1.0`
  を使う（input と output を等重み＝従来の total cost 順序と同じ
  重み付けで中立）。
- built-in Worker の初期値は versioned static configuration:
  bulk-reader `0.05`（大量 input・少量 output）、code-writer `0.3`、
  doc-writer `1.0`。実測検証は #44 telemetry の follow-up。

### effectiveCost と ranking

- `effectiveCost = pricing.input + pricing.output * outputPerInput`。
  pricing unit は `usd-per-1m-tokens` に normalized 済みで、ranking
  は単位に依存しない。
- 母集団は exclude 適用後の利用可能 candidate のうち pricing が
  取得できる model 全体（explicit override / curated exception を
  持つ model も順位計算には含め、tier 割当のみ auto をスキップ）。
- `pricing.input === 0 && pricing.output === 0` の model は ranking
  母集団にも入れず unclassified のまま（価格ゼロを無料と推測しない）。
  workload 由来で effectiveCost が 0 になることはあり得るが、それは
  正当な順位付けとして扱う。

### Percentile tier

- 各 model の percentile は midrank 方式:
  `p = (countBelow + countEqual / 2) / n`（`countBelow` = より小さい
  effectiveCost を持つ model 数、`countEqual` = 同値の model 数、
  `n` = 母集団サイズ）。
- boundary: `p < 1/3 → low`、`p < 2/3 → medium`、それ以外 `high`。
- midrank により同 effectiveCost の model は常に同一 percentile・
  同一 tier。母集団の effectiveCost が全て同値（n=1 を含む）の場合
  全員 `p = 0.5 → medium` となり、相対順位が存在しないことを中立に
  表現する。
- 2 件の異価格 model は `p = 0.25 → low` / `p = 0.75 → high`。
  3 件は `low / medium / high` に 1 件ずつ。

### Precedence（#45 不変条件）

```text
user/project exclude            → eligibility（ranking からも除外）
user/project explicit override  → tierSource = explicit-override
built-in curated exception      → tierSource = curated-exception
price-based auto classification → tierSource = auto
上記いずれも非該当              → tierSource = unclassified（medium 扱い）
```

- auto classifier は explicit / curated tier を上書きしない。
- pricing 欠落・価格ゼロの model は auto 分類せず unclassified
  （effective tier は従来どおり medium）。

### Explainability

- `tierSource: "auto"` の分類に対し、`meta.model` と `cheaper
  models` の行に `effectiveCost`・`effectiveCostPercentile`・
  `workload.outputPerInput` を記録する。`tierSource` の命名は
  #45 の `"auto"` を維持する。
- `cheaper models` は `--worker <name>` でその worker の workload
  profile に基づく分類を表示できる。未指定時は built-in default
  workload で分類する。

### Direct model selection

- `--model <provider>/<id>` は従来どおり tier candidate selection を
  bypass する。availability・exclude・maxTier policy は引き続き
  enforce され、policy 判定用の tier は同一母集団ルールで算出する。

## Acceptance Criteria

- [ ] Worker ごとに `cheaper.model.workload.outputPerInput` を設定
      でき、未指定時は built-in default が使われる
- [ ] `effectiveCost = input + output * outputPerInput` が pricing
      metadata から Worker-specific に算出される
- [ ] exclude・unavailable を除いた利用可能 candidate set 内で
      deterministic な相対 ranking が作られる
- [ ] midrank percentile 境界で low / medium / high が決定的に分類
      され、同 effectiveCost の model は同一 tier になる
- [ ] 母集団が 1 件・2 件・全員同値の場合も deterministic に分類
      される（全員同値・単独は medium）
- [ ] explicit override / curated exception を auto classifier が
      上書きしない
- [ ] pricing 欠落・価格ゼロの model は unclassified のまま扱われる
- [ ] `--model` 指定時は tier candidate selection を bypass し、
      exclude / availability / policy は維持される
- [ ] `meta.model` / `cheaper models` で分類根拠（tierSource・
      effectiveCost・percentile・workload）を追跡できる
- [ ] reference model / absolute USD threshold を必要としない
- [ ] `mise run check` が pass する

## Sources

- issue #46: https://github.com/scarecrow173/cheaper/issues/46
- 前提 spec: `docs/specs/model-pricing-catalog.md`
- 上位 ADR: `docs/adr/model-tier-model-resolver.md`
- discovery: `docs/discovery/auto-tier-classification-effective-cost.md`
