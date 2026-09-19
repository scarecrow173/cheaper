import type { ModelConfig } from "./config.js";
import { findModelException } from "./exceptions.js";
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
  /** pricing metadata 由来の自動分類。#46 が生成経路を追加する seam で、#45 では生成しない */
  | "auto"
  /** どの分類根拠にも当たらず medium にフォールバック */
  | "unclassified";

export interface Classification {
  tier: Tier;
  source: TierSource;
}

/**
 * tier 分類 pipeline。priority は `overrides > curated exceptions >
 * unclassified` で最初に一致したものを採用する。price ベースの自動分類
 * （`source: "auto"`）は #46 の scope であり #45 では生成しない
 * （issue #45 追記）。unclassified の effective tier は `medium`
 * （既存不変条件）だが `source` で区別できる。
 */
export function classifyModel(model: ModelInfo, config: ModelConfig): Classification {
  const override = config.overrides[modelKey(model.provider, model.id)];
  if (override !== undefined) return { tier: override, source: "explicit-override" };

  const exception = findModelException(model.provider, model.id);
  if (exception !== undefined) return { tier: exception.tier, source: "curated-exception" };

  return { tier: "medium", source: "unclassified" };
}
