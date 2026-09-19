import { mkdirSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { Readable } from "node:stream";
import { CheaperError } from "@cheaper/core";
import type { RunWorkerDeps, WorkerRunMeta } from "@cheaper/runtime-pi";
import { afterEach, describe, expect, it } from "vitest";
import { type CliHooks, type CliIo, runCli } from "../src/cli.js";

function stdinOf(text: string, isTTY = false) {
  const s = Readable.from([text]);
  if (isTTY) (s as Readable & { isTTY: boolean }).isTTY = true;
  return s;
}

const tmpDirs: string[] = [];

afterEach(() => {
  for (const d of tmpDirs.splice(0)) rmSync(d, { recursive: true, force: true });
});

/**
 * 実ユーザーの ~/.config/cheaper と repo の .cheaper を読まないよう、
 * HOME/USERPROFILE/CHEAPER_CONFIG_DIR と cwd を tmp へ隔離する（e2e helpers と同じ方針）。
 */
function makeIo(stdinText = '{"task":"x"}', env: NodeJS.ProcessEnv = {}, isTTY = false) {
  let stdout = "";
  let stderr = "";
  const dir = mkdtempSync(path.join(tmpdir(), "cheaper-cli-test-"));
  tmpDirs.push(dir);
  const ws = path.join(dir, "ws");
  mkdirSync(ws);
  const io: CliIo = {
    stdin: stdinOf(stdinText, isTTY),
    stdout: { write: (s: string) => (stdout += s) },
    stderr: { write: (s: string) => (stderr += s) },
    env: {
      HOME: dir,
      USERPROFILE: dir,
      CHEAPER_CONFIG_DIR: path.join(dir, "cheaper-config"),
      ...env,
    },
    cwd: ws,
  };
  return { io, out: () => stdout, err: () => stderr };
}

const meta: WorkerRunMeta = {
  model: { provider: "p", id: "m", tier: "low", selection: "tier", tierSource: "auto" },
  usage: { input: 1, output: 2, cacheRead: 0, cacheWrite: 0, totalTokens: 3, costUsd: 0 },
  turns: 1,
  toolCalls: 0,
  rejectedToolCalls: 0,
  repairs: 0,
  transientRetries: 0,
  changes: { files: [], totals: { files: 0, linesAdded: 0, linesDeleted: 0 } },
};

function hooks(overrides: Partial<CliHooks> = {}): CliHooks {
  return {
    createModelRuntime: async () => ({}) as never,
    runWorker: async () => ({ result: { ok: true }, meta }),
    ...overrides,
  };
}

describe("runCli", () => {
  it("prints help and exits 0", async () => {
    const { io, out } = makeIo();
    expect(await runCli(["--help"], io, hooks())).toBe(0);
    expect(out()).toContain("Usage: cheaper");
  });

  it("prints version and exits 0", async () => {
    const { io, out } = makeIo();
    expect(await runCli(["--version"], io, hooks())).toBe(0);
    expect(out()).toMatch(/^\d+\.\d+\.\d+\n$/);
  });

  it("emits INVALID_REQUEST envelope for bad args", async () => {
    const { io, out } = makeIo();
    expect(await runCli([], io, hooks())).toBe(2);
    const env = JSON.parse(out());
    expect(env.ok).toBe(false);
    expect(env.error.code).toBe("INVALID_REQUEST");
  });

  it("emits INVALID_WORKER_NAME (exit 3) for path-like names", async () => {
    const { io, out } = makeIo();
    expect(await runCli(["run", "./x.md"], io, hooks())).toBe(3);
    expect(JSON.parse(out()).error.code).toBe("INVALID_WORKER_NAME");
  });

  it("emits WORKER_NOT_FOUND (exit 3) for unknown worker", async () => {
    const { io, out } = makeIo();
    expect(await runCli(["run", "no-such-worker-xyz"], io, hooks())).toBe(3);
    const env = JSON.parse(out());
    expect(env.error.code).toBe("WORKER_NOT_FOUND");
    expect(env.worker).toBeNull();
  });

  it("emits INVALID_REQUEST when stdin is a TTY without --input", async () => {
    const { io, out } = makeIo("", {}, true);
    expect(await runCli(["bulk-read"], io, hooks())).toBe(2);
    expect(JSON.parse(out()).error.code).toBe("INVALID_REQUEST");
  });

  it("emits INVALID_INPUT when task input violates the worker schema", async () => {
    const { io, out } = makeIo('{"nope":1}');
    expect(await runCli(["bulk-read"], io, hooks())).toBe(2);
    const env = JSON.parse(out());
    expect(env.error.code).toBe("INVALID_INPUT");
    expect(env.worker).toBe("bulk-reader");
  });

  it("emits INVALID_REQUEST for an unparseable --timeout", async () => {
    const { io, out } = makeIo();
    expect(await runCli(["bulk-read", "--timeout", "soon"], io, hooks())).toBe(2);
    expect(JSON.parse(out()).error.code).toBe("INVALID_REQUEST");
  });

  it("emits POLICY_VIOLATION (exit 4) when --timeout expands the policy", async () => {
    const { io, out } = makeIo();
    expect(await runCli(["bulk-read", "--timeout", "300s"], io, hooks())).toBe(4);
    const env = JSON.parse(out());
    expect(env.error.code).toBe("POLICY_VIOLATION");
    expect(env.worker).toBe("bulk-reader");
  });

  it("emits a success envelope and the model line on the happy path", async () => {
    const { io, out, err } = makeIo();
    let seen: RunWorkerDeps | undefined;
    const code = await runCli(
      ["bulk-read"],
      io,
      hooks({
        runWorker: async (deps) => {
          seen = deps;
          return { result: { ok: true }, meta };
        },
      }),
    );
    expect(code).toBe(0);
    const env = JSON.parse(out());
    expect(env).toMatchObject({ schemaVersion: 1, ok: true, worker: "bulk-reader" });
    expect(env.result).toEqual({ ok: true });
    expect(env.meta.model).toEqual({
      provider: "p",
      id: "m",
      tier: "low",
      selection: "tier",
      tierSource: "auto",
    });
    expect(err()).toBe("model: p/m (tier low)\n");
    expect(seen?.worker.name).toBe("bulk-reader");
    expect(seen?.policy.execution.timeoutMs).toBe(120_000);
  });

  it("narrows the policy timeout when --timeout is within bounds", async () => {
    const { io } = makeIo();
    let seen: RunWorkerDeps | undefined;
    await runCli(
      ["bulk-read", "--timeout", "30s"],
      io,
      hooks({
        runWorker: async (deps) => {
          seen = deps;
          return { result: {}, meta };
        },
      }),
    );
    expect(seen?.policy.execution.timeoutMs).toBe(30_000);
  });

  it("--quiet suppresses the stderr model line", async () => {
    const { io, err } = makeIo();
    expect(await runCli(["bulk-read", "--quiet"], io, hooks())).toBe(0);
    expect(err()).toBe("");
  });

  it("emits an error envelope with worker name when runWorker throws CheaperError", async () => {
    const { io, out } = makeIo();
    const code = await runCli(
      ["bulk-read"],
      io,
      hooks({
        runWorker: async () => {
          throw new CheaperError("TIMEOUT", "took too long", {});
        },
      }),
    );
    expect(code).toBe(7);
    const env = JSON.parse(out());
    expect(env.ok).toBe(false);
    expect(env.worker).toBe("bulk-reader");
    expect(env.error).toMatchObject({ code: "TIMEOUT", exitCode: 7, retryable: true });
  });

  it("maps unknown throws to INTERNAL_ERROR (exit 70)", async () => {
    const { io, out } = makeIo();
    const code = await runCli(
      ["bulk-read"],
      io,
      hooks({
        runWorker: async () => {
          throw new Error("boom");
        },
      }),
    );
    expect(code).toBe(70);
    expect(JSON.parse(out()).error.code).toBe("INTERNAL_ERROR");
  });

  it("propagates an external abort signal into runWorker deps", async () => {
    const { io } = makeIo();
    const ac = new AbortController();
    let abortedDuringRun = false;
    let sawSignal = false;
    await runCli(
      ["bulk-read"],
      io,
      hooks({
        signal: ac.signal,
        runWorker: async (deps) => {
          sawSignal = deps.signal !== undefined;
          ac.abort();
          abortedDuringRun = deps.signal?.aborted === true;
          return { result: {}, meta };
        },
      }),
    );
    expect(sawSignal).toBe(true);
    expect(abortedDuringRun).toBe(true);
  });

  it("prints text format when --format text", async () => {
    const { io, out } = makeIo();
    expect(await runCli(["bulk-read", "--format", "text"], io, hooks())).toBe(0);
    expect(out().split("\n")[0]).toBe("ok");
    expect(out()).toContain("worker:");
  });

  it("ignores CHEAPER_TEST_FAKE_STREAM when NODE_ENV is not test", async () => {
    const { io } = makeIo('{"task":"x"}', {
      NODE_ENV: "production",
      CHEAPER_TEST_FAKE_STREAM: "/nonexistent/fake.mjs",
    });
    let seen: RunWorkerDeps | undefined;
    const code = await runCli(
      ["bulk-read"],
      io,
      hooks({
        runWorker: async (deps) => {
          seen = deps;
          return { result: {}, meta };
        },
      }),
    );
    expect(code).toBe(0);
    expect(seen?.streamFn).toBeUndefined();
  });
});
