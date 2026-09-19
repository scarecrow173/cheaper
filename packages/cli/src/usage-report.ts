import fs from "node:fs";
import path from "node:path";
import {
  type ModelInfo,
  type ModelPricing,
  type PricingSource,
  resolvePricing,
} from "@cheaper/core";
import { loadPricingSnapshot } from "./pricing.js";
import { USAGE_LOG_FILENAME, type UsageLogRecord } from "./usage-log.js";

/** snapshot の updatedAt がこの期間より古い pricing は stale とみなす。 */
export const PRICING_STALE_MS = 30 * 24 * 3_600_000;

export interface UsageFilters {
  /** ts >= now - sinceMs の record のみ対象。 */
  sinceMs?: number;
  worker?: string;
  provider?: string;
  /** `<provider>/<id>` 形式。 */
  model?: string;
}

/** `cheaper usage` の集計 result（utility envelope の result）。 */
export interface UsageReport {
  /** filter 後の run record 件数。 */
  records: number;
  ok: number;
  errors: number;
  usage: {
    input: number;
    output: number;
    cacheRead: number;
    cacheWrite: number;
    totalTokens: number;
    costUsd: number;
  };
  turns: number;
  toolCalls: number;
  durationMs: number;
  /** savings を持つ record のみを母数とする実測 offload 合計。 */
  measured: {
    runs: number;
    sourceTokens: number;
    returnedTokens: number;
    avoidedContextTokens: number;
    /**
     * `directlyComparable` worker のみの subset。estimated の母数。
     * write 系 worker（code-writer / doc-writer 等）の run は
     * 直接比較が成立しないため含まない。
     */
    comparable: {
      runs: number;
      sourceTokens: number;
      returnedTokens: number;
      costUsd: number;
    };
  };
  estimated?: {
    compareModel: string;
    estimatedParentEquivalentCostUsd: number;
    estimatedSavingsUsd: number;
    estimatedSavingsPercent: number | null;
    /** pricing の provenance。`pricing.json` は user 提供 snapshot。 */
    pricing: {
      source: PricingSource | "pricing.json";
      updatedAt?: string;
      snapshotVersion?: string;
      stale: boolean;
    };
  };
}

function isRecord(v: unknown): v is UsageLogRecord {
  if (typeof v !== "object" || v === null) return false;
  const k = (v as { kind?: unknown }).kind;
  return k === "run" || k === "count";
}

/**
 * `<userConfigDir>/usage.jsonl` を per-line parse して返す。
 * file 不在・破損行はエラーにせず skip する（read-only 集計）。
 */
export function readUsageRecords(userConfigDir: string): UsageLogRecord[] {
  let raw: string;
  try {
    raw = fs.readFileSync(path.join(userConfigDir, USAGE_LOG_FILENAME), "utf8");
  } catch {
    return [];
  }
  const records: UsageLogRecord[] = [];
  for (const line of raw.split("\n")) {
    if (line.trim() === "") continue;
    try {
      const parsed: unknown = JSON.parse(line);
      if (isRecord(parsed)) records.push(parsed);
    } catch {
      // 破損行は skip
    }
  }
  return records;
}

/** `provider/id` 文字列を分解する。`/` を含まない場合は null。 */
export function splitModelRef(ref: string): { provider: string; id: string } | null {
  const i = ref.indexOf("/");
  if (i <= 0 || i === ref.length - 1) return null;
  return { provider: ref.slice(0, i), id: ref.slice(i + 1) };
}

export function filterRunRecords(
  records: UsageLogRecord[],
  filters: UsageFilters,
  now: () => number,
): UsageLogRecord[] {
  const modelRef = filters.model !== undefined ? splitModelRef(filters.model) : undefined;
  return records.filter((r) => {
    if (r.kind !== "run") return false;
    if (filters.sinceMs !== undefined) {
      const ts = Date.parse(r.ts);
      if (Number.isNaN(ts) || ts < now() - filters.sinceMs) return false;
    }
    if (filters.worker !== undefined && r.worker !== filters.worker) return false;
    if (filters.provider !== undefined && r.model?.provider !== filters.provider) return false;
    if (modelRef !== undefined && modelRef !== null) {
      if (r.model?.provider !== modelRef.provider || r.model.id !== modelRef.id) return false;
    }
    return true;
  });
}

