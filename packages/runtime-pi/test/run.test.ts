import { mkdtemp, readFile, realpath, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import {
  type CheaperError,
  compileSchema,
  countTextTokens,
  type RuntimePolicy,
  type WorkerDefinition,
} from "@cheaper/core";
import * as toolRuntime from "@cheaper/tool-runtime";
import {
  type AssistantMessage,
  type Context,
  createAssistantMessageEventStream,
  type Model,
  type SimpleStreamOptions,
} from "@earendil-works/pi-ai";
import { ModelRuntime } from "@earendil-works/pi-coding-agent";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ModelCatalog } from "../src/model-runtime.js";
import { runWorker, WorkerRunError } from "../src/run.js";
import { createFakePi, fauxAssistantMessage, fauxToolCall } from "./fixtures/fake-stream.js";

let dir: string;
let root: string;
let runtime: ModelRuntime;

beforeEach(async () => {
  dir = await mkdtemp(path.join(tmpdir(), "cheaper-run-"));
  root = await realpath(dir);
  runtime = await ModelRuntime.create({ refreshOnCreate: false });
});
afterEach(async () => {
  await rm(dir, { recursive: true, force: true });
});

const outputSchema = compileSchema(
  { type: "object", required: ["ok"], properties: { ok: { type: "boolean" } } },
  "output",
);

function worker(
  tools: WorkerDefinition["tools"],
  opts?: { writeScope?: string },
): WorkerDefinition {
  return {
    name: "test-worker",
    description: "test",
    tools,
    cheaper: {
      model: { tier: "low" },
      output: { schema: { inline: {} } },
      ...(opts?.writeScope !== undefined ? { writeScope: opts.writeScope } : {}),
    },
    instructions: "Do the task.",
    sourcePath: "test-worker.md",
    origin: "explicit",
    outputSchema,
  };
}

function policy(overrides: Partial<RuntimePolicy> = {}): RuntimePolicy {
  return {
    execution: { timeoutMs: 10_000, maxTurns: 10, maxToolCalls: 20 },
    tools: { allow: ["read", "grep", "find", "ls", "write", "edit"] },
    filesystem: { symlinks: "deny", denyPaths: [] },
    model: { maxTier: "high", tierFallback: "none" },
    output: { maxTokens: 1024, repairAttempts: 1 },
    retry: { transientAttempts: 0 },
    ...overrides,
  };
}

async function deps(fake: ReturnType<typeof createFakePi>, w: WorkerDefinition, p: RuntimePolicy) {
  runtime.registerNativeProvider(fake.provider);
  await runtime.refresh({ allowNetwork: false });
  return {
    worker: w,
    input: { task: "x" },
    root,
    policy: p,
    modelConfig: { overrides: { "cheaper-fake/fake-model": "low" }, preferences: {} },
    catalog: runtime,
    modelRuntime: runtime,
    streamFn: fake.streamFn,
  };
}

