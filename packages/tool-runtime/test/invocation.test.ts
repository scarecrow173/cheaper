import { describe, expect, it, vi } from "vitest";
import { createInvocationToolRuntime, type ToolRuntimeOptions } from "../src/index.js";

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
