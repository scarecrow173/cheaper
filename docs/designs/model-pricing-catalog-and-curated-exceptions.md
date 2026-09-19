---
id: "DESIGN-034rzP2XX0QSo5LRQDfr9B"
type: "design"
status: "approved"
title: "model pricing catalog and curated exceptions"
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
  implements:
    - "../specs/model-pricing-catalog.md"
  implemented-by: []
  depends-on:
    - "./model-resolver.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./model-resolver.md"
    - "../adr/pricing-metadata-layer-and-curated-exceptions.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/model-pricing-catalog.md"
    - "../adr/pricing-metadata-layer-and-curated-exceptions.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# model pricing catalog and curated exceptions

## Context

現行の model 選択は `packages/core/src/model/` の 3 層:
`config.ts`（`models.yaml` の overrides/preferences 読込）、
`classify.ts`（`BUILTIN_TABLE` regex + `cost.output` heuristic）、
`resolve.ts`（tier candidate 選択と fallback）。runtime-pi 側は
`model-runtime.ts` が Pi `Model` → `ModelInfo` 変換と
`resolvePiModel` を提供し、`run.ts` が `meta.model` を envelope に
記録する。

issue #45 の方針転換により、(a) `BUILTIN_TABLE` を curated
exceptions へ縮小し、(b) normalized pricing metadata layer を
#44/#46 共用の contract として公開し、(c) `exclude` / `--model` /
`cheaper models` を追加する。

Pi `Model.cost`（`ModelCost` = `{ input, output, cacheRead,
cacheWrite, tiers? }`、USD/1M tokens）は models.dev 由来の generated
catalog を保持するため、pricing の正準 source は runtime catalog とし、
欠損のみ bundled snapshot で補完する。

## Scope

### In Scope

- `ModelPricing` normalized contract と `resolvePricing`（runtime
  catalog 優先 → bundled snapshot 補完 → unpriced）
- bundled data module: `model-exceptions`（curated tier exceptions）と
  `pricing-snapshot`（補完用 versioned snapshot）
- `models.yaml` `exclude`（eligibility。tier 値には混ぜない）
- `classifyModel` が `{ tier, source: TierSource }` を返す pipeline 化
- `resolveModel` の candidate への exclude 適用と `tierSource` /
  `pricing` の返却、`resolveExplicitModel`（`--model` 用）
- CLI `--model <provider>/<id>`（run と worker alias）と
  `cheaper models [--provider] [--tier]`
- `meta.model.selection`（`"tier" | "explicit"`）

### Out of Scope

- #46 の正式 auto tier formula（workload profile 連携）
- #44 の savings 集計本体
- pricing snapshot の自動生成 pipeline
- 全 model の exhaustive な built-in tier 一覧

## Components and Boundaries

### `packages/core/src/model/pricing.ts`（新規）

```ts
export type PricingSource = "runtime-catalog" | "bundled-snapshot";

export interface ModelPricing {
  /** USD / 1M tokens */
  input: number;
  output: number;
  cacheRead?: number;
  cacheWrite?: number;
  unit: "usd-per-1m-tokens";
  currency: "USD";
  source: PricingSource;
  /** bundled snapshot の version、または runtime catalog の observed 表記 */
  snapshotVersion?: string;
  updatedAt?: string;
}
```

- `resolvePricing(model: ModelInfo, snapshot: PricingSnapshot):
  ModelPricing | undefined`
  - runtime catalog（`model.cost` が input/output を持つ）→
    `source: "runtime-catalog"`
  - 欠損時に bundled snapshot の `"<provider>/<id>"` entry →
    `source: "bundled-snapshot"`, `snapshotVersion` 付与
  - どちらにも無い → `undefined`（unpriced、error にしない）
- `ModelInfo` に `cost` の cacheRead/cacheWrite を任意 field として
  拡張し、`toModelInfo`（runtime-pi）が Pi `Model.cost` をそのまま
  写す。core 側の normalized 表現は `ModelPricing` のみとし、
  `cost` は runtime 入力 shape として残す（変換は
  `resolvePricing` 一箇所に集約）。

### `packages/core/src/model/data/`（新規、bundled data）

esbuild bundle（`scripts/bundle-cli.mjs`）で単一 `.mjs` に埋め込む
ため、bundled data は runtime file IO を伴わない TS module とする。

- `pricing-snapshot.ts`:
  `{ version: string; updatedAt: string; models: Record<string,
  { input; output; cacheRead?; cacheWrite? }> }`（key は
  `"<provider>/<id>"`）。初版は小規模（Pi catalog に pricing を持たない
  代表 model のみ）。
- `model-exceptions.ts`:
  `Record<string, { tier: Tier; reason: string; source: string;
  checkedAt: string }>`。key は `"<provider>/<id>"` または
  `"<provider>/*"` wildcard（`preferences`/`exclude` と同じ pattern
  規則）。初版 entry: `ollama/*` → `low`（local/zero-price、
  reason 明記）など。

### `classify.ts`（pipeline 化）

```ts
export type TierSource =
  | "explicit-override"
  | "curated-exception"
  | "auto"           // #46 が生成経路を追加する seam。#45 では生成しない
  | "unclassified";

export interface Classification { tier: Tier; source: TierSource }

export function classifyModel(
  model: ModelInfo,
  config: ModelConfig,
): Classification
```

priority: `config.overrides` → bundled exceptions（exact →
`provider/*`）→ unclassified。

