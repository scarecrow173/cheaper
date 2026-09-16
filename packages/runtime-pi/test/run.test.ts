import { mkdtemp, readFile, realpath, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import {
  type CheaperError,
  compileSchema,
  type RuntimePolicy,
  type WorkerDefinition,
} from "@cheaper/core";
import {
  type AssistantMessage,
  type Context,
  createAssistantMessageEventStream,
  type Model,
  type SimpleStreamOptions,
} from "@earendil-works/pi-ai";
import { ModelRuntime } from "@earendil-works/pi-coding-agent";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
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

function worker(tools: WorkerDefinition["tools"]): WorkerDefinition {
  return {
    name: "test-worker",
    description: "test",
    tools,
    cheaper: { model: { tier: "low" }, output: { schema: { inline: {} } } },
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

describe("runWorker", () => {
  it("returns result and meta on the happy path", async () => {
    const fake = createFakePi({ responses: [fauxAssistantMessage('{"ok":true}')] });
    const r = await runWorker(await deps(fake, worker(["read"]), policy()));
    expect(r.result).toEqual({ ok: true });
    expect(r.meta.model).toEqual({
      provider: "cheaper-fake",
      id: "fake-model",
      tier: "low",
    });
    expect(r.meta.turns).toBe(1);
    expect(r.meta.toolCalls).toBe(0);
    expect(r.meta.rejectedToolCalls).toBe(0);
    expect(r.meta.repairs).toBe(0);
    expect(r.meta.usage.output).toBeGreaterThanOrEqual(0);
    expect(r.meta.changes.totals.files).toBe(0);
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
