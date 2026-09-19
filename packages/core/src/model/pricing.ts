import { PRICING_SNAPSHOT } from "./data/pricing-snapshot.js";
import { matchModelPattern } from "./ref.js";

export type PricingSource = "runtime-catalog" | "bundled-snapshot";

/**
 * normalized pricing metadata（ADR-034rzI0Yqw3L8AFM9diZct）。
 * #44（cost/savings 推定）と #46（auto tier classifier）が共通で
 * 消費する contract。unit/currency は USD/1M tokens に固定する。
 */
export interface ModelPricing {
  /** USD / 1M input tokens */
  input: number;
  /** USD / 1M output tokens */
  output: number;
  cacheRead?: number;
  cacheWrite?: number;
  unit: "usd-per-1m-tokens";
  currency: "USD";
  source: PricingSource;
  /** bundled snapshot 由来の場合の snapshot version */
  snapshotVersion?: string;
  /** source 側の観測/更新日（ISO date）。不明なら省略 */
  updatedAt?: string;
}

export interface PricingSnapshotEntry {
  input: number;
  output: number;
  cacheRead?: number;
  cacheWrite?: number;
  updatedAt?: string;
}

export interface PricingSnapshot {
  version: string;
  updatedAt: string;
  /** key は "<provider>/<id>" または "<provider>/*" */
  models: Record<string, PricingSnapshotEntry>;
}

interface PricedLike {
  provider: string;
  id: string;
  cost?: { input: number; output: number; cacheRead?: number; cacheWrite?: number };
}

/**
 * model の effective pricing を返す。runtime catalog（`model.cost` が
 * 存在する場合。zero-price も「無料」という観測として有効）を優先し、
 * 欠損時のみ bundled snapshot で補完する。どちらにも無ければ
 * `undefined`（unpriced、error にしない）。
 */
export function resolvePricing(
  model: PricedLike,
  snapshot: PricingSnapshot = PRICING_SNAPSHOT,
): ModelPricing | undefined {
  const cost = model.cost;
  if (cost !== undefined) {
    return {
      input: cost.input,
      output: cost.output,
      ...(cost.cacheRead !== undefined ? { cacheRead: cost.cacheRead } : {}),
      ...(cost.cacheWrite !== undefined ? { cacheWrite: cost.cacheWrite } : {}),
      unit: "usd-per-1m-tokens",
      currency: "USD",
      source: "runtime-catalog",
    };
  }
  const entryKey = Object.keys(snapshot.models).find((pattern) =>
    matchModelPattern(model.provider, model.id, pattern),
  );
  const entry = entryKey !== undefined ? snapshot.models[entryKey] : undefined;
  if (entry === undefined) return undefined;
  return {
    input: entry.input,
    output: entry.output,
    ...(entry.cacheRead !== undefined ? { cacheRead: entry.cacheRead } : {}),
    ...(entry.cacheWrite !== undefined ? { cacheWrite: entry.cacheWrite } : {}),
    unit: "usd-per-1m-tokens",
    currency: "USD",
    source: "bundled-snapshot",
    snapshotVersion: snapshot.version,
    updatedAt: entry.updatedAt ?? snapshot.updatedAt,
  };
}
