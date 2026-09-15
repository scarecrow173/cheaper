import { CheaperError } from "../errors.js";
import { classifyModel, type ModelInfo } from "./classify.js";
import type { ModelConfig } from "./config.js";
import { compareTier, nextTier, TIERS, type Tier } from "./tier.js";

export interface ResolvedModel {
  model: ModelInfo;
  /** 実際に選ばれた tier（fallback 時は要求と異なりうる）。 */
  tier: Tier;
}

function totalCost(m: ModelInfo): number {
  return m.cost === undefined ? Number.POSITIVE_INFINITY : m.cost.input + m.cost.output;
}

function byCostThenProviderThenId(a: ModelInfo, b: ModelInfo): number {
  return (
    totalCost(a) - totalCost(b) || a.provider.localeCompare(b.provider) || a.id.localeCompare(b.id)
  );
}

function matchesPattern(m: ModelInfo, pattern: string): boolean {
  if (pattern.endsWith("/*")) return m.provider === pattern.slice(0, -2);
  return `${m.provider}/${m.id}` === pattern;
}

/**
 * 順序 = preferences のパターン順 → 残りを (cost.input + cost.output) 昇順
 * → provider → id（cost 欠落は Infinity 扱いで末尾）。同一パターン内も同じ順で total order。
 */
function orderCandidates(candidates: ModelInfo[], patterns: string[]): ModelInfo[] {
  const remaining = [...candidates];
  const picked: ModelInfo[] = [];
  for (const pattern of patterns) {
    const hit = remaining.filter((m) => matchesPattern(m, pattern));
    hit.sort(byCostThenProviderThenId);
    picked.push(...hit);
    for (const m of hit) remaining.splice(remaining.indexOf(m), 1);
  }
  remaining.sort(byCostThenProviderThenId);
  return [...picked, ...remaining];
}

/**
 * 候補 = available ∩ classify==tier。requested > maxTier → POLICY_VIOLATION。
 * 候補空 && tierFallback==="higher" → 1 段ずつ上へ maxTier まで。
 * 尽きれば MODEL_TIER_UNAVAILABLE（details.candidatesPerTier）。
 */
export function resolveModel(opts: {
  available: ModelInfo[];
  requestedTier: Tier;
  policy: { maxTier: Tier; tierFallback: "none" | "higher" };
  config: ModelConfig;
}): ResolvedModel {
  const { available, requestedTier, policy, config } = opts;

  if (available.length === 0) {
    throw new CheaperError("MODEL_TIER_UNAVAILABLE", "no authenticated models available", {
      details: { noAuthenticatedModels: true },
    });
  }
  if (compareTier(requestedTier, policy.maxTier) > 0) {
    throw new CheaperError(
      "POLICY_VIOLATION",
      `requested tier ${requestedTier} exceeds policy maxTier ${policy.maxTier}`,
      { details: { requestedTier, maxTier: policy.maxTier } },
    );
  }

  const byTier = new Map<Tier, ModelInfo[]>();
  for (const m of available) {
    const tier = classifyModel(m, config);
    const list = byTier.get(tier) ?? [];
    list.push(m);
    byTier.set(tier, list);
  }

  let tier: Tier | undefined = requestedTier;
  while (tier !== undefined && compareTier(tier, policy.maxTier) <= 0) {
    const ordered = orderCandidates(byTier.get(tier) ?? [], config.preferences[tier] ?? []);
    if (ordered.length > 0) return { model: ordered[0] as ModelInfo, tier };
    if (policy.tierFallback !== "higher") break;
    tier = nextTier(tier);
  }

  const candidatesPerTier = Object.fromEntries(TIERS.map((t) => [t, (byTier.get(t) ?? []).length]));
  throw new CheaperError("MODEL_TIER_UNAVAILABLE", `no model available for tier ${requestedTier}`, {
    details: { requestedTier, candidatesPerTier },
  });
}
