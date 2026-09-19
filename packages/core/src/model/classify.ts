import type { ModelConfig } from "./config.js";
import { findModelException } from "./exceptions.js";
import { resolvePricing, type ModelPricing } from "./pricing.js";
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
 * 検査可能にする（issue #45）。
 */
export type TierSource =
  /** models.yaml `overrides` の明示指定 */
  | "explicit-override"
  /** bundled curated exceptions（exhaustive な tier list ではない） */
  | "curated-exception"
  /** pricing heuristic による自動分類（#46 が正式 formula に差し替える seam） */
  | "auto"
  /** どの分類根拠にも当たらず medium にフォールバック */
  | "unclassified";

export interface Classification {
  tier: Tier;
  source: TierSource;
}

/**
 * interim auto classifier。effective pricing の `output`（USD/1M token）
 * で `<2 → low` / `≤20 → medium` / `>20 → high` を返す。pricing 欠落・
 * output=0 は「無料」の観測であり low に自動分類しない
 * （issue #45 非目標）。#46 がこの関数を正式 formula に差し替える。
 */
function autoClassify(pricing: ModelPricing | undefined): Tier | undefined {
  const output = pricing?.output;
  if (output === undefined || output === 0) return undefined;
  if (output < 2) return "low";
  if (output <= 20) return "medium";
  return "high";
}

/**
 * tier 分類 pipeline。priority は `overrides > curated exceptions >
 * auto classifier > unclassified` で最初に一致したものを採用する。
 * unclassified の effective tier は `medium`（既存不変条件）だが
 * `source` で区別できる。
 *
 * `opts.pricing` 未指定時は `resolvePricing(model)`（runtime catalog
 * → bundled snapshot）に fallback する。
 */
export function classifyModel(
  model: ModelInfo,
  config: ModelConfig,
  opts?: { pricing?: ModelPricing | undefined },
): Classification {
  const override = config.overrides[modelKey(model.provider, model.id)];
  if (override !== undefined) return { tier: override, source: "explicit-override" };

  const exception = findModelException(model.provider, model.id);
  if (exception !== undefined) return { tier: exception.tier, source: "curated-exception" };

  const pricing = opts?.pricing ?? resolvePricing(model);
  const auto = autoClassify(pricing);
  if (auto !== undefined) return { tier: auto, source: "auto" };

  return { tier: "medium", source: "unclassified" };
}
