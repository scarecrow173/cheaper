import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import type { ModelInfo } from "@cheaper/core";
import { afterEach, describe, expect, it } from "vitest";
import type { UsageLogRecord } from "../src/usage-log.js";
import { buildUsageReport, splitModelRef } from "../src/usage-report.js";

let dir: string | null = null;
afterEach(() => {
  if (dir !== null) fs.rmSync(dir, { recursive: true, force: true });
  dir = null;
});

function makeDir(): string {
  dir = fs.mkdtempSync(path.join(os.tmpdir(), "cheaper-usage-report-"));
  return dir;
}

function writeLog(records: UsageLogRecord[]): string {
  const d = makeDir();
  fs.writeFileSync(
    path.join(d, "usage.jsonl"),
    `${records.map((r) => JSON.stringify(r)).join("\n")}\n`,
    "utf8",
  );
  return d;
}

function writePricing(models: Record<string, unknown>, updatedAt = "2026-09-18"): void {
  fs.writeFileSync(
    path.join(dir as string, "pricing.json"),
    JSON.stringify({ version: "test", updatedAt, models }),
    "utf8",
  );
}

const NOW = () => Date.parse("2026-09-20T00:00:00.000Z");

function run(overrides: Partial<UsageLogRecord> = {}): UsageLogRecord {
  return {
    schemaVersion: 1,
    ts: "2026-09-19T00:00:00.000Z",
    kind: "run",
    worker: "bulk-reader",
    root: "/ws",
    ok: true,
    durationMs: 10,
    model: { provider: "ollama", id: "qwen3.5:9b", tier: "low" },
    usage: { input: 100, output: 20, cacheRead: 0, cacheWrite: 0, totalTokens: 120, costUsd: 0.01 },
    savings: {
      sourceTokens: 800,
      returnedTokens: 50,
      avoidedContextTokens: 750,
      directlyComparable: true,
    },
    ...overrides,
  };
}

const catalogModel: ModelInfo = {
  provider: "openai",
  id: "gpt-5",
  cost: { input: 2, output: 8 },
};

describe("splitModelRef", () => {
  it("splits provider/id", () => {
    expect(splitModelRef("openai/gpt-5")).toEqual({ provider: "openai", id: "gpt-5" });
    expect(splitModelRef("ollama/qwen3.5:9b")).toEqual({ provider: "ollama", id: "qwen3.5:9b" });
    expect(splitModelRef("noslash")).toBeNull();
    expect(splitModelRef("/id")).toBeNull();
    expect(splitModelRef("p/")).toBeNull();
  });
});

