import { CheaperError } from "../errors.js";
import {
  type AutoClassificationDetail,
  buildAutoTierContext,
  classifyModel,
  classifyModels,
  type ModelInfo,
  type TierSource,
} from "./classify.js";
import type { ModelConfig } from "./config.js";
import { type ModelPricing, resolvePricing } from "./pricing.js";
import { matchModelPattern, modelKey } from "./ref.js";
import { compareTier, nextTier, TIERS, type Tier } from "./tier.js";

export interface ResolvedModel {
  model: ModelInfo;
  /** 実際に選ばれた tier（fallback 時は要求と異なりうる）。 */
  tier: Tier;
  /** tier の provenance（explicit-override / curated-exception / auto / unclassified）。 */
  tierSource: TierSource;
  /** effective pricing（unpriced なら undefined）。 */
  pricing?: ModelPricing;
  /** auto 分類の根拠（tierSource === "auto" のときのみ存在）。 */
  auto?: AutoClassificationDetail;
  /** "tier" = tier resolver 経路、"explicit" = `--model` 直接指定。 */
  selection: "tier" | "explicit";
}

function totalCost(m: ModelInfo): number {
  return m.cost === undefined ? Number.POSITIVE_INFINITY : m.cost.input + m.cost.output;
}

function byCostThenProviderThenId(a: ModelInfo, b: ModelInfo): number {
  return (
    totalCost(a) - totalCost(b) || a.provider.localeCompare(b.provider) || a.id.localeCompare(b.id)
  );
}

/** `config.exclude`（exact + provider/* wildcard）に一致するか。 */
export function isExcludedModel(m: ModelInfo, config: ModelConfig): boolean {
  return (config.exclude ?? []).some((pattern) => matchModelPattern(m.provider, m.id, pattern));
}

/**
 * 順序 = preferences のパターン順 → 残りを (cost.input + cost.output) 昇順
 * → provider → id（cost 欠落は Infinity 扱いで末尾）。同一パターン内も同じ順で total order。
 */
function orderCandidates(candidates: ModelInfo[], patterns: string[]): ModelInfo[] {
  const remaining = [...candidates];
  const picked: ModelInfo[] = [];
  for (const pattern of patterns) {
    const hit = remaining.filter((m) => matchModelPattern(m.provider, m.id, pattern));
    hit.sort(byCostThenProviderThenId);
    picked.push(...hit);
    for (const m of hit) remaining.splice(remaining.indexOf(m), 1);
  }
  remaining.sort(byCostThenProviderThenId);
  return [...picked, ...remaining];
}

/**
 * 候補 = available ∖ exclude ∩ classify==tier。requested > maxTier →
 * POLICY_VIOLATION。候補空 && tierFallback==="higher" → 1 段ずつ上へ
 * maxTier まで。尽きれば MODEL_TIER_UNAVAILABLE（details.candidatesPerTier）。
 */
export function resolveModel(opts: {
  available: ModelInfo[];
  requestedTier: Tier;
  policy: { maxTier: Tier; tierFallback: "none" | "higher" };
  config: ModelConfig;
  /** Worker workload profile（issue #46）。未指定は DEFAULT_OUTPUT_PER_INPUT。 */
  outputPerInput?: number;
}): ResolvedModel {
  const { available, requestedTier, policy, config } = opts;

  const eligible = available.filter((m) => !isExcludedModel(m, config));
  if (eligible.length === 0) {
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

  const classifications = classifyModels(eligible, config, {
    ...(opts.outputPerInput !== undefined ? { outputPerInput: opts.outputPerInput } : {}),
  });
  const byTier = new Map<
    Tier,
    { model: ModelInfo; classification: ReturnType<typeof classifyModel>; pricing?: ModelPricing }[]
  >();
  for (const m of eligible) {
    const pricing = resolvePricing(m);
    const classification = classifications.get(m) as ReturnType<typeof classifyModel>;
    const list = byTier.get(classification.tier) ?? [];
    list.push({ model: m, classification, ...(pricing !== undefined ? { pricing } : {}) });
    byTier.set(classification.tier, list);
  }

  let tier: Tier | undefined = requestedTier;
  while (tier !== undefined && compareTier(tier, policy.maxTier) <= 0) {
    const entries = byTier.get(tier) ?? [];
    const ordered = orderCandidates(
      entries.map((e) => e.model),
      config.preferences[tier] ?? [],
    );
    if (ordered.length > 0) {
      const model = ordered[0] as ModelInfo;
      const entry = entries.find((e) => e.model === model) as (typeof entries)[number];
      return {
        model,
        tier,
        tierSource: entry.classification.source,
        ...(entry.pricing !== undefined ? { pricing: entry.pricing } : {}),
        ...(entry.classification.auto !== undefined ? { auto: entry.classification.auto } : {}),
        selection: "tier",
      };
    }
    if (policy.tierFallback !== "higher") break;
    tier = nextTier(tier);
  }

  const candidatesPerTier = Object.fromEntries(TIERS.map((t) => [t, (byTier.get(t) ?? []).length]));
  throw new CheaperError("MODEL_TIER_UNAVAILABLE", `no model available for tier ${requestedTier}`, {
    details: { requestedTier, candidatesPerTier },
  });
}

/**
 * `--model <provider>/<id>` 直接指定の resolver。tier candidate 選択は
 * bypass するが、exclude・availability・maxTier policy は突破しない
 * （issue #45）。
 */
export function resolveExplicitModel(opts: {
  available: ModelInfo[];
  ref: { provider: string; id: string };
  policy: { maxTier: Tier };
  config: ModelConfig;
  /** Worker workload profile（policy 判定用 tier の分類に使う）。 */
  outputPerInput?: number;
}): ResolvedModel {
  const { available, ref, policy, config } = opts;
  const target: ModelInfo = { provider: ref.provider, id: ref.id };

  if (isExcludedModel(target, config)) {
    throw new CheaperError(
      "POLICY_VIOLATION",
      `model ${modelKey(ref.provider, ref.id)} is excluded by models config`,
      { details: { reason: "model-excluded", model: modelKey(ref.provider, ref.id) } },
    );
  }
  const model = available.find((m) => m.provider === ref.provider && m.id === ref.id);
  if (model === undefined) {
    throw new CheaperError(
      "MODEL_TIER_UNAVAILABLE",
      `model ${modelKey(ref.provider, ref.id)} is not available`,
      { details: { reason: "model-unavailable", model: modelKey(ref.provider, ref.id) } },
    );
  }
  const eligible = available.filter((m) => !isExcludedModel(m, config));
  const autoContext = buildAutoTierContext(eligible, {
    ...(opts.outputPerInput !== undefined ? { outputPerInput: opts.outputPerInput } : {}),
  });
  const pricing = resolvePricing(model);
  const classification = classifyModel(model, config, { pricing, autoContext });
  if (compareTier(classification.tier, policy.maxTier) > 0) {
    throw new CheaperError(
      "POLICY_VIOLATION",
      `model ${modelKey(ref.provider, ref.id)} tier ${classification.tier} exceeds policy maxTier ${policy.maxTier}`,
      {
        details: {
          reason: "model-exceeds-max-tier",
          model: modelKey(ref.provider, ref.id),
          tier: classification.tier,
          maxTier: policy.maxTier,
        },
      },
    );
  }
  return {
    model,
    tier: classification.tier,
    tierSource: classification.source,
    ...(pricing !== undefined ? { pricing } : {}),
    ...(classification.auto !== undefined ? { auto: classification.auto } : {}),
    selection: "explicit",
  };
}
