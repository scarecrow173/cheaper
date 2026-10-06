import { CheaperError, countTextTokens } from "@cheaper/core";
import { afterEach, describe, expect, it, vi } from "vitest";
import * as api from "../src/index.js";

const result: api.CanonicalToolResult = { content: [{ type: "text", text: "done" }] };
function gate() {
  let resolve!: (value: api.CanonicalToolResult) => void;
  const promise = new Promise<api.CanonicalToolResult>((yes) => {
    resolve = yes;
  });
  return { promise, resolve: () => resolve(result) };
}
const cleanup: (() => Promise<void>)[] = [];
afterEach(async () => {
  for (const dispose of cleanup.splice(0)) await dispose();
  vi.useRealTimers();
});
const input = (name: string, path: string) =>
  name === "write"
    ? { path, content: "x" }
    : name === "edit"
      ? { path, edits: [{ oldText: "x", newText: "y" }] }
      : { path };

function fixture(maxOperations?: number, mutations: "serialized" | "per-resource" = "serialized") {
  const runtime = api.createInvocationToolRuntime({
    root: "/workspace",
    symlinkPolicy: "deny",
    abortGraceMs: 0,
    ...(maxOperations === undefined ? {} : { concurrency: { maxOperations, mutations } }),
  });
  const started: string[] = [];
  const gates = new Map<string, ReturnType<typeof gate>>();
  const promises: Promise<api.CanonicalToolResult>[] = [];
  const block = (key: string) => {
    let g = gates.get(key);
    if (!g) {
      g = gate();
      gates.set(key, g);
    }
    return g;
  };
  const scope = (ownerId: string, extra: Partial<api.ToolRuntimeOptions> = {}) =>
    runtime.createWorkerScope({
      root: "/workspace",
      symlinkPolicy: "deny",
      names: ["ls", "write", "edit"],
      maxToolCalls: 100,
      onLimit: vi.fn(),
      handlers: Object.fromEntries(
        ["ls", "write", "edit"].map((name) => [
          name,
          async (args: unknown) => {
            const key = (args as { path: string }).path;
            started.push(`${ownerId}:${name}:${key}`);
            return block(key).promise;
          },
        ]),
      ),
      ownerId,
      ...extra,
    });
  const track = (promise: Promise<api.CanonicalToolResult>) => {
    promises.push(promise);
    void promise.catch(() => {});
    return promise;
  };
  const run = (s: api.ToolRuntime, name: string, key: string, signal?: AbortSignal) =>
    track(s.execute(name, input(name, key), signal ? { signal } : undefined));
  cleanup.push(async () => {
    for (const g of gates.values()) g.resolve();
    runtime.abort();
    await runtime.dispose();
    await Promise.allSettled(promises);
  });
  return { runtime, started, block, scope, run, track };
}
const flush = async () => {
  for (let i = 0; i < 8; i++) await Promise.resolve();
};

