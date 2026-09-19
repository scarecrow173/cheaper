import fs from "node:fs";
import path from "node:path";
import type { PricingSnapshot, PricingSnapshotEntry } from "@cheaper/core";

export const PRICING_FILENAME = "pricing.json";

function isPlainObject(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

function isNonNegativeNumber(v: unknown): v is number {
  return typeof v === "number" && Number.isFinite(v) && v >= 0;
}

/**
 * `<userConfigDir>/pricing.json` を user 提供の pricing snapshot として読む。
 * 形式は core の bundled snapshot（`PricingSnapshot`: `{version, updatedAt,
 * models: {"<provider>/<id>": {input, output, ...}}}`）と同一で、core
 * `resolvePricing` の snapshot 引数としてそのまま渡せる bridge
 * （ADR-034rzI0Yqw3L8AFM9diZct の snapshot contract に整合）。
 * file 不在・parse 失敗・shape 不正は null を返す（読み取り専用・fail soft）。
 * entry 単位の不正はその entry のみ skip する。
 */
export function loadPricingSnapshot(userConfigDir: string): PricingSnapshot | null {
  let raw: string;
  try {
    raw = fs.readFileSync(path.join(userConfigDir, PRICING_FILENAME), "utf8");
  } catch {
    return null;
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return null;
  }
  if (!isPlainObject(parsed)) return null;
  const { version, updatedAt, models } = parsed;
  if (typeof version !== "string" || typeof updatedAt !== "string") return null;
  if (!isPlainObject(models)) return null;

  const entries: Record<string, PricingSnapshotEntry> = {};
  for (const [key, e] of Object.entries(models)) {
    if (!isPlainObject(e)) continue;
    const { input, output, cacheRead, cacheWrite, updatedAt: entryUpdatedAt } = e;
    if (!isNonNegativeNumber(input) || !isNonNegativeNumber(output)) continue;
    entries[key] = {
      input,
      output,
      ...(isNonNegativeNumber(cacheRead) ? { cacheRead } : {}),
      ...(isNonNegativeNumber(cacheWrite) ? { cacheWrite } : {}),
      ...(typeof entryUpdatedAt === "string" ? { updatedAt: entryUpdatedAt } : {}),
    };
  }
  return { version, updatedAt, models: entries };
}
