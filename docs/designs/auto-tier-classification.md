---
id: "DESIGN-C4VbdOatUn4aVt3blVe8pk"
type: "design"
status: "approved"
title: "auto tier classification by workload effectiveCost percentile"
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
    - "./model-resolver.md"
    - "./model-pricing-catalog-and-curated-exceptions.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./model-resolver.md"
    - "./model-pricing-catalog-and-curated-exceptions.md"
    - "../adr/price-based-percentile-auto-tier-classification.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/auto-tier-classification.md"
    - "../adr/price-based-percentile-auto-tier-classification.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# auto tier classification by workload effectiveCost percentile

## Context

`packages/core/src/model/classify.ts` の `autoClassify` は #45 が残した
interim heuristic（`cost.output` の絶対 threshold）で、per-model 完結の
ため母集団相対の分類を表現できない。issue #46 は Worker workload
（`outputPerInput`）から `effectiveCost` を算出し、利用可能 candidate
set 内の percentile で tier を決める正式実装を要求する。

分類消費点は 3 箇所: `resolveModel`（tier 候補選択）、
`resolveExplicitModel`（`--model` の maxTier policy 判定）、
`cheaper models`（一覧表示）。全て同一母集団ルールを共有する必要が
ある。

## Scope

### In Scope

- `worker/validate.ts`: `cheaper.model.workload.outputPerInput` の
  schema 受理と `WorkerDefinition` への保持
- `classify.ts`: interim `autoClassify` を ranking ベースの正式実装に
  置き換え、母集団 context を受け取る API を追加
- `resolve.ts`: `resolveModel` / `resolveExplicitModel` が workload を
  受け取り母集団単位で分類し、auto 分類の根拠を返す
- runtime-pi: `resolvePiModel` が worker の `outputPerInput` を渡し、
  `meta.model` に explainability field を載せる
- cli: `cheaper models --worker <name>` と result 行への分類根拠追加
- built-in worker 3 種への初期 `outputPerInput` 設定

### Out of Scope

- cache pricing（`cacheReadPerInput` / `cacheWritePerInput`）の式追加
- percentile boundary の user config 化
- profile の runtime 自動学習
- `tierSource` 値の変更（`"auto"` を維持）

## Components and Boundaries

### `worker/validate.ts`（workload schema）

`MODEL_KEYS` に `"workload"` を追加し、`cheaper.model.workload` を
optional object として受理する。

```ts
cheaper: {
  model: {
    tier: Tier;
    workload?: { outputPerInput: number };
  };
}
```

`outputPerInput` は `typeof === "number"` かつ finite かつ `>= 0` のみ
有効。不正値は `INVALID_WORKER_DEFINITION`（reason
`workload-outputPerInput-invalid`）。`workload` 内の未知 key は
`unknown-field-workload-<key>` で拒否する。

### `classify.ts`（ranking-based auto classifier）

interim `autoClassify` を削除し、母集団単位の分類に置き換える。

```ts
export const DEFAULT_OUTPUT_PER_INPUT = 1.0;

/** 分類根拠（source === "auto" のときのみ存在）。 */
export interface AutoClassificationDetail {
  effectiveCost: number;
  /** midrank percentile (0,1)。全員同値なら 0.5。 */
  effectiveCostPercentile: number;
  outputPerInput: number;
}

export interface Classification {
  tier: Tier;
  source: TierSource;
  auto?: AutoClassificationDetail;
}
```

- `effectiveCost = pricing.input + pricing.output * outputPerInput`。
- ranking 母集団: 入力集合のうち `resolvePricing` が pricing を返し
  `input + output > 0` の model。`input = 0 && output = 0` は
  「無料」ではなく opaque billing の観測として除外し、auto 分類も
  しない（unclassified → medium、#45 不変条件）。
- midrank percentile: `p = (countBelow + countEqual / 2) / n`。
- boundary は built-in 定数 `p < 1/3 → low`、`p < 2/3 → medium`、
  else `high`。整数比較 `3*(2*countBelow + countEqual) < 2*n` /
  `>= 4*n` で float 境界誤差を避ける。

```ts
/** 母集団から auto 分類 context を構築する。 */
export function buildAutoTierContext(
  models: readonly ModelInfo[],
  opts?: { outputPerInput?: number },
): AutoTierContext

export function classifyModel(
  model: ModelInfo,
  config: ModelConfig,
  opts?: { pricing?: ModelPricing; autoContext?: AutoTierContext },
): Classification

export function classifyModels(
  models: readonly ModelInfo[],
  config: ModelConfig,
  opts?: { outputPerInput?: number },
): Map<ModelInfo, Classification>
```

- `classifyModels` は入力集合から `buildAutoTierContext` を構築し、
  各 model に `classifyModel` を適用する。母集団には override /
  curated を持つ model も含める（順位の安定性、tier 割当のみ
  auto を skip）。
- `autoContext` 未指定の `classifyModel` は auto 分類を実行できず
  `unclassified` に落ちる。per-model 呼出側は必ず context を渡す。

