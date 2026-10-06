import { mkdtemp, realpath, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { CheaperError, countTextTokens } from "@cheaper/core";
import {
  type CanonicalToolResult,
  createInvocationToolRuntime,
  createToolRuntime,
  MutationJournal,
} from "@cheaper/tool-runtime";
import { ModelRuntime } from "@earendil-works/pi-coding-agent";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createWorkerSession } from "../src/session.js";
import { createSandboxedTools } from "../src/tools/registry.js";
import { createFakePi, fauxAssistantMessage, fauxToolCall } from "./fixtures/fake-stream.js";

let root: string;
beforeEach(async () => {
  root = await realpath(await mkdtemp(path.join(tmpdir(), "cheaper-scheduler-")));
});
afterEach(async () => {
  await rm(root, { recursive: true, force: true });
});
function gate() {
  let resolve!: () => void;
  const promise = new Promise<void>((yes) => {
    resolve = yes;
  });
  return { promise, resolve };
}
const result: CanonicalToolResult = { content: [{ type: "text", text: "done" }] };

describe("Pi and direct tool scheduling boundary", () => {
  it("shares the invocation cap between Pi sibling calls and Workflow-style direct callers", async () => {
    const gates = new Map(["direct", "a", "b", "c"].map((key) => [key, gate()]));
    const started: string[] = [];
    let active = 0,
      peak = 0;
    const options = {
      root,
      symlinkPolicy: "deny" as const,
      names: ["ls"] as const,
      maxToolCalls: 20,
      onLimit: vi.fn(),
      journal: new MutationJournal(),
      handlers: {
        ls: async (input: unknown) => {
          const key = (input as { path: string }).path;
          const g = gates.get(key);
          if (!g) throw new Error("missing gate");
          started.push(key);
          active++;
          peak = Math.max(peak, active);
          try {
            await g.promise;
            return result;
          } finally {
            active--;
          }
        },
      },
    };
    const invocation = createInvocationToolRuntime({
      ...options,
      concurrency: { maxOperations: 2, mutations: "serialized" },
      abortGraceMs: 0,
    });
    const direct = invocation.createWorkerScope({ ...options, ownerId: "workflow-tool" });
    const onLimit = vi.fn();
    const pi = invocation.createWorkerScope({
      ...options,
      ownerId: "worker",
      maxToolCalls: 3,
      onLimit,
    });
    const sandboxed = createSandboxedTools({ ...options, names: ["ls"], runtime: pi });
    const modelRuntime = await ModelRuntime.create({ refreshOnCreate: false });
    const fake = createFakePi({
      responses: [
        fauxAssistantMessage(
          ["a", "b", "c", "over-budget"].map((key, index) =>
            fauxToolCall("ls", { path: key }, { id: `call-${index}` }),
          ),
          { stopReason: "toolUse" },
        ),
        fauxAssistantMessage('{"done":true}'),
      ],
    });
    modelRuntime.registerNativeProvider(fake.provider);
    const { session } = await createWorkerSession({
      cwd: root,
      model: fake.model,
      modelRuntime,
      systemPrompt: "scheduler probe",
      customTools: sandboxed.tools,
      toolNames: ["ls"],
      maxOutputTokens: 512,
      transientRetries: 0,
      streamFn: fake.streamFn,
    });
    const pending = direct.execute("ls", { path: "direct" });
    const prompt = session.prompt("go");
    void prompt.catch(() => {});
    try {
      await vi.waitFor(() => expect(pi.budget.toolCalls).toBe(3));
      expect(started).toEqual(["direct", "a"]);
      expect(peak).toBe(2);
      expect(invocation.operations().map((o) => o.state)).toEqual([
        "running",
        "running",
        "queued",
        "queued",
      ]);
      gates.get("direct")?.resolve();
      await pending;
      await vi.waitFor(() => expect(started).toEqual(["direct", "a", "b"]));
      gates.get("a")?.resolve();
      await vi.waitFor(() => expect(started).toEqual(["direct", "a", "b", "c"]));
      gates.get("b")?.resolve();
      gates.get("c")?.resolve();
      await prompt;
      expect(peak).toBe(2);
      expect(pi.operations()).toHaveLength(3);
      expect(direct.operations()).toHaveLength(1);
      expect(pi.budget.toolCalls).toBe(3);
      expect(pi.meter.sourceTokens).toBe(3 * countTextTokens("done"));
      expect(pi.budget.rejectedToolCalls).toBe(1);
      expect(onLimit).toHaveBeenCalledTimes(1);
    } finally {
      for (const g of gates.values()) g.resolve();
      await Promise.allSettled([pending, prompt]);
      session.dispose();
      await invocation.dispose();
    }
  });
  it("propagates Pi call-context cancellation through the same queued operation and accounting", async () => {
    const running = gate(),
      queued = gate();
    const handler = vi.fn(async (args: unknown) => {
      await ((args as { path: string }).path === "running" ? running.promise : queued.promise);
      return result;
    });
    const options = {
      root,
      symlinkPolicy: "deny" as const,
      names: ["ls"] as const,
      maxToolCalls: 20,
      onLimit: vi.fn(),
      journal: new MutationJournal(),
      concurrency: { maxOperations: 1, mutations: "serialized" as const },
      handlers: { ls: handler },
      abortGraceMs: 0,
    };
    const runtime = createToolRuntime(options);
    const tools = createSandboxedTools({ ...options, names: ["ls"], runtime });
    const controller = new AbortController(),
      reason = new CheaperError("ABORTED", "Pi context cancelled");
    const first = runtime.execute("ls", { path: "running" });
    const tool = tools.tools[0];
    if (!tool) throw new Error("missing tool");
    const second = tool.execute(
      "queued",
      { path: "queued" },
      controller.signal,
      undefined,
      {} as never,
    );
    void second.catch(() => {});
    try {
      expect(runtime.operations().map((o) => o.state)).toEqual(["running", "queued"]);
      controller.abort(reason);
      await expect(second).rejects.toBe(reason);
      expect(handler).toHaveBeenCalledTimes(1);
      expect(runtime.operations()[1]).toMatchObject({
        state: "cancelled",
        cancellation: { reason: { code: "ABORTED" } },
      });
      expect(runtime.budget).toMatchObject({ toolCalls: 2, rejectedToolCalls: 0 });
      running.resolve();
      await first;
      expect(runtime.meter.sourceTokens).toBe(countTextTokens("done"));
    } finally {
      running.resolve();
      queued.resolve();
      await Promise.allSettled([first, second]);
      await runtime.dispose();
    }
  });
});