export function aggregate(records: UsageLogRecord[]): Omit<UsageReport, "estimated"> {
  const report = {
    records: records.length,
    ok: 0,
    errors: 0,
    usage: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, totalTokens: 0, costUsd: 0 },
    turns: 0,
    toolCalls: 0,
    durationMs: 0,
    measured: {
      runs: 0,
      sourceTokens: 0,
      returnedTokens: 0,
      avoidedContextTokens: 0,
      comparable: { runs: 0, sourceTokens: 0, returnedTokens: 0, costUsd: 0 },
    },
  };
  for (const r of records) {
    if (r.ok) report.ok += 1;
    else report.errors += 1;
    if (r.usage !== undefined) {
      report.usage.input += r.usage.input;
      report.usage.output += r.usage.output;
      report.usage.cacheRead += r.usage.cacheRead;
      report.usage.cacheWrite += r.usage.cacheWrite;
      report.usage.totalTokens += r.usage.totalTokens;
      report.usage.costUsd += r.usage.costUsd;
    }
    // 旧 record には turns/toolCalls が無い（undefined → 0 加算）
    report.turns += r.turns ?? 0;
    report.toolCalls += r.toolCalls ?? 0;
    report.durationMs += r.durationMs ?? 0;
    if (r.savings !== undefined) {
      report.measured.runs += 1;
      report.measured.sourceTokens += r.savings.sourceTokens;
      report.measured.returnedTokens += r.savings.returnedTokens;
      report.measured.avoidedContextTokens += r.savings.avoidedContextTokens;
      // 直接比較が成立する worker の run のみ estimated の母数にする。
      // 旧 record（flag 不在）や write 系 worker は comparable に含めない。
      if (r.savings.directlyComparable === true) {
        report.measured.comparable.runs += 1;
        report.measured.comparable.sourceTokens += r.savings.sourceTokens;
        report.measured.comparable.returnedTokens += r.savings.returnedTokens;
        report.measured.comparable.costUsd += r.usage?.costUsd ?? 0;
      }
    }
  }
  return report;
}

/**
 * comparison model の pricing が解決でき、かつ直接比較可能な measured
 * run がある場合のみ estimated を返す。
 * parentEquiv = (comparable.sourceTokens * input + comparable.returnedTokens
 * * output) / 1e6 で「同じ量を親 model で直接処理した場合」の概算を出し、
 * comparable run の実 worker cost との差を savings とする。
 * `pricing.stale`（updatedAt が PRICING_STALE_MS 超）では算出しない。
 */
export function estimate(
  agg: Omit<UsageReport, "estimated">,
  pricing: ModelPricing & { stale: boolean; provenance: PricingSource | "pricing.json" },
  compareModel: string,
): UsageReport["estimated"] {
  if (pricing.stale) return undefined;
  const c = agg.measured.comparable;
  if (c.runs === 0) return undefined;
  const parentEquiv = (c.sourceTokens * pricing.input + c.returnedTokens * pricing.output) / 1e6;
  const savings = parentEquiv - c.costUsd;
  return {
    compareModel,
    estimatedParentEquivalentCostUsd: parentEquiv,
    estimatedSavingsUsd: savings,
    estimatedSavingsPercent: parentEquiv > 0 ? (savings / parentEquiv) * 100 : null,
    pricing: {
      source: pricing.provenance,
      ...(pricing.updatedAt !== undefined ? { updatedAt: pricing.updatedAt } : {}),
      ...(pricing.snapshotVersion !== undefined
        ? { snapshotVersion: pricing.snapshotVersion }
        : {}),
      stale: false,
    },
  };
}

/** snapshot 由来 pricing の updatedAt が stale か（updatedAt 不在＝runtime-catalog は stale にしない）。 */
function isStalePricing(pricing: ModelPricing, now: () => number): boolean {
  if (pricing.updatedAt === undefined) return false;
  const updatedMs = Date.parse(pricing.updatedAt);
  return Number.isNaN(updatedMs) || now() - updatedMs > PRICING_STALE_MS;
}

/**
 * usage.jsonl → filter → 集計 →（compare-model + pricing が揃い、
 * 直接比較可能な run があるときのみ）estimated まで一括で組み立てる。
 *
 * pricing は core の pricing layer（`resolvePricing`）で解決する:
 * runtime catalog（`compareCatalogModel.cost`）→ user 提供の
 * `<userConfigDir>/pricing.json` snapshot → bundled snapshot の順。
 */
export function buildUsageReport(opts: {
  userConfigDir: string;
  filters: UsageFilters;
  compareModel?: string | undefined;
  /**
   * `--compare-model` の runtime catalog 観測（`toModelInfo` 済み）。
   * 未取得・catalog に無い場合は省略し、snapshot のみが参照される。
   */
  compareCatalogModel?: ModelInfo | undefined;
  now?: () => number;
}): UsageReport {
  const now = opts.now ?? Date.now;
  const records = filterRunRecords(readUsageRecords(opts.userConfigDir), opts.filters, now);
  const agg = aggregate(records);
  if (opts.compareModel === undefined) return agg;
  const ref = splitModelRef(opts.compareModel);
  if (ref === null) return agg;
  const model: ModelInfo = opts.compareCatalogModel ?? { provider: ref.provider, id: ref.id };

  const userSnapshot = loadPricingSnapshot(opts.userConfigDir);
  // runtime-catalog cost は resolvePricing 内で最優先される。
  // snapshot 由来が user file から返った場合は provenance を "pricing.json"
  // に読み替える。
  let pricing: ModelPricing | undefined;
  let provenance: PricingSource | "pricing.json" | undefined;
  if (userSnapshot !== null) {
    pricing = resolvePricing(model, userSnapshot);
    if (pricing !== undefined) {
      provenance = pricing.source === "bundled-snapshot" ? "pricing.json" : pricing.source;
    }
  }
  pricing ??= resolvePricing(model);
  provenance ??= pricing?.source;
  if (pricing === undefined || provenance === undefined) return agg;

  const estimated = estimate(
    agg,
    { ...pricing, stale: isStalePricing(pricing, now), provenance },
    opts.compareModel,
  );
  return estimated === undefined ? agg : { ...agg, estimated };
}
