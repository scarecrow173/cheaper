export const TIERS = ["low", "medium", "high"] as const;

export type Tier = (typeof TIERS)[number];

export function compareTier(a: Tier, b: Tier): number {
  return TIERS.indexOf(a) - TIERS.indexOf(b);
}

export function isTier(v: unknown): v is Tier {
  return typeof v === "string" && (TIERS as readonly string[]).includes(v);
}

export function nextTier(t: Tier): Tier | undefined {
  const next = TIERS.indexOf(t) + 1;
  return next < TIERS.length ? TIERS[next] : undefined;
}