### `resolve.ts`（resolver 統合）

- `resolveModel` / `resolveExplicitModel` の opts に
  `outputPerInput?: number` を追加（default `DEFAULT_OUTPUT_PER_INPUT`）。
- `resolveModel` は `eligible`（exclude 適用後）を母集団として
  `classifyModels` で一括分類する。
- `resolveExplicitModel` も `eligible` を母集団に target を分類し、
  従来どおり maxTier policy 判定に使う（tier selection bypass は
  変わらない）。
- `ResolvedModel` に auto 分類根拠を載せる:

```ts
export interface ResolvedModel {
  // 既存 field...
  /** source === "auto" の分類根拠。 */
  auto?: AutoClassificationDetail;
}
```

### runtime-pi

- `resolvePiModel` / `resolveExplicitPiModel` の opts に
  `outputPerInput?: number` を追加し core に委譲。
- `run.ts` は `worker.cheaper.model.workload?.outputPerInput` を渡す
  （未指定は core の default）。
- `ResolvedPiModel` に `auto?: AutoClassificationDetail` を追加し、
  `WorkerRunMeta.model` に optional field を拡張:

```ts
model: {
  provider: string; id: string; tier: Tier;
  selection: "tier" | "explicit"; tierSource: TierSource;
  /** source === "auto" のときのみ存在。 */
  effectiveCost?: number;
  effectiveCostPercentile?: number;
  workload?: { outputPerInput: number };
}
```

### `packages/cli`

- `cheaper models` に `--worker <name>` を追加。指定時は
  `resolveWorker` で worker を解決しその `outputPerInput`（未指定は
  default）で分類する。未指定時は `DEFAULT_OUTPUT_PER_INPUT`。
- result の各 model 行に auto 分類時の `effectiveCost` /
  `effectiveCostPercentile` / `workload.outputPerInput` を含める。
  exclude 済み model は母集団から除きつつ表示上は従来どおり
  classification を付ける。

### built-in worker

`packages/cli/workers/*.md` と `packages/apm/.apm/hooks/cli/workers/*.md`
の frontmatter に `workload.outputPerInput` を追加する:

| worker | outputPerInput | 根拠 |
| --- | --- | --- |
| bulk-reader | 0.05 | 大量 input・少量の構造化 output |
| code-writer | 0.3 | spec+context に対し中程度の code output |
| doc-writer | 1.0 | document generation で output が大きい |

初期値は推定であり、#44 telemetry による実測検証を follow-up とする。

## Data and Control Flow

```text
run <worker>:
  worker.workload.outputPerInput ?? DEFAULT_OUTPUT_PER_INPUT
        ↓
  resolvePiModel → resolveModel(available, outputPerInput)
        ↓
  eligible = available ∖ exclude
        ↓
  classifyModels(eligible):
    override → curated → (priced 母集団で effectiveCost rank →
    midrank percentile → boundary) → unclassified
        ↓
  requestedTier の candidates → preferences 順 → ResolvedModel
        ↓
  meta.model += { effectiveCost, effectiveCostPercentile, workload }

--model <provider>/<id>:
  eligible を母集団に target を分類 → maxTier policy 判定のみ
  （candidate selection は bypass）

cheaper models [--worker w]:
  eligible 母集団で全 available model を分類して表示
```

## Non-Functional Constraints

- deterministic: 同一入力集合・同一 workload → 同一 tier。sort は
  effectiveCost → provider → id の total order。
- offline: ranking は `resolvePricing` の bundled snapshot / runtime
  catalog cache のみで完結し、run ごとの network を要しない。
- 表現 drift 回避: percentile 計算・boundary は `classify.ts` に
  集約し、resolver / cli は結果を消費するだけにする。

## Risks and Trade-offs

- midrank は 2 件の母集団で medium が出ない（low/high に二分）。
  「最安 = low / 最高 = high」という直感に合うため許容する。
- 分布が偏ると medium が空になりうる。percentile 分類の性質であり、
  fallback `higher` で吸収される。
- boundary を変えると全 model の tier が再配分される。config 化せず
  コード変更扱いとすることで変更の重大性を明示する。

## Deferred Design Concerns

- cache pricing の式追加（`cacheReadPerInput` / `cacheWritePerInput`
  を workload schema に追加する余地は `workload` object で確保済み）
- percentile boundary の config 化
- #44 telemetry による built-in `outputPerInput` の実測検証・補正
- 分類根拠の `cheaper models` text format への詳細表示

## Verification Notes

- `packages/core/test/model/classify.test.ts`: interim heuristic テストを
  ranking ベースのテストに置き換え、effectiveCost・percentile 境界・
  tie・small set・precedence・価格ゼロを網羅する。
- `resolve.test.ts`: workload 差で tier が変わること・exclude が
  母集団から除かれること・`--model` bypass を確認する。
- `worker/validate` テスト: `workload` schema の受理・不正値拒否。
- cli e2e / unit: `--worker` option・meta explainability field。
