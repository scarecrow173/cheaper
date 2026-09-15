import { CheaperError } from "@cheaper/core";
import type { WorkerRunMeta } from "@cheaper/runtime-pi";
import { describe, expect, it } from "vitest";
import { errorEnvelope, successEnvelope } from "../src/envelope.js";
import { formatText } from "../src/text-format.js";

const runMeta: WorkerRunMeta = {
  model: { provider: "p", id: "m", tier: "low" },
  usage: { input: 1, output: 2, cacheRead: 0, cacheWrite: 0, totalTokens: 3, costUsd: 0.0012 },
  turns: 2,
  toolCalls: 1,
  rejectedToolCalls: 0,
  repairs: 0,
  transientRetries: 0,
  changes: {
    files: [{ path: "src/a.ts", kind: "created", linesAdded: 10, linesDeleted: 0 }],
    totals: { files: 1, linesAdded: 10, linesDeleted: 0 },
  },
};

describe("formatText", () => {
  it("starts with `ok` and shows worker/model/usage/changes table plus result JSON", () => {
    const env = successEnvelope({
      worker: "cheap-writer",
      result: { summary: "done" },
      meta: runMeta,
      durationMs: 42,
    });
    const out = formatText(env);
    const lines = out.split("\n");
    expect(lines[0]).toBe("ok");
    expect(out).toContain("worker:");
    expect(out).toContain("cheap-writer");
    expect(out).toContain("p/m");
    expect(out).toContain("tier low");
    expect(out).toContain("changes:");
    expect(out).toContain("src/a.ts");
    expect(out).toContain(JSON.stringify({ summary: "done" }, null, 2));
    expect(out.endsWith("\n")).toBe(true);
  });

  it("starts with `error <code>` for error envelopes", () => {
    const env = errorEnvelope(new CheaperError("INVALID_INPUT", "bad input", {}), {
      worker: "bulk-reader",
      durationMs: 1,
    });
    const out = formatText(env);
    expect(out.split("\n")[0]).toBe("error INVALID_INPUT");
    expect(out).toContain("retryable=false");
    expect(out).toContain('"code": "INVALID_INPUT"');
    expect(out.endsWith("\n")).toBe(true);
  });

  it("handles null meta fields on errors", () => {
    const env = errorEnvelope(new CheaperError("WORKER_NOT_FOUND", "nf", {}), {
      worker: null,
      durationMs: null,
    });
    const out = formatText(env);
    expect(out.split("\n")[0]).toBe("error WORKER_NOT_FOUND");
    expect(out).toContain("worker:");
  });
});
