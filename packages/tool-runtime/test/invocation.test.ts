import { CheaperError } from "@cheaper/core";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  type CanonicalToolResult,
  createInvocationToolRuntime,
  type ToolRuntimeOptions,
} from "../src/index.js";

afterEach(() => vi.useRealTimers());
function gate<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

function options(): ToolRuntimeOptions {
  return {
    root: "/workspace",
    symlinkPolicy: "deny",
    names: ["write"],
    maxToolCalls: 20,
    onLimit: vi.fn(),
    handlers: { write: async () => ({ content: [{ type: "text", text: "done" }] }) },
  };
}
const input = { path: "a", content: "x" };
describe("invocation facade ownership", () => {
  it("records unique ids across owners and retains successful result and failed error", async () => {
    const invocation = createInvocationToolRuntime(options());
    expect(invocation.operations).toBeTypeOf("function");
    const result: CanonicalToolResult = {
      content: [{ type: "text", text: "result" }],
      details: { value: 1 },
    };
    const error = new CheaperError("TOOL_EXECUTION_FAILURE", "handler failed");
    const a = invocation.createWorkerScope({
      ...options(),
      ownerId: "a",
      handlers: { write: async () => result },
    });
    const b = invocation.createWorkerScope({
      ...options(),
      ownerId: "b",
      handlers: {
        write: async () => {
          throw error;
        },
      },
    });
    expect(await a.execute("write", input)).toBe(result);
    await expect(b.execute("write", input)).rejects.toBe(error);
    expect(invocation.operations().map((op) => [op.ownerId, op.state])).toEqual([
      ["a", "completed"],
      ["b", "failed"],
    ]);
    expect(new Set(invocation.operations().map((op) => op.id)).size).toBe(2);
    expect(a.operations()).toHaveLength(1);
    expect(b.operations()[0]?.error?.code).toBe("TOOL_EXECUTION_FAILURE");
    expect(a.operations()[0]?.finishedAt).toEqual(expect.any(Number));
    await invocation.dispose();
  });

  it("root timeout cancels every owner but stamps terminal only when handlers actually settle", async () => {
    const root = new AbortController();
    const invocation = createInvocationToolRuntime({ ...options(), signal: root.signal });
    const first = gate<CanonicalToolResult>();
    const second = gate<CanonicalToolResult>();
    const a = invocation.createWorkerScope({
      ...options(),
      ownerId: "a",
      handlers: { write: async () => first.promise },
    });
    const b = invocation.createWorkerScope({
      ...options(),
      ownerId: "b",
      handlers: { write: async () => second.promise },
    });
    const pa = a.execute("write", input);
    const pb = b.execute("write", input).then(
      (result) => ({ result }),
      (error) => ({ error }),
    );
    expect(invocation.operations).toBeTypeOf("function");
    root.abort(new CheaperError("TIMEOUT", "workflow deadline"));
    expect(
      invocation.operations().map((op) => [op.state, op.signal.aborted, op.finishedAt]),
    ).toEqual([
      ["running", true, undefined],
      ["running", true, undefined],
    ]);
    const result: CanonicalToolResult = { content: [{ type: "text", text: "late" }] };
    first.resolve(result);
    const failure = new Error("abort cleanup failed");
    second.reject(failure);
    expect(await pa).toBe(result);
    expect(await pb).toEqual({ error: failure });
    expect(invocation.operations().map((op) => op.state)).toEqual(["timed_out", "timed_out"]);
    expect(invocation.operations().every((op) => op.finishedAt !== undefined && !op.owned)).toBe(
      true,
    );
    expect(invocation.operations()[1]?.error?.message).toBe("abort cleanup failed");
    await invocation.dispose();
  });

  it("combines call and owner signals with first-cause reason while retaining sibling scope", async () => {
    const invocation = createInvocationToolRuntime(options());
    const child = new AbortController();
    const call = new AbortController();
    const first = gate<CanonicalToolResult>();
    const second = gate<CanonicalToolResult>();
    let n = 0;
    const a = invocation.createWorkerScope({
      ...options(),
      ownerId: "a",
      signal: child.signal,
      handlers: { write: async () => (++n === 1 ? first.promise : second.promise) },
    });
    const b = invocation.createWorkerScope({ ...options(), ownerId: "b" });
    const p1 = a.execute("write", input, { signal: call.signal });
    const p2 = a.execute("write", input);
    expect(a.operations).toBeTypeOf("function");
    call.abort(new CheaperError("ABORTED", "call cancelled"));
    expect(a.operations().map((op) => op.signal.aborted)).toEqual([true, false]);
    child.abort(new CheaperError("TIMEOUT", "worker deadline"));
    expect(a.operations().map((op) => op.signal.aborted)).toEqual([true, true]);
    await expect(a.execute("write", input)).rejects.toThrow("aborted");
    const result: CanonicalToolResult = { content: [{ type: "text", text: "done" }] };
    first.resolve(result);
    second.resolve(result);
    await Promise.all([p1, p2]);
    expect(a.operations().map((op) => [op.state, op.cancellation?.reason.code])).toEqual([
      ["cancelled", "ABORTED"],
      ["timed_out", "TIMEOUT"],
    ]);
    await a.dispose();
    expect((await b.execute("write", input)).content[0]?.text).toBe("done");
    const c = invocation.createWorkerScope({ ...options(), ownerId: "c" });
    expect((await c.execute("write", input)).content[0]?.text).toBe("done");
    await invocation.dispose();
  });

  it.each(["fulfilled", "rejected"] as const)(
    "bounds scope disposal and records late %s after detachedAt",
    async (outcome) => {
      vi.useFakeTimers();
      const invocation = createInvocationToolRuntime({ ...options(), abortGraceMs: 10 });
      const late = gate<CanonicalToolResult>();
      const a = invocation.createWorkerScope({
        ...options(),
        ownerId: "a",
        handlers: { write: async () => late.promise },
      });
      const b = invocation.createWorkerScope({ ...options(), ownerId: "b" });
      const pending = a.execute("write", input).then(
        (result) => ({ result }),
        (error) => ({ error }),
      );
      expect(a.operations).toBeTypeOf("function");
      const disposal = a.dispose();
      await vi.advanceTimersByTimeAsync(10);
      await disposal;
      expect(a.operations()[0]).toMatchObject({
        state: "running",
        settlement: "unsettled",
        owned: false,
        detachedAt: expect.any(Number),
      });
      expect(a.operations()[0]?.finishedAt).toBeUndefined();
      expect((await b.execute("write", input)).content[0]?.text).toBe("done");
      if (outcome === "fulfilled") late.resolve({ content: [{ type: "text", text: "late" }] });
      else late.reject(new Error("late failure"));
      await pending;
      expect(a.operations()[0]).toMatchObject({
        state: "cancelled",
        settlement: "settled",
        finishedAt: expect.any(Number),
        owned: false,
      });
      await a.dispose();
      await invocation.dispose();
    },
  );

  it("finite root disposal honors each scope's existing grace override", async () => {
    vi.useFakeTimers();
    const invocation = createInvocationToolRuntime({ ...options(), abortGraceMs: 100 });
    for (const [ownerId, abortGraceMs] of [
      ["a", 0],
      ["b", 20],
    ] as const) {
      const scope = invocation.createWorkerScope({
        ...options(),
        ownerId,
        abortGraceMs,
        handlers: { write: async () => new Promise<CanonicalToolResult>(() => {}) },
      });
      void scope.execute("write", input);
    }
    expect(invocation.operations).toBeTypeOf("function");
    let disposed = false;
    const pending = invocation.dispose().then(() => {
      disposed = true;
    });
    await vi.advanceTimersByTimeAsync(0);
    expect(invocation.operations("a")[0]?.settlement).toBe("unsettled");
    await vi.advanceTimersByTimeAsync(19);
    expect(disposed).toBe(false);
    await vi.advanceTimersByTimeAsync(1);
    await pending;
    expect(invocation.operations().every((op) => !op.owned)).toBe(true);
  });
  it("fixes the boundary configuration at construction and rejects unbounded grace", async () => {
    const opts = { root: "/workspace", symlinkPolicy: "deny" as const };
    const invocation = createInvocationToolRuntime(opts);
    opts.root = "/changed";
    expect(invocation.root).toBe("/workspace");
    expect(() =>
      invocation.createWorkerScope({ ...options(), root: opts.root, ownerId: "a" }),
    ).toThrow("root mismatch");
    expect(() =>
      createInvocationToolRuntime({ ...options(), abortGraceMs: Number.POSITIVE_INFINITY }),
    ).toThrow("finite");
    await invocation.dispose();
  });
  it("rejects duplicate owner, boundary mismatch, and unimplemented parent lease", async () => {
    const invocation = createInvocationToolRuntime(options());
    invocation.createWorkerScope({ ...options(), ownerId: "a" });
    expect(() => invocation.createWorkerScope({ ...options(), ownerId: "a" })).toThrow("owner");
    expect(() =>
      invocation.createWorkerScope({ ...options(), ownerId: "b", root: "/other" }),
    ).toThrow("root");
    expect(() =>
      invocation.createWorkerScope({
        ...options(),
        ownerId: "b",
        symlinkPolicy: "within-workspace",
      }),
    ).toThrow("policy");
    expect(() =>
      invocation.createWorkerScope({ ...options(), ownerId: "b", parentLease: {} as never }),
    ).toThrow("lease");
    await invocation.dispose();
    expect(() => invocation.createWorkerScope({ ...options(), ownerId: "c" })).toThrow("aborted");
  });
  it("child abort and dispose leave sibling and root alive", async () => {
    const invocation = createInvocationToolRuntime(options());
    const childAbort = new AbortController();
    const a = invocation.createWorkerScope({
      ...options(),
      ownerId: "a",
      signal: childAbort.signal,
    });
    const b = invocation.createWorkerScope({ ...options(), ownerId: "b" });
    childAbort.abort();
    await expect(a.execute("write", input)).rejects.toThrow("aborted");
    await a.dispose();
    expect((await b.execute("write", input)).content[0]?.text).toBe("done");
    expect(b.budget.toolCalls).toBe(1);
    const c = invocation.createWorkerScope({ ...options(), ownerId: "c" });
    invocation.abort();
    await expect(b.execute("write", input)).rejects.toThrow("aborted");
    await expect(c.execute("write", input)).rejects.toThrow("aborted");
    await invocation.dispose();
  });
  it("bounds drain for a non-cooperative handler and tolerates late settlement", async () => {
    const invocation = createInvocationToolRuntime({ ...options(), abortGraceMs: 10 });
    let finish!: () => void;
    let signal: AbortSignal | undefined;
    const scope = invocation.createWorkerScope({
      ...options(),
      ownerId: "a",
      handlers: {
        write: async (_, s) => {
          signal = s;
          await new Promise<void>((resolve) => {
            finish = resolve;
          });
          return { content: [{ type: "text", text: "late" }] };
        },
      },
    });
    const pending = scope.execute("write", input);
    expect(signal?.aborted).toBe(false);
    await invocation.dispose();
    expect(signal?.aborted).toBe(true);
    finish();
    await expect(pending).resolves.toMatchObject({ content: [{ text: "late" }] });
    await invocation.dispose();
  }, 1000);
});