describe("canonical operation scheduling", () => {
  it("overlaps reads to the configured invocation bound across owners and exposes the wait", async () => {
    const f = fixture(2);
    const a = f.scope("a"),
      b = f.scope("b", { concurrency: { maxOperations: 100 } });
    const p = [f.run(a, "ls", "one"), f.run(b, "ls", "two"), f.run(a, "ls", "three")];
    expect(f.started).toEqual(["a:ls:one", "b:ls:two"]);
    expect(f.runtime.operations().map((o) => o.state)).toEqual(["running", "running", "queued"]);
    expect(f.runtime.operations()[2]?.startedAt).toBeUndefined();
    f.block("one").resolve();
    await p[0];
    await flush();
    expect(f.started).toEqual(["a:ls:one", "b:ls:two", "a:ls:three"]);
    const last = f.runtime.operations()[2];
    expect(last?.state).toBe("running");
    expect(last?.startedAt).toBeGreaterThanOrEqual(last?.createdAt ?? Infinity);
    f.block("two").resolve();
    f.block("three").resolve();
    await Promise.all(p);
  });
  it("defaults standalone runtime to eight simultaneous shared operations", async () => {
    const gates = Array.from({ length: 9 }, gate);
    let started = 0;
    const runtime = api.createToolRuntime({
      root: "/workspace",
      symlinkPolicy: "deny",
      names: ["ls"],
      maxToolCalls: 20,
      onLimit: vi.fn(),
      abortGraceMs: 0,
      handlers: {
        ls: async () => {
          const g = gates[started++];
          if (!g) throw new Error("missing gate");
          return g.promise;
        },
      },
    });
    const p = Array.from({ length: 9 }, () => runtime.execute("ls", { path: "." }));
    for (const promise of p) void promise.catch(() => {});
    try {
      expect(started).toBe(8);
      expect(runtime.operations().filter((o) => o.state === "queued")).toHaveLength(1);
      gates[0]?.resolve();
      await p[0];
      await flush();
      expect(started).toBe(9);
    } finally {
      for (const g of gates) g.resolve();
      await Promise.allSettled(p);
      await runtime.dispose();
    }
  });
  it.each(["serialized", "per-resource"] as const)(
    "serializes mutations and prevents read/writer overlap under %s",
    async (mode) => {
      const f = fixture(4, mode),
        a = f.scope("a"),
        b = f.scope("b");
      const p = [
        f.run(a, "ls", "read"),
        f.run(b, "write", "A"),
        f.run(a, "edit", "B"),
        f.run(b, "write", "C"),
        f.run(a, "ls", "tail"),
      ];
      expect(f.started).toEqual(["a:ls:read"]);
      for (const [index, key, next] of [
        [0, "read", "b:write:A"],
        [1, "A", "a:edit:B"],
        [2, "B", "b:write:C"],
        [3, "C", "a:ls:tail"],
      ] as const) {
        f.block(key).resolve();
        await p[index];
        await flush();
        expect(f.started.at(-1)).toBe(next);
        expect(f.runtime.operations().filter((o) => o.state === "running")).toHaveLength(1);
      }
      f.block("tail").resolve();
      await Promise.all(p);
      expect(f.started).toEqual(["a:ls:read", "b:write:A", "a:edit:B", "b:write:C", "a:ls:tail"]);
    },
  );
  it("cancels a waiting writer without blocking following shared work or losing first cause", async () => {
    const f = fixture(2),
      a = f.scope("a"),
      b = f.scope("b");
    const call = new AbortController();
    const first = new CheaperError("ABORTED", "caller cancelled");
    const p = f.run(a, "ls", "read");
    const queued = f.run(b, "write", "writer", call.signal);
    const tail = f.run(a, "ls", "tail");
    expect(b.operations()[0]?.state).toBe("queued");
    call.abort(first);
    b.abort(new CheaperError("TIMEOUT", "later timeout"));
    await expect(queued).rejects.toBe(first);
    await flush();
    expect(f.started).toEqual(["a:ls:read", "a:ls:tail"]);
    expect(b.operations()[0]).toMatchObject({
      state: "cancelled",
      cancellation: { reason: { code: "ABORTED" } },
    });
    expect(b.operations()[0]?.startedAt).toBeUndefined();
    f.block("read").resolve();
    f.block("tail").resolve();
    await Promise.all([p, tail]);
  });
  it.each(["owner", "root"] as const)(
    "cancels all queued %s work without starting any handler during propagation",
    async (kind) => {
      const f = fixture(1),
        a = f.scope("a"),
        b = f.scope("b");
      const reason = new CheaperError("TIMEOUT", "deadline");
      const p = f.run(a, "ls", "running"),
        one = f.run(b, "ls", "one"),
        two = f.run(b, "write", "two");
      expect(b.operations().map((o) => o.state)).toEqual(["queued", "queued"]);
      if (kind === "root") f.runtime.abort(reason);
      else b.abort(reason);
      await expect(one).rejects.toBe(reason);
      await expect(two).rejects.toBe(reason);
      await flush();
      expect(f.started).toEqual(["a:ls:running"]);
      expect(
        b.operations().every((o) => o.state === "timed_out" && o.startedAt === undefined),
      ).toBe(true);
      f.block("running").resolve();
      await p;
    },
  );
  it("defers re-entrant enqueue admission until owner cancellation has reached every queued record", async () => {
    const f = fixture(2),
      a = f.scope("a"),
      b = f.scope("b");
    const reason = new CheaperError("ABORTED", "owner cancelled");
    const running = f.run(a, "ls", "running");
    const writer = f.run(b, "write", "writer");
    const cancelledRead = f.run(b, "ls", "cancelled-read");
    let reentrant: Promise<api.CanonicalToolResult> | undefined;
    b.operations()[0]?.signal.addEventListener(
      "abort",
      () => {
        reentrant = f.run(a, "ls", "reentrant");
      },
      { once: true },
    );
    b.abort(reason);
    expect(f.started).toEqual(["a:ls:running"]);
    await flush();
    expect(f.started).toEqual(["a:ls:running", "a:ls:reentrant"]);
    await expect(writer).rejects.toBe(reason);
    await expect(cancelledRead).rejects.toBe(reason);
    expect(b.operations().every((o) => o.state === "cancelled" && o.startedAt === undefined)).toBe(
      true,
    );
    f.block("running").resolve();
    f.block("reentrant").resolve();
    await Promise.all([running, reentrant]);
  });
  it("holds capacity after non-cooperative owner detach until actual late settlement", async () => {
    const f = fixture(1),
      a = f.scope("a"),
      b = f.scope("b");
    const late = f.run(a, "write", "late");
    await a.dispose();
    expect(a.operations()[0]).toMatchObject({
      state: "running",
      owned: false,
      settlement: "unsettled",
    });
    const next = f.run(b, "ls", "next");
    expect(f.started).toEqual(["a:write:late"]);
    expect(b.operations()[0]?.state).toBe("queued");
    f.block("late").resolve();
    await late;
    await flush();
    expect(a.operations()[0]?.state).toBe("cancelled");
    expect(f.started.at(-1)).toBe("b:ls:next");
    f.block("next").resolve();
    await next;
  });
  it("keeps descriptor decoration separate from the raw handler and accounts exactly once", async () => {
    const f = fixture(1),
      onLimit = vi.fn(),
      scope = f.scope("a", { maxToolCalls: 1, onLimit });
    const descriptor = scope.definitions.find((d) => d.name === "write");
    if (!descriptor) throw new Error("missing descriptor");
    const original = descriptor.execute;
    descriptor.execute = vi.fn((args, signal) => original(args, signal));
    const p = f.track(descriptor.execute(input("write", "one")));
    expect(scope.budget.toolCalls).toBe(1);
    await expect(f.run(scope, "write", "two")).rejects.toThrow("maxToolCalls");
    expect(scope.budget).toMatchObject({ toolCalls: 1, rejectedToolCalls: 1 });
    expect(onLimit).toHaveBeenCalledTimes(1);
    expect(descriptor.execute).toHaveBeenCalledTimes(1);
    expect(scope.operations()).toHaveLength(1);
    expect(f.started).toEqual(["a:write:one"]);
    f.block("one").resolve();
    await p;
    expect(scope.meter.sourceTokens).toBe(countTextTokens("done"));
  });
  it("rejects pre-aborted facade calls with their original reason before budget reservation", async () => {
    const f = fixture(1),
      scope = f.scope("a"),
      call = new AbortController();
    const reason = new CheaperError("TIMEOUT", "pre-aborted");
    call.abort(reason);
    await expect(f.run(scope, "ls", "unused", call.signal)).rejects.toBe(reason);
    expect(scope.budget.toolCalls).toBe(0);
    expect(scope.operations()).toHaveLength(0);
    scope.abort(reason);
    await expect(f.run(scope, "ls", "unused")).rejects.toBe(reason);
  });
});

