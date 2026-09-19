import fs from "node:fs";
import path from "node:path";

export const PRICING_FILENAME = "pricing.json";
/** checkedAt がこの期間より古い pricing は stale とみなす。 */
export const PRICING_STALE_MS = 30 * 24 * 3_600_000;

export interface PricingEntry {
  inputPerMillionUsd: number;
  outputPerMillionUsd: number;
  checkedAt: string;
  source: string;
}

export interface ResolvedPricing {
  entry: PricingEntry;
  stale: boolean;
}

function isPricingEntry(v: unknown): v is PricingEntry {
  if (typeof v !== "object" || v === null) return false;
  const e = v as Record<string, unknown>;
  return (
    typeof e.inputPerMillionUsd === "number" &&
    typeof e.outputPerMillionUsd === "number" &&
    typeof e.checkedAt === "string" &&
    typeof e.source === "string"
  );
}

/**
 * `<userConfigDir>/pricing.json` から `<provider>/<id>` の価格 entry を
 * 読む consumer contract（#45 が供給元。ここでは読み取りのみ）。
 * file 不在・parse 失敗・model 欠損・entry 不正は「pricing 不明」と
 * して null を返す（throw しない）。
 */
export function loadPricingEntry(opts: {
  userConfigDir: string;
  model: string;
  now?: () => number;
}): ResolvedPricing | null {
  let raw: string;
  try {
    raw = fs.readFileSync(path.join(opts.userConfigDir, PRICING_FILENAME), "utf8");
  } catch {
    return null;
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return null;
  }
  const models = (parsed as { models?: unknown } | null)?.models;
  if (typeof models !== "object" || models === null) return null;
  const entry = (models as Record<string, unknown>)[opts.model];
  if (!isPricingEntry(entry)) return null;

  const checkedMs = Date.parse(entry.checkedAt);
  const stale = Number.isNaN(checkedMs) || (opts.now ?? Date.now)() - checkedMs > PRICING_STALE_MS;
  return { entry, stale };
}
