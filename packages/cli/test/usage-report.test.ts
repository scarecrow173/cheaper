import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { UsageLogRecord } from "../src/usage-log.js";
import {
  aggregate,
  buildUsageReport,
  filterRunRecords,
  readUsageRecords,
  splitModelRef,
} from "../src/usage-report.js";

const NOW = Date.parse("2026-09-19T12:00:00.000Z");
const now = () => NOW;

function runRecord(overrides: Partial<UsageLogRecord> = {}): UsageLogRecord {
  return {
    schemaVersion: 1,
    ts: "2026-09-19T10:00:00.000Z",
    kind: "run",
    worker: "bulk-reader",
    root: "/ws",
    ok: true,
    durationMs: 100,
    model: { provider: "ollama", id: "qwen3.5:9b", tier: "low" },
    usage: { input: 100, output: 20, cacheRead: 0, cacheWrite: 0, totalTokens: 120, costUsd: 0 },
    turns: 2,
    toolCalls: 1,
    ...overrides,
  };
}

describe("readUsageRecords", () => {
  let dir: string;
  beforeEach(() => {
    dir = fs.mkdtempSync(path.join(os.tmpdir(), "usage-report-"));
  });
  afterEach(() => {
    fs.rmSync(dir, { recursive: true, force: true });
  });

  it("returns an empty list when usage.jsonl does not exist", () => {
    expect(readUsageRecords(dir)).toEqual([]);
  });

  it("skips malformed lines and keeps valid records", () => {
    const lines = [
      JSON.stringify(runRecord()),
      "not json {",
      JSON.stringify({ kind: "unknown" }),
      "",
      JSON.stringify({ schemaVersion: 1, ts: "t", kind: "count", worker: null, root: "/w" }),
    ].join("\n");
    fs.writeFileSync(path.join(dir, "usage.jsonl"), lines);
    const records = readUsageRecords(dir);
    expect(records).toHaveLength(2);
    expect(records.map((r) => r.kind)).toEqual(["run", "count"]);
  });
});

describe("filterRunRecords", () => {
  const records = [
    runRecord({ worker: "bulk-reader" }),
    runRecord({
      worker: "code-writer",
      model: { provider: "openai", id: "gpt-5", tier: "low" },
    }),
    runRecord({ ts: "2026-09-01T00:00:00.000Z" }),
    { schemaVersion: 1, ts: "t", kind: "count", worker: null, root: "/w", ok: true } as const,
  ];

  it("keeps only run records by default", () => {
    expect(filterRunRecords(records, {}, now)).toHaveLength(3);
  });

  it("filters by since duration against record ts", () => {
    const filtered = filterRunRecords(records, { sinceMs: 48 * 3_600_000 }, now);
    expect(filtered).toHaveLength(2);
  });

  it("filters by worker, provider, and model ref", () => {
    expect(filterRunRecords(records, { worker: "code-writer" }, now)).toHaveLength(1);
    expect(filterRunRecords(records, { provider: "openai" }, now)).toHaveLength(1);
    expect(filterRunRecords(records, { model: "openai/gpt-5" }, now)).toHaveLength(1);
    expect(filterRunRecords(records, { model: "ollama/qwen3.5:9b" }, now)).toHaveLength(2);
  });
});

describe("aggregate", () => {
  it("sums usage, turns, toolCalls, duration, and measured savings", () => {
    const records = [
      runRecord({ savings: { sourceTokens: 800, returnedTokens: 12, avoidedContextTokens: 788 } }),
      runRecord({
        ok: false,
        durationMs: null,
        savings: { sourceTokens: 100, returnedTokens: 0, avoidedContextTokens: 100 },
      }),
      runRecord(), // savings なし（旧 record 相当）
    ];
    const agg = aggregate(records);
    expect(agg.records).toBe(3);
    expect(agg.ok).toBe(2);
    expect(agg.errors).toBe(1);
    expect(agg.usage.input).toBe(300);
    expect(agg.usage.totalTokens).toBe(360);
    expect(agg.turns).toBe(6);
    expect(agg.toolCalls).toBe(3);
    expect(agg.durationMs).toBe(200);
    expect(agg.measured).toEqual({
      runs: 2,
      sourceTokens: 900,
      returnedTokens: 12,
      avoidedContextTokens: 888,
    });
  });
});