describe("scheduler and OperationManager admission", () => {
  it("preserves pre-aborted composite first cause and terminal state without enqueue/start", async () => {
    expect(api.ToolScheduler).toBeTypeOf("function");
    const manager = new api.OperationManager({ abortGraceMs: 0 });
    const scheduler = new api.ToolScheduler(manager, { maxOperations: 1 });
    const c = new AbortController();
    const reason = new CheaperError("TIMEOUT", "already expired");
    c.abort(reason);
    const execute = vi.fn(async () => result);
    await expect(
      scheduler.run(
        { ownerId: "a", kind: "tool", signal: AbortSignal.any([c.signal]) },
        { access: "shared" },
        execute,
      ),
    ).rejects.toBe(reason);
    expect(manager.records()[0]).toMatchObject({
      state: "timed_out",
      cancellation: { reason: { code: "TIMEOUT" } },
    });
    expect(manager.records()[0]?.startedAt).toBeUndefined();
    expect(execute).not.toHaveBeenCalled();
    await manager.dispose();
  });
  it("records exact queue wait and releases failed execution capacity", async () => {
    expect(api.ToolScheduler).toBeTypeOf("function");
    let now = 10;
    const manager = new api.OperationManager({ clock: () => now, abortGraceMs: 0 });
    const scheduler = new api.ToolScheduler(manager, { maxOperations: 1 });
    let fail!: (reason: unknown) => void;
    const first = scheduler.run(
      { ownerId: "a", kind: "tool" },
      { access: "shared" },
      () =>
        new Promise((_yes, no) => {
          fail = no;
        }),
    );
    void first.catch(() => {});
    now = 20;
    const second = scheduler.run(
      { ownerId: "b", kind: "process" },
      { access: "shared" },
      async () => result,
    );
    expect(manager.records()[1]).toMatchObject({ state: "queued", createdAt: 20 });
    now = 50;
    const error = new Error("handler failed");
    fail(error);
    await expect(first).rejects.toBe(error);
    await second;
    expect(manager.records().map((o) => o.state)).toEqual(["failed", "completed"]);
    expect(manager.records()[1]).toMatchObject({ createdAt: 20, startedAt: 50, finishedAt: 50 });
    await manager.dispose();
  });
});