describe("buildUsageReport", () => {
  it("aggregates actual usage only when no savings is recorded", () => {
    const d = writeLog([{ ...run(), savings: undefined }]);
    const report = buildUsageReport({ userConfigDir: d, filters: {}, now: NOW });
    expect(report.records).toBe(1);
    expect(report.measured).toEqual({
      runs: 0,
      sourceTokens: 0,
      returnedTokens: 0,
      avoidedContextTokens: 0,
      comparable: { runs: 0, sourceTokens: 0, returnedTokens: 0, costUsd: 0 },
    });
    expect(report.estimated).toBeUndefined();
  });

  it("counts measured offload for all savings-bearing runs including failures", () => {
    const d = writeLog([
      run(),
      run({
        ok: false,
        savings: {
          sourceTokens: 100,
          returnedTokens: 0,
          avoidedContextTokens: 100,
          directlyComparable: true,
        },
      }),
    ]);
    const report = buildUsageReport({ userConfigDir: d, filters: {}, now: NOW });
    expect(report.records).toBe(2);
    expect(report.ok).toBe(1);
    expect(report.errors).toBe(1);
    expect(report.measured).toEqual({
      runs: 2,
      sourceTokens: 900,
      returnedTokens: 50,
      avoidedContextTokens: 850,
      comparable: { runs: 2, sourceTokens: 900, returnedTokens: 50, costUsd: 0.02 },
    });
  });

  it("keeps non-comparable workers in measured but excludes them from comparable", () => {
    const d = writeLog([
      run(),
      run({
        worker: "code-writer",
        savings: {
          sourceTokens: 400,
          returnedTokens: 10,
          avoidedContextTokens: 390,
          directlyComparable: false,
        },
      }),
    ]);
    const report = buildUsageReport({ userConfigDir: d, filters: {}, now: NOW });
    expect(report.measured.runs).toBe(2);
    expect(report.measured.sourceTokens).toBe(1200);
    expect(report.measured.comparable).toEqual({
      runs: 1,
      sourceTokens: 800,
      returnedTokens: 50,
      costUsd: 0.01,
    });
  });

  it("treats legacy savings records without directlyComparable as non-comparable", () => {
    // 旧 schema record は directlyComparable を持たない（非 comparable 扱い）。
    const legacy = run({
      savings: { sourceTokens: 800, returnedTokens: 50, avoidedContextTokens: 750 } as never,
    });
    const d = writeLog([legacy]);
    const report = buildUsageReport({ userConfigDir: d, filters: {}, now: NOW });
    expect(report.measured.runs).toBe(1);
    expect(report.measured.comparable.runs).toBe(0);
  });

  it("emits no estimated section without --compare-model", () => {
    const d = writeLog([run()]);
    const report = buildUsageReport({ userConfigDir: d, filters: {}, now: NOW });
    expect(report.estimated).toBeUndefined();
  });

  it("emits no estimated section when the compare model has no pricing", () => {
    const d = writeLog([run()]);
    const report = buildUsageReport({
      userConfigDir: d,
      filters: {},
      compareModel: "openai/unknown-xyz-99",
      now: NOW,
    });
    expect(report.estimated).toBeUndefined();
  });

  it("uses runtime catalog pricing when compareCatalogModel carries cost", () => {
    const d = writeLog([run()]);
    const report = buildUsageReport({
      userConfigDir: d,
      filters: {},
      compareModel: "openai/gpt-5",
      compareCatalogModel: catalogModel,
      now: NOW,
    });
    const e = report.estimated;
    expect(e).toBeDefined();
    expect(e?.pricing.source).toBe("runtime-catalog");
    // parentEquiv = (800*2 + 50*8) / 1e6 = 0.002 ; savings = 0.002 - 0.01 = -0.008
    expect(e?.estimatedParentEquivalentCostUsd).toBeCloseTo(0.002);
    expect(e?.estimatedSavingsUsd).toBeCloseTo(-0.008);
    // parentEquiv > 0 のため percent は算出される（negative savings も含む）
    expect(e?.estimatedSavingsPercent).toBeCloseTo(-400);
  });

  it("falls back to a user pricing.json snapshot via the core contract", () => {
    const d = writeLog([run()]);
    writePricing({ "test-prov/test-model": { input: 2, output: 8 } });
    const report = buildUsageReport({
      userConfigDir: d,
      filters: {},
      compareModel: "test-prov/test-model",
      now: NOW,
    });
    const e = report.estimated;
    expect(e).toBeDefined();
    expect(e?.pricing.source).toBe("pricing.json");
    expect(e?.pricing.updatedAt).toBe("2026-09-18");
  });

  it("prefers runtime catalog pricing over pricing.json", () => {
    const d = writeLog([run()]);
    writePricing({ "openai/gpt-5": { input: 100, output: 100 } });
    const report = buildUsageReport({
      userConfigDir: d,
      filters: {},
      compareModel: "openai/gpt-5",
      compareCatalogModel: catalogModel,
      now: NOW,
    });
    expect(report.estimated?.pricing.source).toBe("runtime-catalog");
    expect(report.estimated?.estimatedParentEquivalentCostUsd).toBeCloseTo(0.002);
  });

  it("emits no estimated section for stale snapshot pricing", () => {
    const d = writeLog([run()]);
    writePricing({ "test-prov/test-model": { input: 2, output: 8 } }, "2020-01-01");
    const report = buildUsageReport({
      userConfigDir: d,
      filters: {},
      compareModel: "test-prov/test-model",
      now: NOW,
    });
    expect(report.estimated).toBeUndefined();
  });

  it("emits no estimated section when only non-comparable workers are present", () => {
    const writer = run({
      worker: "code-writer",
      savings: {
        sourceTokens: 800,
        returnedTokens: 50,
        avoidedContextTokens: 750,
        directlyComparable: false,
      },
    });
    const d = writeLog([writer]);
    writePricing({ "test-prov/test-model": { input: 2, output: 8 } });
    const report = buildUsageReport({
      userConfigDir: d,
      filters: {},
      compareModel: "test-prov/test-model",
      now: NOW,
    });
    expect(report.measured.runs).toBe(1);
    expect(report.estimated).toBeUndefined();
  });

  it("computes estimated savings only from the comparable subset", () => {
    const d = writeLog([
      run(),
      run({
        worker: "code-writer",
        usage: { input: 1, output: 1, cacheRead: 0, cacheWrite: 0, totalTokens: 2, costUsd: 5 },
        savings: {
          sourceTokens: 9000,
          returnedTokens: 10,
          avoidedContextTokens: 8990,
          directlyComparable: false,
        },
      }),
    ]);
    writePricing({ "test-prov/test-model": { input: 2, output: 8 } });
    const report = buildUsageReport({
      userConfigDir: d,
      filters: {},
      compareModel: "test-prov/test-model",
      now: NOW,
    });
    const e = report.estimated;
    expect(e).toBeDefined();
    // comparable subset のみ: parentEquiv=(800*2+50*8)/1e6=0.002, worker=0.01
    expect(e?.estimatedParentEquivalentCostUsd).toBeCloseTo(0.002);
    expect(e?.estimatedSavingsUsd).toBeCloseTo(-0.008);
  });

  it("applies sinceMs/worker/model filters before aggregating", () => {
    const d = writeLog([
      run(),
      run({ worker: "doc-writer", savings: undefined }),
      run({ ts: "2020-01-01T00:00:00.000Z" }),
    ]);
    const report = buildUsageReport({
      userConfigDir: d,
      filters: { worker: "bulk-reader", sinceMs: 7 * 24 * 3_600_000 },
      now: NOW,
    });
    expect(report.records).toBe(1);
  });

  it("applies provider and model filters to run records", () => {
    const other = run({ model: { provider: "openai", id: "gpt-5-mini", tier: "low" } });
    const d = writeLog([run(), other]);
    expect(
      buildUsageReport({ userConfigDir: d, filters: { provider: "ollama" }, now: NOW }).records,
    ).toBe(1);
    expect(
      buildUsageReport({ userConfigDir: d, filters: { model: "openai/gpt-5-mini" }, now: NOW })
        .records,
    ).toBe(1);
  });

  it("skips malformed JSONL lines and missing log file", () => {
    const d = makeDir();
    fs.writeFileSync(path.join(d, "usage.jsonl"), `{broken\n${JSON.stringify(run())}\n`);
    expect(buildUsageReport({ userConfigDir: d, filters: {}, now: NOW }).records).toBe(1);
    const empty = makeDir();
    fs.rmSync(path.join(empty, "usage.jsonl"), { force: true });
    expect(buildUsageReport({ userConfigDir: empty, filters: {}, now: NOW }).records).toBe(0);
  });
});