describe("splitModelRef", () => {
  it("splits provider/id on the first slash", () => {
    expect(splitModelRef("openai/gpt-5")).toEqual({ provider: "openai", id: "gpt-5" });
    expect(splitModelRef("openrouter/a/b")).toEqual({ provider: "openrouter", id: "a/b" });
  });
  it("rejects refs without a valid provider/id", () => {
    expect(splitModelRef("gpt-5")).toBeNull();
    expect(splitModelRef("/x")).toBeNull();
    expect(splitModelRef("x/")).toBeNull();
  });
});

describe("buildUsageReport", () => {
  let dir: string;
  beforeEach(() => {
    dir = fs.mkdtempSync(path.join(os.tmpdir(), "usage-report-"));
  });
  afterEach(() => {
    fs.rmSync(dir, { recursive: true, force: true });
  });

  function seed(records: UsageLogRecord[]): void {
    fs.writeFileSync(
      path.join(dir, "usage.jsonl"),
      `${records.map((r) => JSON.stringify(r)).join("\n")}\n`,
    );
  }

  it("aggregates with no estimated section when --compare-model is absent", () => {
    seed([
      runRecord({ savings: { sourceTokens: 10, returnedTokens: 2, avoidedContextTokens: 8 } }),
    ]);
    const report = buildUsageReport({ userConfigDir: dir, filters: {}, now });
    expect(report.records).toBe(1);
    expect(report.estimated).toBeUndefined();
  });

  it("omits estimated when pricing.json or the model entry is missing", () => {
    seed([runRecord()]);
    const report = buildUsageReport({
      userConfigDir: dir,
      filters: {},
      compareModel: "openai/gpt-5",
      now,
    });
    expect(report.estimated).toBeUndefined();
  });

  it("omits estimated when pricing is stale", () => {
    seed([
      runRecord({ savings: { sourceTokens: 10, returnedTokens: 2, avoidedContextTokens: 8 } }),
    ]);
    fs.writeFileSync(
      path.join(dir, "pricing.json"),
      JSON.stringify({
        models: {
          "openai/gpt-5": {
            inputPerMillionUsd: 3,
            outputPerMillionUsd: 15,
            checkedAt: "2026-01-01T00:00:00.000Z",
            source: "test",
          },
        },
      }),
    );
    const report = buildUsageReport({
      userConfigDir: dir,
      filters: {},
      compareModel: "openai/gpt-5",
      now,
    });
    expect(report.estimated).toBeUndefined();
  });

  it("computes estimated savings from fresh pricing with provenance", () => {
    seed([
      runRecord({
        usage: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, totalTokens: 0, costUsd: 0.001 },
        savings: {
          sourceTokens: 1_000_000,
          returnedTokens: 100_000,
          avoidedContextTokens: 900_000,
        },
      }),
    ]);
    fs.writeFileSync(
      path.join(dir, "pricing.json"),
      JSON.stringify({
        models: {
          "openai/gpt-5": {
            inputPerMillionUsd: 3,
            outputPerMillionUsd: 15,
            checkedAt: "2026-09-18T00:00:00.000Z",
            source: "test-feed",
          },
        },
      }),
    );
    const report = buildUsageReport({
      userConfigDir: dir,
      filters: {},
      compareModel: "openai/gpt-5",
      now,
    });
    // parentEquiv = (1e6 * 3 + 1e5 * 15) / 1e6 = 3 + 1.5 = 4.5
    expect(report.estimated).toEqual({
      compareModel: "openai/gpt-5",
      estimatedParentEquivalentCostUsd: 4.5,
      estimatedSavingsUsd: 4.5 - 0.001,
      estimatedSavingsPercent: ((4.5 - 0.001) / 4.5) * 100,
      pricing: { source: "test-feed", checkedAt: "2026-09-18T00:00:00.000Z", stale: false },
    });
  });
});