describe("runWorker", { timeout: 20000 }, () => {
  it.each([false, true])(
    "reports deterministic owned execution and preserves usage/savings: failure=%s",
    async (failure) => {
      await writeFile(path.join(root, "a.txt"), "alpha beta gamma\n".repeat(20));
      const fake = createFakePi({
        responses: [
          fauxAssistantMessage(fauxToolCall("read", { path: "a.txt" }), { stopReason: "toolUse" }),
          failure
            ? fauxAssistantMessage(fauxToolCall("read", { path: "a.txt" }, "second"), {
                stopReason: "toolUse",
              })
            : fauxAssistantMessage('{"ok":true}'),
        ],
      });
      const p = policy();
      if (failure) p.execution.maxToolCalls = 1;
      const ticks = [10, 20, 45, 50];
      const result = await runWorker({
        ...(await deps(fake, worker(["read"]), p)),
        operationClock: () => ticks.shift() ?? 50,
      }).then(
        (value) => value,
        (error: WorkerRunError) => error,
      );
      const meta = result instanceof WorkerRunError ? result.runMeta : result.meta;
      expect(meta?.execution).toEqual({
        operations: 1,
        byState: { queued: 0, running: 0, completed: 1, failed: 0, cancelled: 0, timed_out: 0 },
        byTool: { read: 1 },
        maxConcurrency: 1,
        operationMs: 25,
        activeMs: 25,
        overlapMs: 0,
        queueMs: 10,
      });
      expect(meta?.toolCalls).toBe(1);
      expect(meta?.rejectedToolCalls).toBe(failure ? 1 : 0);
      expect(meta?.turns).toBeGreaterThan(0);
      expect(meta?.usage?.totalTokens).toBeGreaterThan(0);
      expect(meta?.savings?.sourceTokens).toBeGreaterThan(0);
      expect(meta?.savings?.returnedTokens).toBe(failure ? 0 : countTextTokens('{"ok":true}'));
      if (failure) expect(result).toMatchObject({ code: "MAX_TOOL_CALLS_EXCEEDED" });
    },
  );

  it("includes a zero snapshot when model resolution fails after invocation creation", async () => {
    const fake = createFakePi({ responses: [fauxAssistantMessage('{"ok":true}')] });
    const error = await runWorker({
      ...(await deps(fake, worker(["read"]), policy())),
      explicitModel: "invalid-ref",
      operationClock: () => 0,
    }).then(
      () => null,
      (value: WorkerRunError) => value,
    );
    expect(error).toBeInstanceOf(WorkerRunError);
    expect(error?.code).toBe("INVALID_REQUEST");
    expect(error?.runMeta?.execution).toEqual({
      operations: 0,
      byState: { queued: 0, running: 0, completed: 0, failed: 0, cancelled: 0, timed_out: 0 },
      byTool: {},
      maxConcurrency: 0,
      operationMs: 0,
      activeMs: 0,
      overlapMs: 0,
      queueMs: 0,
    });
    expect(error?.runMeta?.model).toBeUndefined();
    expect(error?.runMeta?.usage).toBeUndefined();
  });

  it("reports only its borrowed owner while a sibling is active", async () => {
    let now = 0;
    let release!: () => void;
    let started!: () => void;
    const waiting = new Promise<void>((resolve) => {
      release = resolve;
    });
    const ready = new Promise<void>((resolve) => {
      started = resolve;
    });
    const invocation = toolRuntime.createInvocationToolRuntime({
      root,
      symlinkPolicy: "deny",
      clock: () => now,
    });
    const sibling = invocation.createWorkerScope({
      root,
      symlinkPolicy: "deny",
      ownerId: "sibling",
      names: ["ls"],
      maxToolCalls: 10,
      onLimit: () => {},
      handlers: {
        ls: async () => {
          started();
          await waiting;
          return { content: [] };
        },
      },
    });
    const pending = sibling.execute("ls", { path: "." });
    await ready;
    now = 10;
    await writeFile(path.join(root, "a.txt"), "hello");
    const fake = createFakePi({
      responses: [
        fauxAssistantMessage(fauxToolCall("read", { path: "a.txt" }), { stopReason: "toolUse" }),
        fauxAssistantMessage('{"ok":true}'),
      ],
    });
    try {
      const value = await runWorker({
        ...(await deps(fake, worker(["read"]), policy())),
        execution: { runtime: invocation, ownerId: "worker" },
      });
      expect(value.meta.execution).toMatchObject({
        operations: 1,
        byTool: { read: 1 },
        maxConcurrency: 1,
        operationMs: 0,
        activeMs: 0,
        queueMs: 0,
      });
      expect(invocation.executionMetrics()).toMatchObject({
        operations: 2,
        maxConcurrency: 2,
        byState: { running: 1, completed: 1 },
      });
    } finally {
      now = 20;
      release();
      await pending;
      await invocation.dispose();
    }
  });

  it("propagates named process policy through the owned Pi invocation", async () => {
    await writeFile(
      path.join(root, "child.mjs"),
      'import {writeFileSync} from "node:fs"; writeFileSync("process.txt", "ran");',
    );
    const fake = createFakePi({
      responses: [
        fauxAssistantMessage(fauxToolCall("process", { task: "check" }), { stopReason: "toolUse" }),
        fauxAssistantMessage('{"ok":true}'),
      ],
    });
    const p = policy({
      tools: { allow: ["process"] },
      process: {
        maxConcurrent: 1,
        tasks: {
          check: {
            command: ["node", "child.mjs"],
            inputs: ["child.mjs"],
            outputs: ["process.txt"],
          },
        },
      },
    });
    const result = await runWorker(await deps(fake, worker(["process"]), p));
    expect(result.result).toEqual({ ok: true });
    expect(result.meta.toolCalls).toBe(1);
    expect(await readFile(path.join(root, "process.txt"), "utf8")).toBe("ran");
  });
  it.each([
    ["timeout", "TIMEOUT"],
    ["signal", "ABORTED"],
    ["maxTurns", "MAX_TURNS_EXCEEDED"],
    ["maxToolCalls", "MAX_TOOL_CALLS_EXCEEDED"],
  ] as const)(
    "passes the first %s reason into its actual borrowed child scope",
    async (trigger, code) => {
      await writeFile(path.join(root, "a.txt"), "hello\n");
      const invocation = toolRuntime.createInvocationToolRuntime({ root, symlinkPolicy: "deny" });
      const opts = {
        root,
        symlinkPolicy: "deny" as const,
        names: ["read"] as const,
        maxToolCalls: 10,
        onLimit: () => {},
      };
      const sibling = invocation.createWorkerScope({ ...opts, ownerId: "sibling" });
      const createScope = invocation.createWorkerScope.bind(invocation);
      let childSignal: AbortSignal | undefined;
      const scopeSpy = vi.spyOn(invocation, "createWorkerScope").mockImplementation((options) => {
        childSignal = options.signal;
        return createScope(options);
      });
      const external = new AbortController();
      const p = policy();
      const fake =
        trigger === "timeout" || trigger === "signal"
          ? createFakePi({
              responses: [
                (_ctx, options) =>
                  new Promise<AssistantMessage>((resolve) => {
                    options?.signal?.addEventListener(
                      "abort",
                      () => resolve(fauxAssistantMessage("", { stopReason: "aborted" })),
                      { once: true },
                    );
                    if (trigger === "signal") external.abort();
                  }),
              ],
            })
          : createFakePi({
              responses: [
                fauxAssistantMessage(fauxToolCall("read", { path: "a.txt" }), {
                  stopReason: "toolUse",
                }),
                fauxAssistantMessage(fauxToolCall("read", { path: "a.txt" }), {
                  stopReason: "toolUse",
                }),
                fauxAssistantMessage('{"ok":true}'),
              ],
            });
      if (trigger === "timeout") p.execution.timeoutMs = 30;
      if (trigger === "maxTurns") p.execution.maxTurns = 1;
      if (trigger === "maxToolCalls") p.execution.maxToolCalls = 1;
      try {
        const error = await runWorker({
          ...(await deps(fake, worker(["read"]), p)),
          execution: { runtime: invocation, ownerId: "worker" },
          signal: external.signal,
          abortGraceMs: 20,
        }).catch((error) => error as CheaperError);
        expect((error as CheaperError).code).toBe(code);
        expect(childSignal?.aborted).toBe(true);
        expect(childSignal?.reason).toMatchObject({ code, details: { reason: trigger } });
        expect(invocation.operations("worker").every((op) => !op.owned)).toBe(true);
        expect((await sibling.execute("read", { path: "a.txt" })).content[0]?.text).toBe("hello\n");
      } finally {
        scopeSpy.mockRestore();
        await invocation.dispose();
      }
    },
  );

  it("owns one invocation and disposes it in finally", async () => {
    const create = toolRuntime.createInvocationToolRuntime;
    let dispose: ReturnType<typeof vi.spyOn> | undefined;
    const factory = vi
      .spyOn(toolRuntime, "createInvocationToolRuntime")
      .mockImplementation((options) => {
        const invocation = create(options);
        dispose = vi.spyOn(invocation, "dispose");
        return invocation;
      });
    try {
      const fake = createFakePi({ responses: [fauxAssistantMessage('{"ok":true}')] });
      await runWorker({ ...(await deps(fake, worker(["read"]), policy())), abortGraceMs: 15 });
      expect(factory).toHaveBeenCalledTimes(1);
      expect(factory.mock.calls[0]?.[0].abortGraceMs).toBe(15);
      expect(dispose).toHaveBeenCalledTimes(1);
    } finally {
      factory.mockRestore();
    }
  });

  it("ends borrowed child scope while retaining shared root and sibling", async () => {
    await writeFile(path.join(root, "a.txt"), "hello\n");
    const invocation = toolRuntime.createInvocationToolRuntime({ root, symlinkPolicy: "deny" });
    const opts = {
      root,
      symlinkPolicy: "deny" as const,
      names: ["read"] as const,
      maxToolCalls: 10,
      onLimit: () => {},
    };
    const sibling = invocation.createWorkerScope({ ...opts, ownerId: "sibling" });
    const create = invocation.createWorkerScope.bind(invocation);
    let child: toolRuntime.ToolRuntime | undefined;
    const scopeSpy = vi.spyOn(invocation, "createWorkerScope").mockImplementation((options) => {
      child = create(options);
      return child;
    });
    const dispose = vi.spyOn(invocation, "dispose");
    const fake = createFakePi({ responses: [fauxAssistantMessage('{"ok":true}')] });
    try {
      await runWorker({
        ...(await deps(fake, worker(["read"]), policy())),
        execution: { runtime: invocation, ownerId: "worker" },
      });
      expect(scopeSpy).toHaveBeenCalledTimes(1);
      expect(dispose).not.toHaveBeenCalled();
      if (!child) throw new Error("child scope was not created");
      await expect(child.execute("read", { path: "a.txt" })).rejects.toThrow("aborted");
      expect((await sibling.execute("read", { path: "a.txt" })).content[0]?.text).toBe("hello\n");
    } finally {
      await invocation.dispose();
    }
  });

  it("fails closed on a borrowed root mismatch before session execution", async () => {
    const invocation = toolRuntime.createInvocationToolRuntime({
      root: `${root}-other`,
      symlinkPolicy: "deny",
    });
    const fake = createFakePi({ responses: [fauxAssistantMessage('{"ok":true}')] });
    try {
      await expect(
        runWorker({
          ...(await deps(fake, worker(["read"]), policy())),
          execution: { runtime: invocation, ownerId: "worker" },
        }),
      ).rejects.toThrow("root mismatch");
    } finally {
      await invocation.dispose();
    }
  });
  it("returns result and meta on the happy path", async () => {
    const fake = createFakePi({ responses: [fauxAssistantMessage('{"ok":true}')] });
    const r = await runWorker(await deps(fake, worker(["read"]), policy()));
    expect(r.result).toEqual({ ok: true });
    expect(r.meta.model).toEqual({
      provider: "cheaper-fake",
      id: "fake-model",
      tier: "low",
      selection: "tier",
      tierSource: "explicit-override",
    });
    expect(r.meta.turns).toBe(1);
    expect(r.meta.toolCalls).toBe(0);
    expect(r.meta.rejectedToolCalls).toBe(0);
    expect(r.meta.repairs).toBe(0);
    expect(r.meta.usage.output).toBeGreaterThanOrEqual(0);
    expect(r.meta.changes.totals.files).toBe(0);
  });

  it("auto 分類は meta.model に effectiveCost/percentile/workload を記録する", async () => {
    const fake = createFakePi({
      models: [
        {
          id: "fake-model",
          name: "Fake Model",
          cost: { input: 0.5, output: 1, cacheRead: 0, cacheWrite: 0 },
        },
        {
          id: "fake-mid",
          name: "Fake Mid",
          cost: { input: 3, output: 15, cacheRead: 0, cacheWrite: 0 },
        },
        {
          id: "fake-high",
          name: "Fake High",
          cost: { input: 15, output: 75, cacheRead: 0, cacheWrite: 0 },
        },
      ],
      responses: [fauxAssistantMessage('{"ok":true}')],
    });
    const w = worker(["read"]);
    w.cheaper.model.workload = { outputPerInput: 0.3 };
    const base = await deps(fake, w, policy());
    // 実 catalog には環境依存の model が混ざるため、ranking 母集団を
    // fake model に限定して auto 分類を決定的に検証する。
    const fakeOnly = {
      getAvailable: async () =>
        (await base.catalog.getAvailable()).filter((m) => m.provider === "cheaper-fake"),
      getModel: (p: string, id: string) => base.catalog.getModel(p, id),
    };
    const r = await runWorker({
      ...base,
      catalog: fakeOnly,
      modelConfig: { overrides: {}, preferences: {} },
    });
    expect(r.result).toEqual({ ok: true });
    expect(r.meta.model).toEqual({
      provider: "cheaper-fake",
      id: "fake-model",
      tier: "low",
      selection: "tier",
      tierSource: "auto",
      effectiveCost: 0.8,
      effectiveCostPercentile: 1 / 6,
      workload: { outputPerInput: 0.3 },
    });
  });

  it("--model 直接指定は selection=explicit で同じ model を解決する", async () => {
    const fake = createFakePi({ responses: [fauxAssistantMessage('{"ok":true}')] });
    const base = await deps(fake, worker(["read"]), policy());
    const r = await runWorker({ ...base, explicitModel: "cheaper-fake/fake-model" });
    expect(r.result).toEqual({ ok: true });
    expect(r.meta.model).toEqual({
      provider: "cheaper-fake",
      id: "fake-model",
      tier: "low",
      selection: "explicit",
      tierSource: "explicit-override",
    });
  });

  it("--model 指定が exclude に一致すると POLICY_VIOLATION", async () => {
    const fake = createFakePi({ responses: [fauxAssistantMessage('{"ok":true}')] });
    const base = await deps(fake, worker(["read"]), policy());
    const err = await runWorker({
      ...base,
      explicitModel: "cheaper-fake/fake-model",
      modelConfig: { overrides: {}, preferences: {}, exclude: ["cheaper-fake/*"] },
    }).then(
      () => undefined,
      (e: unknown) => e,
    );
    expect((err as CheaperError).code).toBe("POLICY_VIOLATION");
  });

  it("--model 指定が catalog に無いと MODEL_TIER_UNAVAILABLE", async () => {
    const fake = createFakePi({ responses: [fauxAssistantMessage('{"ok":true}')] });
    const base = await deps(fake, worker(["read"]), policy());
    const err = await runWorker({ ...base, explicitModel: "cheaper-fake/no-such" }).then(
      () => undefined,
      (e: unknown) => e,
    );
    expect((err as CheaperError).code).toBe("MODEL_TIER_UNAVAILABLE");
  });

  it("--model の形式不正 → INVALID_REQUEST", async () => {
    const fake = createFakePi({ responses: [fauxAssistantMessage('{"ok":true}')] });
    const base = await deps(fake, worker(["read"]), policy());
    const err = await runWorker({ ...base, explicitModel: "no-slash" }).then(
      () => undefined,
      (e: unknown) => e,
    );
    expect((err as CheaperError).code).toBe("INVALID_REQUEST");
  });

  it("executes sandboxed tools and records changes", async () => {
    const fake = createFakePi({
      responses: [
        fauxAssistantMessage(fauxToolCall("write", { path: "o.txt", content: "hi\n" }), {
          stopReason: "toolUse",
        }),
        fauxAssistantMessage('{"ok":true}'),
      ],
    });
    const r = await runWorker(await deps(fake, worker(["write"]), policy()));
    expect(r.result).toEqual({ ok: true });
    expect(await readFile(path.join(root, "o.txt"), "utf8")).toBe("hi\n");
    expect(r.meta.toolCalls).toBe(1);
    expect(r.meta.turns).toBe(2);
    expect(r.meta.changes.files).toEqual([
      { path: "o.txt", kind: "created", linesAdded: 1, linesDeleted: 0 },
    ]);
  });

  it("classifies maxToolCalls as MAX_TOOL_CALLS_EXCEEDED when no mutation", async () => {
    const fake = createFakePi({
      responses: [
        fauxAssistantMessage(fauxToolCall("read", { path: "a" }), { stopReason: "toolUse" }),
        fauxAssistantMessage(fauxToolCall("read", { path: "b" }), { stopReason: "toolUse" }),
        fauxAssistantMessage("unreached"),
      ],
    });
    const p = policy();
    p.execution.maxToolCalls = 1;
    const err = await runWorker(await deps(fake, worker(["read"]), p)).then(
      () => undefined,
      (e: unknown) => e,
    );
    expect((err as CheaperError).code).toBe("MAX_TOOL_CALLS_EXCEEDED");
  });

  it("wraps abort-after-mutation as PARTIAL_MUTATION with cause", async () => {
    const fake = createFakePi({
      responses: [
        fauxAssistantMessage(fauxToolCall("write", { path: "m.txt", content: "x" }), {
          stopReason: "toolUse",
        }),
        fauxAssistantMessage(fauxToolCall("write", { path: "m2.txt", content: "y" }), {
          stopReason: "toolUse",
        }),
      ],
    });
    const p = policy();
    p.execution.maxToolCalls = 1;
    const err = await runWorker(await deps(fake, worker(["write"]), p)).then(
      () => undefined,
      (e: unknown) => e,
    );
    const ce = err as CheaperError;
    expect(ce.code).toBe("PARTIAL_MUTATION");
    expect((ce.cause as CheaperError).code).toBe("MAX_TOOL_CALLS_EXCEEDED");
    const changes = (ce.details as { changes: { files: Array<{ path: string }> } }).changes;
    expect(changes.files.map((f) => f.path)).toContain("m.txt");
  });

  it("classifies maxTurns as MAX_TURNS_EXCEEDED", async () => {
    const fake = createFakePi({
      responses: [
        fauxAssistantMessage(fauxToolCall("read", { path: "a" }), { stopReason: "toolUse" }),
        fauxAssistantMessage('{"ok":true}'),
      ],
    });
    const p = policy();
    p.execution.maxTurns = 1;
    const err = await runWorker(await deps(fake, worker(["read"]), p)).then(
      () => undefined,
      (e: unknown) => e,
    );
    expect((err as CheaperError).code).toBe("MAX_TURNS_EXCEEDED");
  });

  it("classifies timeout as TIMEOUT", async () => {
    // signal abort まで応答しない stream
    const hanging = (_m: Model<string>, _c: Context, o?: SimpleStreamOptions) => {
      const s = createAssistantMessageEventStream();
      o?.signal?.addEventListener(
        "abort",
        () => {
          const msg: AssistantMessage = {
            role: "assistant",
            content: [],
            api: "cheaper-fake-api",
            provider: "cheaper-fake",
            model: "fake-model",
            usage: {
              input: 0,
              output: 0,
              cacheRead: 0,
              cacheWrite: 0,
              totalTokens: 0,
              cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: 0 },
            },
            stopReason: "aborted",
            timestamp: Date.now(),
          };
          s.push({ type: "error", reason: "aborted", error: msg });
          s.end(msg);
        },
        { once: true },
      );
      return s;
    };
    const fake = createFakePi();
    const p = policy();
    p.execution.timeoutMs = 30;
    const err = await runWorker({
      ...(await deps(fake, worker(["read"]), p)),
      streamFn: hanging,
    }).then(
      () => undefined,
      (e: unknown) => e,
    );
    expect((err as CheaperError).code).toBe("TIMEOUT");
  });

  it("classifies timeout during model resolution as TIMEOUT", async () => {
    // timeout は runWorker 開始時点から計測されるため、model 解決が
    // timeout を跨いでも TIMEOUT に分類される（DESIGN-0004 / ADR-0010）
    const fake = createFakePi({ responses: [fauxAssistantMessage('{"ok":true}')] });
    const p = policy();
    p.execution.timeoutMs = 30;
    const d = await deps(fake, worker(["read"]), p);
    const slowCatalog: ModelCatalog = {
      getAvailable: async (providerId) => {
        await new Promise((r) => setTimeout(r, 100));
        return runtime.getAvailable(providerId);
      },
      getModel: (providerId, modelId) => runtime.getModel(providerId, modelId),
    };
    const err = await runWorker({ ...d, catalog: slowCatalog }).then(
      () => undefined,
      (e: unknown) => e,
    );
    expect((err as CheaperError).code).toBe("TIMEOUT");
  });

  it("does not start a stream after the Worker timeout fired during setup", async () => {
    const fake = createFakePi();
    const p = policy();
    p.execution.timeoutMs = 20;
    const d = await deps(fake, worker(["read"]), p);
    const stream = vi.fn(() => createAssistantMessageEventStream());
    const slowCatalog: ModelCatalog = {
      getAvailable: async (providerId) => {
        await new Promise((resolve) => setTimeout(resolve, 80));
        return runtime.getAvailable(providerId);
      },
      getModel: (providerId, modelId) => runtime.getModel(providerId, modelId),
    };
    const err = await runWorker({
      ...d,
      catalog: slowCatalog,
      streamFn: stream,
      abortGraceMs: 30,
    }).catch((e) => e as CheaperError);
    expect(err.code).toBe("TIMEOUT");
    expect(stream).not.toHaveBeenCalled();
  });

  it("classifies external signal abort as ABORTED", async () => {
    const fake = createFakePi({
      // 遅いが abort には応答する応答
      responses: [
        (_ctx, opts) =>
          new Promise<AssistantMessage>((resolve) => {
            const timer = setTimeout(() => resolve(fauxAssistantMessage('{"ok":true}')), 5000);
            opts?.signal?.addEventListener(
              "abort",
              () => {
                clearTimeout(timer);
                resolve(
                  fauxAssistantMessage("", {
                    stopReason: "aborted",
                    errorMessage: "Request was aborted",
                  }),
                );
              },
              { once: true },
            );
          }),
      ],
    });
    const ac = new AbortController();
    const p = policy();
    const promise = runWorker({ ...(await deps(fake, worker(["read"]), p)), signal: ac.signal });
    setTimeout(() => ac.abort(), 50);
    const err = await promise.then(
      () => undefined,
      (e: unknown) => e,
    );
    expect((err as CheaperError).code).toBe("ABORTED");
  });

  it("repairs invalid output and counts repairs", async () => {
    const fake = createFakePi({
      responses: [
        fauxAssistantMessage("not json"),
        fauxAssistantMessage('{"ok":false}'), // repair session の応答
      ],
    });
    const r = await runWorker(await deps(fake, worker(["read"]), policy()));
    expect(r.result).toEqual({ ok: false });
    expect(r.meta.repairs).toBe(1);
  });

  it("aggregates repair session usage into meta", async () => {
    const fake = createFakePi({
      responses: [fauxAssistantMessage("not json"), fauxAssistantMessage('{"ok":false}')],
    });
    const r = await runWorker(await deps(fake, worker(["read"]), policy()));
    expect(r.meta.repairs).toBe(1);
    // repair session の turn_end / message_end も同じ collector に合算される
    expect(r.meta.turns).toBe(2);
    const control = createFakePi({ responses: [fauxAssistantMessage('{"ok":false}')] });
    const rc = await runWorker(await deps(control, worker(["read"]), policy()));
    expect(r.meta.usage.output).toBeGreaterThan(rc.meta.usage.output);
  });

  it("counts repair session auto_retry_start as meta.transientRetries", async () => {
    const fake = createFakePi({
      responses: [
        fauxAssistantMessage("not json"), // main → invalid output
        fauxAssistantMessage("", { stopReason: "error", errorMessage: "503" }), // repair → retry
        fauxAssistantMessage('{"ok":true}'), // repair retry で成功
      ],
    });
    const p = policy();
    p.retry.transientAttempts = 2;
    const r = await runWorker(await deps(fake, worker(["read"]), p));
    expect(r.result).toEqual({ ok: true });
    expect(r.meta.repairs).toBe(1);
    expect(r.meta.transientRetries).toBe(1);
    expect(fake.state.callCount).toBe(3);
  }, 15_000);

  it("classifies timeout during repair session as TIMEOUT", async () => {
    const fake = createFakePi({
      responses: [
        fauxAssistantMessage("not json"), // main → invalid output → repair へ
        // repair session: abort signal を受けて settle する遅延応答
        (_ctx, opts) =>
          new Promise<AssistantMessage>((resolve) => {
            const timer = setTimeout(() => resolve(fauxAssistantMessage('{"ok":true}')), 10_000);
            opts?.signal?.addEventListener(
              "abort",
              () => {
                clearTimeout(timer);
                resolve(
                  fauxAssistantMessage("", {
                    stopReason: "aborted",
                    errorMessage: "Request was aborted",
                  }),
                );
              },
              { once: true },
            );
          }),
      ],
    });
    const p = policy();
    p.execution.timeoutMs = 50;
    const err = await runWorker(await deps(fake, worker(["read"]), p)).then(
      () => undefined,
      (e: unknown) => e,
    );
    expect((err as CheaperError).code).toBe("TIMEOUT");
    // error にはその時点の meta snapshot が載る（envelope error meta 用）
    expect(err).toBeInstanceOf(WorkerRunError);
    expect((err as WorkerRunError).runMeta?.turns).toBeGreaterThanOrEqual(1);
    expect((err as WorkerRunError).runMeta?.usage).toBeDefined();
  });

  it("throws INVALID_OUTPUT when repair attempts are exhausted", async () => {
    const fake = createFakePi({
      responses: [fauxAssistantMessage("bad"), fauxAssistantMessage("still bad")],
    });
    const err = await runWorker(await deps(fake, worker(["read"]), policy())).then(
      () => undefined,
      (e: unknown) => e,
    );
    expect((err as CheaperError).code).toBe("INVALID_OUTPUT");
  });

  it("reports consumed repair attempts in meta.repairs on failure", async () => {
    const fake = createFakePi({
      responses: [
        fauxAssistantMessage("bad"),
        fauxAssistantMessage("bad"),
        fauxAssistantMessage("bad"),
      ],
    });
    const p = policy();
    p.output.repairAttempts = 2;
    const err = await runWorker(await deps(fake, worker(["read"]), p)).then(
      () => undefined,
      (e: unknown) => e,
    );
    expect((err as CheaperError).code).toBe("INVALID_OUTPUT");
    expect((err as WorkerRunError).runMeta?.repairs).toBe(2);
  });

  it("maps a repair provider error end state to PROVIDER_FAILURE", async () => {
    const fake = createFakePi({
      responses: [
        fauxAssistantMessage("not json"), // main → invalid output → repair へ
        fauxAssistantMessage("", { stopReason: "error", errorMessage: "provider down" }),
      ],
    });
    const err = await runWorker(await deps(fake, worker(["read"]), policy())).then(
      () => undefined,
      (e: unknown) => e,
    );
    expect((err as CheaperError).code).toBe("PROVIDER_FAILURE");
  });

  it("cuts off a repair session that ignores abort after the grace", async () => {
    const fake = createFakePi({
      responses: [
        fauxAssistantMessage("not json"), // main → invalid output → repair へ
        // repair session: abort signal を無視して settle しない応答
        () => new Promise<AssistantMessage>(() => {}),
      ],
    });
    const p = policy();
    p.execution.timeoutMs = 30;
    const startedAt = Date.now();
    const err = await runWorker({
      ...(await deps(fake, worker(["read"]), p)),
      abortGraceMs: 30,
    }).then(
      () => undefined,
      (e: unknown) => e,
    );
    expect((err as CheaperError).code).toBe("TIMEOUT");
    expect(Date.now() - startedAt).toBeLessThan(2000);
  });

  it("maps provider error end state to PROVIDER_FAILURE", async () => {
    const fake = createFakePi({
      responses: [fauxAssistantMessage("", { stopReason: "error", errorMessage: "provider down" })],
    });
    const p = policy();
    const err = await runWorker(await deps(fake, worker(["read"]), p)).then(
      () => undefined,
      (e: unknown) => e,
    );
    expect((err as CheaperError).code).toBe("PROVIDER_FAILURE");
  });

  it("counts Pi auto_retry_start events as meta.transientRetries", async () => {
    const fake = createFakePi({
      responses: [
        fauxAssistantMessage("", {
          stopReason: "error",
          errorMessage: "503 service unavailable",
        }),
        fauxAssistantMessage('{"ok":true}'),
      ],
    });
    const p = policy();
    p.retry.transientAttempts = 2;
    const r = await runWorker(await deps(fake, worker(["read"]), p));
    expect(r.result).toEqual({ ok: true });
    expect(r.meta.transientRetries).toBe(1);
    expect(fake.state.callCount).toBe(2); // cheaper 独自 retry は無く Pi auto-retry のみ
  }, 15_000);

  it("measures savings from tool output tokens and returned result", async () => {
    // read output >> returned result になるよう十分な量の入力を読ませる
    await writeFile(path.join(root, "big.txt"), "alpha beta gamma delta\n".repeat(50), "utf8");
    const fake = createFakePi({
      responses: [
        fauxAssistantMessage(fauxToolCall("read", { path: "big.txt" }), {
          stopReason: "toolUse",
        }),
        fauxAssistantMessage('{"ok":true}'),
      ],
    });
    const r = await runWorker(await deps(fake, worker(["read"]), policy()));
    const savings = r.meta.savings;
    expect(savings).toBeDefined();
    expect(savings?.sourceTokens).toBeGreaterThan(0);
    expect(savings?.returnedTokens).toBe(countTextTokens(JSON.stringify(r.result)));
    expect(savings?.avoidedContextTokens).toBe(
      Math.max(0, (savings?.sourceTokens ?? 0) - (savings?.returnedTokens ?? 0)),
    );
    // この fixture では read output が result を大きく上回る
    expect(savings?.avoidedContextTokens).toBeGreaterThan(0);
  });

  it("clamps avoidedContextTokens at zero when no tool output exists", async () => {
    const fake = createFakePi({ responses: [fauxAssistantMessage('{"ok":true}')] });
    const r = await runWorker(await deps(fake, worker(["read"]), policy()));
    expect(r.meta.savings).toEqual({
      sourceTokens: 0,
      returnedTokens: countTextTokens(JSON.stringify(r.result)),
      avoidedContextTokens: 0,
      directlyComparable: true,
    });
  });

  it("marks savings as not directly comparable for write-capable workers", async () => {
    const fake = createFakePi({ responses: [fauxAssistantMessage('{"ok":true}')] });
    const r = await runWorker(await deps(fake, worker(["read", "write"]), policy()));
    expect(r.meta.savings?.directlyComparable).toBe(false);
  });

  it("marks savings as not directly comparable when writeScope is declared", async () => {
    const fake = createFakePi({ responses: [fauxAssistantMessage('{"ok":true}')] });
    const d = await deps(fake, worker(["read"], { writeScope: "targets" }), policy());
    d.input = { task: "x", targets: [] };
    const r = await runWorker(d);
    expect(r.meta.savings?.directlyComparable).toBe(false);
  });

  it("reports sourceTokens as fully avoided on usage-bearing failure", async () => {
    await writeFile(path.join(root, "a.txt"), "alpha beta gamma\n".repeat(20), "utf8");
    const fake = createFakePi({
      responses: [
        fauxAssistantMessage(fauxToolCall("read", { path: "a.txt" }), { stopReason: "toolUse" }),
        fauxAssistantMessage(fauxToolCall("read", { path: "a.txt" }), { stopReason: "toolUse" }),
        fauxAssistantMessage("unreached"),
      ],
    });
    const p = policy();
    p.execution.maxToolCalls = 1;
    const err = await runWorker(await deps(fake, worker(["read"]), p)).then(
      () => undefined,
      (e: unknown) => e,
    );
    const savings = (err as WorkerRunError).runMeta?.savings;
    expect(savings).toBeDefined();
    expect(savings?.sourceTokens).toBeGreaterThan(0);
    expect(savings?.returnedTokens).toBe(0);
    expect(savings?.avoidedContextTokens).toBe(savings?.sourceTokens);
  });

  it("rejects worker tools outside policy allowlist", async () => {
    const fake = createFakePi({ responses: [fauxAssistantMessage("{}")] });
    const p = policy();
    p.tools.allow = ["read"];
    const err = await runWorker(await deps(fake, worker(["write"]), p)).then(
      () => undefined,
      (e: unknown) => e,
    );
    expect((err as CheaperError).code).toBe("POLICY_VIOLATION");
  });
});
