import { readFile, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { parseCliArgs } from "../src/args.js";
import { type CliHooks, runCli } from "../src/cli.js";
import { makeIsolatedEnv } from "./e2e/helpers.js";

describe("workflow run public contract", () => {
  let isolated: Awaited<ReturnType<typeof makeIsolatedEnv>>;
  beforeEach(async () => {
    isolated = await makeIsolatedEnv();
  });
  afterEach(async () => {
    await rm(isolated.dir, { recursive: true, force: true });
  });

  async function invoke(
    definition: unknown,
    options: string[] = [],
    input?: string,
    hooks: Partial<CliHooks> = {},
  ) {
    await writeFile(path.join(isolated.ws, "workflow.json"), JSON.stringify(definition));
    const stdout: string[] = [];
    const stderr: string[] = [];
    const createModelRuntime = vi.fn(() => {
      throw new Error("unexpected model initialization");
    });
    const code = await runCli(
      ["workflow", "run", "workflow.json", ...options],
      {
        cwd: isolated.ws,
        env: isolated.env,
        stdin: {
          async *[Symbol.asyncIterator]() {
            if (input === undefined) throw new Error("unexpected stdin");
            yield input;
          },
        },
        stdout: { write: (s) => stdout.push(s) },
        stderr: { write: (s) => stderr.push(s) },
      },
      { createModelRuntime, ...hooks },
    );
    expect(stdout).toHaveLength(1);
    if (!hooks.createModelRuntime) expect(createModelRuntime).not.toHaveBeenCalled();
    return { code, stdout: stdout[0] ?? "", stderr: stderr.join("") };
  }

  it("distinguishes omitted input from explicit stdin and rejects unrelated flags", () => {
    expect(parseCliArgs(["workflow", "run", "x.json"])).toMatchObject({
      kind: "workflow-run",
      input: undefined,
    });
    expect(parseCliArgs(["workflow", "run", "x.json", "--input", "-"])).toMatchObject({
      input: "-",
    });
    for (const flag of ["--model", "--worker", "--provider", "--worker-file", "--since"]) {
      expect(() => parseCliArgs(["workflow", "run", "x.json", flag, "x"])).toThrow();
    }
  });

  it("omits stdin and preserves false, zero, null and tokens without usage records", async () => {
    const r = await invoke({
      schemaVersion: 1,
      inputs: {},
      jobs: {},
      outputs: {
        falsy: { schema: { type: "boolean" }, value: { $literal: false } },
        tokens: { schema: { type: "number" }, value: { $literal: 0 } },
        empty: { schema: { type: "null" }, value: { $literal: null } },
      },
    });
    expect(r.code).toBe(0);
    expect(JSON.parse(r.stdout)).toMatchObject({
      workflow: true,
      worker: null,
      ok: true,
      result: { falsy: false, tokens: 0, empty: null },
      jobs: [],
    });
    const configDir = isolated.env.CHEAPER_CONFIG_DIR;
    expect(configDir).toBeDefined();
    await expect(readFile(path.join(configDir ?? "", "usage.jsonl"))).rejects.toMatchObject({
      code: "ENOENT",
    });
  });

  it("publishes bounded artifact metadata as unavailable without a top-level preview", async () => {
    await writeFile(path.join(isolated.ws, "published.txt"), "PRIVATE_ARTIFACT_BODY");
    const definition = {
      schemaVersion: 1,
      inputs: {},
      jobs: {
        publish: {
          kind: "tool",
          tool: "read",
          input: { path: "published.txt", artifact: "source" },
        },
      },
      outputs: {
        reference: {
          schema: { type: "object" },
          value: { $from: "publish", path: "/details/artifact" },
        },
      },
    };
    const r = await invoke(definition);
    expect(r.code).toBe(0);
    const envelope = JSON.parse(r.stdout);
    expect(envelope.result.reference.producer).toEqual({ jobId: "publish", name: "source" });
    expect(envelope.meta.artifacts).toMatchObject([
      { availability: "unavailable", producer: { jobId: "publish", name: "source" } },
    ]);
    expect(envelope.meta.artifacts[0]).not.toHaveProperty("preview");
    expect(envelope.result.reference.preview).toBe("PRIVATE_ARTIFACT_BODY");
  });

  it("reads explicit stdin and input files through the existing JSON boundary", async () => {
    const definition = {
      schemaVersion: 1,
      inputs: { x: { type: "number" } },
      jobs: {},
      outputs: { value: { schema: { type: "number" }, value: { $input: "x" } } },
    };
    expect(
      JSON.parse((await invoke(definition, ["--input", "-"], '{"x":0}')).stdout).result,
    ).toEqual({ value: 0 });
    const file = path.join(isolated.ws, "input.json");
    await writeFile(file, '{"x":7}');
    expect(JSON.parse((await invoke(definition, ["--input", file])).stdout).result).toEqual({
      value: 7,
    });
  });

  it("shares one lazy model runtime across Worker jobs", async () => {
    const createModelRuntime = vi.fn(() => ({}) as never);
    const meta = {
      execution: {
        operations: 0,
        byState: { queued: 0, running: 0, completed: 0, failed: 0, cancelled: 0, timed_out: 0 },
        byTool: {},
        maxConcurrency: 0,
        operationMs: 0,
        activeMs: 0,
        overlapMs: 0,
        queueMs: 0,
      },
      model: { provider: "test", id: "fake", tier: "low", selection: "tier", tierSource: "auto" },
      usage: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, totalTokens: 0, costUsd: 0 },
      turns: 1,
      toolCalls: 0,
      rejectedToolCalls: 0,
      repairs: 0,
      transientRetries: 0,
      changes: { files: [], totals: { files: 0, linesAdded: 0, linesDeleted: 0 } },
    } as const;
    const runWorker = vi.fn(async () => ({
      result: {
        summary: "ok",
        findings: [],
        relevantFiles: [],
        suggestedReads: [],
        openQuestions: [],
      },
      meta,
    }));
    const definition = {
      schemaVersion: 1,
      inputs: {},
      jobs: {
        a: { kind: "worker", worker: "bulk-reader", input: { task: "first" } },
        b: { kind: "worker", worker: "bulk-reader", input: { task: "second" } },
      },
      outputs: { answer: { schema: { type: "string" }, value: { $from: "a", path: "/summary" } } },
    };
    const r = await invoke(definition, [], undefined, { createModelRuntime, runWorker });
    expect(r.code).toBe(0);
    expect(createModelRuntime).toHaveBeenCalledTimes(1);
    expect(runWorker).toHaveBeenCalledTimes(2);
    expect(JSON.parse(r.stdout).result).toEqual({ answer: "ok" });
  });

  it("collects independent success and hides private results in JSON and text failures", async () => {
    await writeFile(path.join(isolated.ws, "private.txt"), "PRIVATE_SENTINEL");
    const definition = {
      schemaVersion: 1,
      inputs: {},
      jobs: {
        good: { kind: "tool", tool: "read", input: { path: "private.txt" } },
        bad: { kind: "tool", tool: "read", input: { path: "missing.txt" } },
      },
      outputs: { surviving: { schema: { type: "boolean" }, value: { $literal: false } } },
    };
    for (const format of ["json", "text"]) {
      const r = await invoke(definition, ["--format", format]);
      expect(r.code).not.toBe(0);
      if (format === "json") expect(JSON.parse(r.stdout).result).toEqual({ surviving: false });
      else expect(r.stdout).toContain('"surviving": false');
      expect(r.stdout + r.stderr).not.toContain("PRIVATE_SENTINEL");
      if (format === "json")
        expect(JSON.parse(r.stdout).jobs).toMatchObject([
          { id: "bad", status: "failed" },
          { id: "good", status: "succeeded" },
        ]);
    }
  });

  it("preserves format and quiet for errors before engine execution", async () => {
    const r = await invoke({ schemaVersion: 2 }, ["--format", "text", "--quiet"]);
    expect(r.code).toBe(2);
    expect(r.stdout).toContain("INVALID_REQUEST");
    expect(r.stderr).toBe("");
  });
});
