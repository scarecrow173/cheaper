import { CheaperError } from "@cheaper/core";
import { WorkerRunError, type WorkerRunMeta } from "@cheaper/runtime-pi";
import { describe, expect, it } from "vitest";
import { envelopeJson, errorEnvelope, successEnvelope } from "../src/envelope.js";

const runMeta: WorkerRunMeta = {
  model: { provider: "p", id: "m", tier: "low" },
  usage: { input: 1, output: 2, cacheRead: 0, cacheWrite: 0, totalTokens: 3, costUsd: 0 },
  turns: 1,
  toolCalls: 0,
  rejectedToolCalls: 0,
  repairs: 0,
  transientRetries: 0,
  changes: { files: [], totals: { files: 0, linesAdded: 0, linesDeleted: 0 } },
};

describe("successEnvelope", () => {
  it("builds the DESIGN-0005 success envelope", () => {
    const env = successEnvelope({
      worker: "bulk-reader",
      result: { ok: true },
      meta: runMeta,
      durationMs: 12,
    });
    expect(env).toEqual({
      schemaVersion: 1,
      ok: true,
      worker: "bulk-reader",
      result: { ok: true },
      meta: {
        durationMs: 12,
        model: { provider: "p", id: "m", tier: "low" },
        usage: { input: 1, output: 2, cacheRead: 0, cacheWrite: 0, totalTokens: 3, costUsd: 0 },
        turns: 1,
        toolCalls: 0,
        rejectedToolCalls: 0,
        repairs: 0,
        transientRetries: 0,
        changes: { files: [], totals: { files: 0, linesAdded: 0, linesDeleted: 0 } },
      },
    });
  });
});

describe("errorEnvelope", () => {
  it("serializes CheaperError with code/exitCode/retryable", () => {
    const env = errorEnvelope(new CheaperError("INVALID_REQUEST", "bad", {}), {
      worker: null,
      durationMs: 3,
    });
    expect(env.schemaVersion).toBe(1);
    expect(env.ok).toBe(false);
    expect(env.worker).toBeNull();
    expect(env.error).toMatchObject({
      code: "INVALID_REQUEST",
      message: "bad",
      exitCode: 2,
      retryable: false,
    });
    expect(env.meta.durationMs).toBe(3);
    expect(env.meta.model).toBeNull();
    expect(env.meta.changes).toBeNull();
  });

  it("keeps worker name and serializes CheaperError cause", () => {
    const cause = new CheaperError("TIMEOUT", "t", {});
    const err = new CheaperError("PARTIAL_MUTATION", "partial", {
      cause,
      details: {
        changes: {
          files: [{ path: "a.ts", kind: "created", linesAdded: 1, linesDeleted: 0 }],
          totals: { files: 1, linesAdded: 1, linesDeleted: 0 },
        },
      },
    });
    const env = errorEnvelope(err, { worker: "code-writer", durationMs: 5 });
    expect(env.worker).toBe("code-writer");
    expect(env.error.exitCode).toBe(9);
    expect(env.error.cause).toEqual({ code: "TIMEOUT", message: "t" });
    expect(env.meta.changes).toEqual({
      files: [{ path: "a.ts", kind: "created", linesAdded: 1, linesDeleted: 0 }],
      totals: { files: 1, linesAdded: 1, linesDeleted: 0 },
    });
  });

  it("fills error meta from WorkerRunError.runMeta", () => {
    const err = new WorkerRunError("TIMEOUT", "worker run aborted: timeout", {
      details: { reason: "timeout" },
      runMeta: { ...runMeta, turns: 3, repairs: 1 },
    });
    const env = errorEnvelope(err, { worker: "w", durationMs: 9 });
    expect(env.error.code).toBe("TIMEOUT");
    expect(env.meta.durationMs).toBe(9); // durationMs は常に CLI 計測値
    expect(env.meta.model).toEqual(runMeta.model);
    expect(env.meta.usage).toEqual(runMeta.usage);
    expect(env.meta.turns).toBe(3);
    expect(env.meta.toolCalls).toBe(0);
    expect(env.meta.repairs).toBe(1);
    expect(env.meta.changes).toEqual(runMeta.changes);
  });

  it("prefers runMeta.changes over details.changes", () => {
    const changes = {
      files: [{ path: "b.ts", kind: "modified" as const, linesAdded: 2, linesDeleted: 0 }],
      totals: { files: 1, linesAdded: 2, linesDeleted: 0 },
    };
    const err = new WorkerRunError("PARTIAL_MUTATION", "partial", {
      details: { changes: runMeta.changes },
      runMeta: { ...runMeta, changes },
    });
    const env = errorEnvelope(err, { worker: null, durationMs: 1 });
    expect(env.meta.changes).toEqual(changes);
  });

  it("maps unknown errors to INTERNAL_ERROR 70", () => {
    const env = errorEnvelope(new Error("boom"), { worker: null, durationMs: 1 });
    expect(env.error.code).toBe("INTERNAL_ERROR");
    expect(env.error.exitCode).toBe(70);
    expect(env.error.retryable).toBe(false);
  });

  it("maps non-Error throws to INTERNAL_ERROR 70", () => {
    const env = errorEnvelope("oops", { worker: null, durationMs: 1 });
    expect(env.error.code).toBe("INTERNAL_ERROR");
    expect(env.error.exitCode).toBe(70);
  });
});

describe("envelopeJson", () => {
  it("is a single JSON object followed by exactly one newline", () => {
    const env = successEnvelope({ worker: "w", result: {}, meta: runMeta, durationMs: 0 });
    const out = envelopeJson(env);
    expect(out.endsWith("\n")).toBe(true);
    expect(out.endsWith("\n\n")).toBe(false);
    expect(JSON.parse(out)).toEqual(env);
  });
});
