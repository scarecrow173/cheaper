import type { ModelConfig } from "./config.js";
import { findModelException } from "./exceptions.js";
import { type ModelPricing, resolvePricing } from "./pricing.js";
import { modelKey } from "./ref.js";
import type { Tier } from "./tier.js";

/** Pi 型に依存しない core 側の最小 model shape。 */
export interface ModelInfo {
  provider: string;
  id: string;
  cost?: { input: number; output: number; cacheRead?: number; cacheWrite?: number };
  maxTokens?: number;
}

/**
 * tier 分類の provenance。`cheaper models`・`meta.model.tierSource` で
 * 検査可能にする（issue #45 / #46）。
 */
export type TierSource =
  /** models.yaml `overrides` の明示指定 */
  | "explicit-override"
  /** bundled curated exceptions（exhaustive な tier list ではない） */
  | "curated-exception"
  /** workload effectiveCost の母集団相対 percentile による自動分類 */
  | "auto"
  /** どの分類根拠にも当たらず medium にフォールバック */
  | "unclassified";

/**
 * `workload.outputPerInput` 未指定 worker の default（issue #46）。
 * input/output 等重み = 従来の total cost 順序と同じ中立な重み付け。
 */
export const DEFAULT_OUTPUT_PER_INPUT = 1.0;

/** auto 分類の根拠（`source === "auto"` のときのみ存在）。 */
export interface AutoClassificationDetail {
  /** USD/1M input tokens 相当の workload 加重コスト。 */
  effectiveCost: number;
  /** 母集団内の midrank percentile（全員同値・母集団 1 件なら 0.5）。 */
  effectiveCostPercentile: number;
  /** 分類に使った workload profile。 */
  outputPerInput: number;
}

export interface Classification {
  tier: Tier;
  source: TierSource;
  auto?: AutoClassificationDetail;
}

/**
 * auto 分類の ranking 母集団。`costs` は母集団全 model の
 * effectiveCost 昇順（同値を含む）。
 */
export interface AutoTierContext {
  outputPerInput: number;
  costs: readonly number[];
}

/**
 * workload 加重の effectiveCost。pricing は `usd-per-1m-tokens` に
 * normalized 済み前提（issue #46）。
 */
export function effectiveCost(pricing: ModelPricing, outputPerInput: number): number {
  return pricing.input + pricing.output * outputPerInput;
}

/**
 * input/output が共に 0 の pricing は「無料」ではなく opaque billing
 * （local model・subscription alias・unknown model 等）の観測として
 * ranking 母集団と auto 分類の双方から除外する（issue #46 /
 * #45 不変条件）。
 */
function isPricedForRanking(pricing: ModelPricing | undefined): pricing is ModelPricing {
  return pricing !== undefined && pricing.input + pricing.output > 0;
}

/**
 * ranking 母集団から auto 分類 context を構築する。母集団には
 * override / curated exception を持つ model も含める（順位は実際の
 * 価格観測全体に基づき、tier 割当のみ auto を skip する）。
 */
export function buildAutoTierContext(
  models: readonly ModelInfo[],
  opts?: { outputPerInput?: number },
): AutoTierContext {
  const outputPerInput = opts?.outputPerInput ?? DEFAULT_OUTPUT_PER_INPUT;
  const costs: number[] = [];
  for (const model of models) {
    const pricing = resolvePricing(model);
    if (!isPricedForRanking(pricing)) continue;
    costs.push(effectiveCost(pricing, outputPerInput));
  }
  costs.sort((a, b) => a - b);
  return { outputPerInput, costs };
}

/**
 * midrank percentile と tier 境界。`p = (countBelow + countEqual/2)/n`。
 * 同値は同一 p・同一 tier で恣意的に分割しない。母集団の
 * effectiveCost が全て同値（n=1 を含む）なら全員 `p=0.5 → medium`
 * で「相対順位なし」を中立に表現する。境界比較は整数演算で行い
 * float 誤差を避ける（low: `3(2·countBelow + countEqual) < 2n`、
 * medium: `< 4n`、else high）。
 */
function percentileTier(
  costs: readonly number[],
  cost: number,
): { tier: Tier; percentile: number } | undefined {
  const n = costs.length;
  if (n === 0) return undefined;
  let countBelow = 0;
  let countEqual = 0;
  for (const c of costs) {
    if (c < cost) countBelow += 1;
    else if (c === cost) countEqual += 1;
    else break;
  }
  const percentile = (countBelow + countEqual / 2) / n;
  const score = 3 * (2 * countBelow + countEqual);
  const tier: Tier = score < 2 * n ? "low" : score < 4 * n ? "medium" : "high";
  return { tier, percentile };
}

/**
 * tier 分類 pipeline。priority は `overrides > curated exceptions >
 * auto classifier > unclassified` で最初に一致したものを採用する。
 * unclassified の effective tier は `medium`（既存不変条件）だが
 * `source` で区別できる。
 *
 * auto 分類は `opts.autoContext`（`buildAutoTierContext` で構築した
 * 母集団）が必要。未指定時は auto 分類を実行せず unclassified に
 * 落ちる。`opts.pricing` 未指定時は `resolvePricing(model)`
 * （runtime catalog → bundled snapshot）に fallback する。
 */
export function classifyModel(
  model: ModelInfo,
  config: ModelConfig,
  opts?: { pricing?: ModelPricing | undefined; autoContext?: AutoTierContext },
): Classification {
  const override = config.overrides[modelKey(model.provider, model.id)];
  if (override !== undefined) return { tier: override, source: "explicit-override" };

  const exception = findModelException(model.provider, model.id);
  if (exception !== undefined) return { tier: exception.tier, source: "curated-exception" };

  const pricing = opts?.pricing ?? resolvePricing(model);
  const ctx = opts?.autoContext;
  if (ctx !== undefined && isPricedForRanking(pricing)) {
    const cost = effectiveCost(pricing, ctx.outputPerInput);
    const ranked = percentileTier(ctx.costs, cost);
    if (ranked !== undefined) {
      return {
        tier: ranked.tier,
        source: "auto",
        auto: {
          effectiveCost: cost,
          effectiveCostPercentile: ranked.percentile,
          outputPerInput: ctx.outputPerInput,
        },
      };
    }
  }

  return { tier: "medium", source: "unclassified" };
}

/**
 * 入力集合全体を母集団として一括分類する。`classifyModel` に
 * 共通の autoContext を渡す wrapper。
 */
export function classifyModels(
  models: readonly ModelInfo[],
  config: ModelConfig,
  opts?: { outputPerInput?: number },
): Map<ModelInfo, Classification> {
  const autoContext = buildAutoTierContext(models, opts);
  const out = new Map<ModelInfo, Classification>();
  for (const model of models) {
    out.set(model, classifyModel(model, config, { pricing: resolvePricing(model), autoContext }));
  }
  return out;
}
