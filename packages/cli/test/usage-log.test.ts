import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { CheaperError } from "@cheaper/core";
import { WorkerRunError, type WorkerRunMeta } from "@cheaper/runtime-pi";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { errorEnvelope, successEnvelope, utilitySuccessEnvelope } from "../src/envelope.js";
import { appendUsageLog, buildUsageRecord, recordUsage } from "../src/usage-log.js";

const runMeta: WorkerRunMeta = {
  model: { provider: "ollama", id: "qwen3.5:9b", tier: "low" },
  usage: { input: 100, output: 20, cacheRead: 0, cacheWrite: 0, totalTokens: 120, costUsd: 0 },
  turns: 2,
  toolCalls: 1,
  rejectedToolCalls: 0,
  repairs: 0,
  transientRetries: 0,
  changes: { files: [], totals: { files: 0, linesAdded: 0, linesDeleted: 0 } },
};

const savings = { sourceTokens: 800, returnedTokens: 12, avoidedContextTokens: 788 };

const NOW = () => new Date("2026-09-18T00:00:00.000Z");

describe("buildUsageRecord", () => {
  it("builds a run record from a success envelope", () => {
    const env = successEnvelope({
      worker: "bulk-reader",
      result: { findings: [] },
      meta: runMeta,
      durationMs: 42,
    });
    const rec = buildUsageRecord(env, { root: "/ws", now: NOW });
    expect(rec).toEqual({
      schemaVersion: 1,
      ts: "2026-09-18T00:00:00.000Z",
      kind: "run",
      worker: "bulk-reader",
      root: "/ws",
      ok: true,
      durationMs: 42,
      model: { provider: "ollama", id: "qwen3.5:9b", tier: "low" },
      usage: runMeta.usage,
    });
  });

  it("builds a failed run record from an error envelope that carries usage", () => {
    const err = new WorkerRunError("INVALID_OUTPUT", "boom", {
      runMeta: { ...runMeta, turns: 3 },
    });
    const env = errorEnvelope(err, { worker: "doc-writer", durationMs: 9 });
    const rec = buildUsageRecord(env, { root: "/ws", now: NOW });
    expect(rec).toMatchObject({
      kind: "run",
      worker: "doc-writer",
      ok: false,
      durationMs: 9,
      model: runMeta.model,
      usage: runMeta.usage,
      error: { code: "INVALID_OUTPUT" },
    });
  });

  it("carries savings into a run record when meta provides it", () => {
    const env = successEnvelope({
      worker: "bulk-reader",
      result: { findings: [] },
      meta: { ...runMeta, savings },
      durationMs: 42,
    });
    const rec = buildUsageRecord(env, { root: "/ws", now: NOW });
    expect(rec).toMatchObject({ kind: "run", savings });
  });

  it("omits savings when the envelope meta does not provide it", () => {
    const env = successEnvelope({
      worker: "bulk-reader",
      result: {},
      meta: runMeta,
      durationMs: 1,
    });
    const rec = buildUsageRecord(env, { root: "/ws", now: NOW });
    expect(rec).not.toBeNull();
    expect("savings" in (rec as object)).toBe(false);
  });

  it("carries savings into a failed run record from runMeta", () => {
    const err = new WorkerRunError("PARTIAL_MUTATION", "boom", {
      runMeta: {
        ...runMeta,
        savings: { sourceTokens: 500, returnedTokens: 0, avoidedContextTokens: 500 },
      },
    });
    const env = errorEnvelope(err, { worker: "bulk-reader", durationMs: 9 });
    const rec = buildUsageRecord(env, { root: "/ws", now: NOW });
    expect(rec).toMatchObject({
      kind: "run",
      ok: false,
      savings: { sourceTokens: 500, returnedTokens: 0, avoidedContextTokens: 500 },
    });
  });

  it("does not add savings to a count record", () => {
    const env = utilitySuccessEnvelope({ result: { tokens: 10 }, durationMs: 1 });
    const rec = buildUsageRecord(env, { root: "/ws", now: NOW });
    expect(rec).not.toBeNull();
    expect("savings" in (rec as object)).toBe(false);
  });

  it("returns null for an error envelope without usage", () => {
    const env = errorEnvelope(new CheaperError("INVALID_REQUEST", "bad", {}), {
      worker: "bulk-reader",
      durationMs: 1,
    });
    expect(buildUsageRecord(env, { root: "/ws", now: NOW })).toBeNull();
  });

  it("builds a count record from a count-tokens utility envelope", () => {
    const env = utilitySuccessEnvelope({
      result: {
        path: "src/a.ts",
        tokens: 123,
        bytes: 456,
        tokenizer: { name: "x" },
      },
      durationMs: 7,
    });
    const rec = buildUsageRecord(env, { root: "/ws", now: NOW });
    expect(rec).toEqual({
      schemaVersion: 1,
      ts: "2026-09-18T00:00:00.000Z",
      kind: "count",
      worker: null,
      root: "/ws",
      ok: true,
      durationMs: 7,
      result: { path: "src/a.ts", tokens: 123, bytes: 456 },
    });
  });

  it("returns null for a utility envelope without a token count", () => {
    const env = utilitySuccessEnvelope({ result: { status: "ok" }, durationMs: 1 });
    expect(buildUsageRecord(env, { root: "/ws", now: NOW })).toBeNull();
  });
});