- auto classifier stage は #46 の scope。#45 では `source: "auto"` を
  生成する経路を持たず、override・exception 非該当の model はすべて
  unclassified に落ちる（issue #45 追記により absolute USD
  threshold の暫定実装は不採用）。`TierSource` の `"auto"` は #46 が
  差し込む seam として union に残す。
- `BUILTIN_TABLE` regex 分類は削除する。
- unclassified は tier `medium` を返す（既存不変条件）が
  `source: "unclassified"` で区別できる。

### `resolve.ts`（exclude + explicit + metadata 返却）

- `resolveModel(opts)`:
  - `available` から `config.exclude`（exact + `provider/*`）に一致
    する model を candidate set から先に除去
  - `ResolvedModel` に `tierSource: TierSource` と
    `pricing?: ModelPricing` を追加
  - `details.candidatesPerTier` は exclude 適用後の集合で算出
- `resolveExplicitModel(opts)`（新規）:
  `{ available, ref: { provider, id }, config, policy }`
  - `exclude` 一致 → `POLICY_VIOLATION`（`details.reason:
    "model-excluded"`）。`--model` でも exclude は突破しない
  - `available` に `provider/id` が無い → `MODEL_TIER_UNAVAILABLE`
    （availability は bypass しない）
  - effective tier が `policy.model.maxTier` を超える →
    `POLICY_VIOLATION`（policy の明示 deny は bypass しない）
  - 返却 `{ model, tier, tierSource, pricing, selection: "explicit" }`

### `config.ts`（`exclude` 追加）

- `MODELS_CONFIG_SCHEMA` に `exclude: { type: "array", items: string }`
- `ModelConfig.exclude: string[]`
- merge 規則: `exclude` は deny list のため user/project の配列を
  **union（concat + dedupe）**する。`mergePolicyLayers` の array
  replace 規則には乗せず、`loadModelConfig` で層ごとの値を収集して
  合成する（project 層が user の exclude を消さない）。

### runtime-pi

- `model-runtime.ts`:
  - `toModelInfo` が `cost` の cacheRead/cacheWrite も写す
  - `resolvePiModel` が `ResolvedPiModel` に `tierSource`・`pricing`・
    `selection: "tier"` を返す
  - `resolveExplicitPiModel({ catalog, ref, config, policy })` を追加
    （core `resolveExplicitModel` → `getModel` で実体解決）
- `run.ts`:
  - `RunWorkerDeps.explicitModel?: string`（`"provider/id"`）
  - 指定時は `resolveExplicitPiModel`、未指定時は従来の tier 経路
  - `WorkerRunMeta.model` に `selection: "tier" | "explicit"` と
    `tierSource` を追加

### `packages/cli`

- `args.ts`: `--model <provider>/<id>` option（run / alias のみ有効、
  `provider/id` 形式を検証）、`models` コマンド（`--provider`・
  `--tier` filter option）
- `cli.ts`:
  - run 経路で `explicitModel` を `runWorker` deps へ渡す
  - `models` command: ModelRuntime 生成 → `getAvailable()` →
    exclude/pricing/classification を適用した一覧を envelope で返す
    （`meta` ではなく `result.models[]`）
- `text-format.ts`: models の text 表示（provider/model、available/
  excluded、pricing、tier、tier source、pricing source/freshness）

## Data and Control Flow

```text
run path:
  args --model? → loadModelConfig（exclude 含む）→ ModelRuntime
    → getAvailable() → ModelInfo[]
    → explicit? resolveExplicitPiModel : resolvePiModel
        （内部: exclude 適用 → resolvePricing → classify → order）
    → session → meta.model{provider,id,tier,selection,tierSource}

models path:
  ModelRuntime → getAvailable() → per model:
    excluded? / effective pricing / classification{tier,source}
    / pricing source・freshness → envelope result.models[]
```

## Non-Functional Constraints

- bundled data は TS module に埋め込み、runtime file IO・network を
  要求しない（esbuild bundle・offline 動作）
- stale/missing pricing は fail-open（error にせず freshness を露出）
- 新規依存なし
- `exclude`/`overrides`/`preferences` は `models.yaml` 既存 schema
  version 1 のまま拡張（後方互換: exclude 未指定 = 空）

## Risks and Trade-offs

- #46 の auto classifier 提供まで override/exception 非該当の
  model は unclassified（medium 扱い）に倒れる → low tier が
  必要な model は `overrides` で pin する運用でカバーする
- regex 表削除で name-pattern 由来の分類が消える → exceptions +
  #46 の auto classifier で代替。分類品質は #46 が担保する
- `exclude` union merge は `mergePolicyLayers` の規則と異なる →
  deny list 性質上の意図的例外として design に明記

## Deferred Design Concerns

- pricing snapshot の自動生成・更新 pipeline | reason: 初版は小規模
  手編集で十分 | trigger: snapshot entry 数の増加・#44 で広範な
  pricing が必要になった時点 | risk: 手編集漏れで unpriced 増加
- freshness 閾値超過時の挙動（warn のみか分類除外か）| reason:
  #44/#46 の消費要件次第 | trigger: それらの実装 | risk: stale
  pricing 由来の分類 drift
- alias/canonical id の双方向 mapping | reason: 初版は exact +
  `provider/*` で足りる | trigger: alias 不一致の exception 増加 |
  risk: 例外 entry の重複管理

## Verification Notes

- [ ] classify priority・exclude・explicit selection・snapshot 補完が
      `packages/core/test/model/` の単体テストで確認される
- [ ] `--model` と `models` コマンドが `packages/cli/test` で確認される
- [ ] `meta.model.selection`/`tierSource` が envelope に現れる