describe("appendUsageLog", () => {
  let dir: string;
  beforeEach(() => {
    dir = fs.mkdtempSync(path.join(os.tmpdir(), "usage-log-"));
  });
  afterEach(() => {
    fs.rmSync(dir, { recursive: true, force: true });
  });

  const record = {
    schemaVersion: 1 as const,
    ts: "2026-09-18T00:00:00.000Z",
    kind: "run" as const,
    worker: "bulk-reader",
    root: "/ws",
    ok: true,
    durationMs: 1,
  };

  it("creates the config dir and appends one JSONL line per call", () => {
    const cfg = path.join(dir, "nested", "cfg");
    appendUsageLog({ userConfigDir: cfg, record });
    appendUsageLog({ userConfigDir: cfg, record });
    const lines = fs
      .readFileSync(path.join(cfg, "usage.jsonl"), "utf8")
      .split("\n")
      .filter((l) => l !== "");
    expect(lines).toHaveLength(2);
    expect(JSON.parse(lines[0])).toMatchObject({ kind: "run", worker: "bulk-reader" });
  });
});

describe("recordUsage", () => {
  let dir: string;
  let stderrOut: string[];
  const io = () => ({ write: (s: string) => stderrOut.push(s) });
  beforeEach(() => {
    dir = fs.mkdtempSync(path.join(os.tmpdir(), "usage-log-"));
    stderrOut = [];
  });
  afterEach(() => {
    fs.rmSync(dir, { recursive: true, force: true });
  });

  it("writes a run record into <userConfigDir>/usage.jsonl", () => {
    const env = successEnvelope({
      worker: "code-writer",
      result: {},
      meta: runMeta,
      durationMs: 5,
    });
    recordUsage({ env, root: dir, userConfigDir: dir, stderr: io(), quiet: false, now: NOW });
    const lines = fs.readFileSync(path.join(dir, "usage.jsonl"), "utf8").trim().split("\n");
    expect(lines).toHaveLength(1);
    expect(JSON.parse(lines[0])).toMatchObject({ kind: "run", ok: true });
    expect(stderrOut).toHaveLength(0);
  });

  it("swallows write failures and emits a stderr diagnostic unless quiet", () => {
    // userConfigDir 直下が file になっていて mkdir -p できない状態を作る
    const blocker = path.join(dir, "blocker");
    fs.writeFileSync(blocker, "x");
    const env = successEnvelope({ worker: "w", result: {}, meta: runMeta, durationMs: 1 });
    expect(() =>
      recordUsage({
        env,
        root: dir,
        userConfigDir: path.join(blocker, "cfg"),
        stderr: io(),
        quiet: false,
        now: NOW,
      }),
    ).not.toThrow();
    expect(stderrOut.join("")).toContain("usage-log:");
    stderrOut = [];
    recordUsage({
      env,
      root: dir,
      userConfigDir: path.join(blocker, "cfg"),
      stderr: io(),
      quiet: true,
      now: NOW,
    });
    expect(stderrOut).toHaveLength(0);
  });
});
