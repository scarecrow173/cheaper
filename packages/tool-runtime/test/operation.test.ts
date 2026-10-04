import { CheaperError } from "@cheaper/core";
import { afterEach, describe, expect, it, vi } from "vitest";
import * as api from "../src/index.js";

afterEach(() => vi.useRealTimers());
const input = { ownerId: "a", kind: "tool", toolName: "write" };
const states = ["queued", "running", "completed", "failed", "cancelled", "timed_out"] as const;
// 期待値は公開契約から手で定め、実装の transition table は再利用しない。
const allowed = [
  [false, true, false, false, true, true],
  [false, false, true, true, true, true],
  [false, false, false, false, false, false],
  [false, false, false, false, false, false],
  [false, false, false, false, false, false],
  [false, false, false, false, false, false],
];

function manager(options: ConstructorParameters<typeof api.OperationManager>[0] = {}) {
  expect(api.OperationManager).toBeTypeOf("function");
  return new api.OperationManager(options);
}

describe("operation state contract", () => {
  for (const [row, from] of states.entries()) {
    for (const [column, to] of states.entries()) {
      it(`${from} -> ${to}: ${allowed[row]?.[column] ? "valid" : "invalid"}`, () => {
        expect(api.assertOperationTransition).toBeTypeOf("function");
        const transition = () => api.assertOperationTransition(from, to);
        if (allowed[row]?.[column]) expect(transition).not.toThrow();
        else expect(transition).toThrow("transition");
      });
    }
  }

  it("stamps queued, started and actual terminal times and preserves result/failure", () => {
    let now = 10;
    const m = manager({ clock: () => now });
    const first = m.create(input);
    expect(first).toMatchObject({ state: "queued", createdAt: 10, owned: true });
    expect(first.startedAt).toBeUndefined();
    now = 20;
    expect(m.start(first.id)).toMatchObject({ state: "running", startedAt: 20 });
    now = 40;
    expect(m.complete(first.id, { answer: 1 })).toMatchObject({
      state: "completed",
      finishedAt: 40,
      result: { answer: 1 },
      owned: false,
      settlement: "settled",
    });
    const second = m.queue(input);
    expect(second.id).not.toBe(first.id);
    m.start(second.id);
    const failed = m.fail(second.id, new CheaperError("TOOL_EXECUTION_FAILURE", "broken"));
    expect(failed.error).toEqual({
      name: "CheaperError",
      message: "broken",
      code: "TOOL_EXECUTION_FAILURE",
    });
    expect(failed).toMatchObject({ state: "failed", finishedAt: 40 });
  });

  it("rejects lifecycle methods from the wrong state and never changes terminal metadata", () => {
    const m = manager();
    const queued = m.create(input);
    expect(() => m.complete(queued.id, 1)).toThrow("transition");
    expect(() => m.fail(queued.id, new Error("no"))).toThrow("transition");
    for (const terminal of ["completed", "failed", "cancelled", "timed_out"] as const) {
      const op = m.create(input);
      if (terminal === "completed" || terminal === "failed") {
        m.start(op.id);
        if (terminal === "completed") m.complete(op.id, 1);
        else m.fail(op.id, new Error("failure"));
      } else
        m.cancel(
          op.id,
          terminal === "timed_out" ? new DOMException("deadline", "TimeoutError") : "stop",
        );
      const before = m.get(op.id);
      for (const action of [
        () => m.start(op.id),
        () => m.complete(op.id, 2),
        () => m.fail(op.id, "changed"),
        () => m.cancel(op.id, "changed"),
      ])
        expect(action).toThrow("transition");
      expect(m.get(op.id)).toEqual(before);
    }
    expect(() => m.start("unknown")).toThrow("unknown operation");
  });

  it.each([
    ["TIMEOUT code", { code: "TIMEOUT", message: "deadline" }, "timed_out"],
    ["TimeoutError code", { code: "TimeoutError", message: "deadline" }, "timed_out"],
    ["TimeoutError name", new DOMException("deadline", "TimeoutError"), "timed_out"],
    ["other code", { code: "MAX_TURNS_EXCEEDED", message: "budget" }, "cancelled"],
  ] as const)("classifies %s with a queued terminal timestamp", (_, reason, want) => {
    const m = manager({ clock: () => 20 });
    const op = m.create(input);
    expect(m.cancel(op.id, reason)).toMatchObject({
      state: want,
      finishedAt: 20,
      settlement: "settled",
      owned: false,
    });
    expect(m.get(op.id).signal.aborted).toBe(true);
  });

  it("retains first cancellation cause and ends running only on actual settlement", () => {
    let now = 10;
    const m = manager({ clock: () => now });
    const op = m.create(input);
    now = 20;
    m.start(op.id);
    now = 30;
    m.cancel(op.id, new CheaperError("TIMEOUT", "first"));
    m.cancel(op.id, new Error("second"));
    expect(m.get(op.id)).toMatchObject({
      state: "running",
      cancellation: { requestedAt: 30, reason: { code: "TIMEOUT", message: "first" } },
    });
    expect(m.get(op.id).finishedAt).toBeUndefined();
    now = 50;
    expect(m.complete(op.id, "late result")).toMatchObject({
      state: "timed_out",
      finishedAt: 50,
      result: "late result",
    });
  });

  it("root AbortSignal cancels queued and running records and closes global admission", async () => {
    const root = new AbortController();
    const m = manager({ signal: root.signal, abortGraceMs: 0 });
    const queued = m.create(input);
    const running = m.create({ ...input, ownerId: "b" });
    m.start(running.id);
    root.abort(new CheaperError("ABORTED", "workflow cancellation"));
    expect(m.get(queued.id)).toMatchObject({
      state: "cancelled",
      cancellation: { reason: { message: "workflow cancellation" } },
    });
    expect(m.get(running.id).signal.aborted).toBe(true);
    expect(() => m.create({ ...input, ownerId: "c" })).toThrow("aborted");
    await m.dispose();
    expect(m.records().filter((op) => op.owned)).toHaveLength(0);
    m.fail(running.id, new Error("settled rejection"));
    expect(m.get(running.id)).toMatchObject({
      state: "cancelled",
      error: { message: "settled rejection" },
    });
    await m.dispose();
  });

  it("owner cancellation closes only its admission while individual call cancellation leaves siblings live", () => {
    const m = manager();
    const call = new AbortController();
    const a = m.create({ ...input, signal: call.signal });
    const b = m.create({ ...input, ownerId: "b" });
    call.abort("call stop");
    expect(m.get(a.id).state).toBe("cancelled");
    const sameOwner = m.create(input);
    m.cancelOwner("a", new Error("owner stop"));
    expect(m.get(sameOwner.id).state).toBe("cancelled");
    expect(() => m.create(input)).toThrow("aborted");
    m.start(b.id);
    expect(m.complete(b.id, "sibling").state).toBe("completed");
    m.cancelAll("root stop");
    expect(() => m.queue({ ...input, ownerId: "c" })).toThrow("aborted");
  });

  it("handles a pre-aborted root and a pre-aborted call without executing work", async () => {
    const root = new AbortController();
    root.abort("root stop");
    const m = manager({ signal: root.signal });
    expect(() => m.create(input)).toThrow("aborted");
    const call = new AbortController();
    call.abort("call stop");
    const n = manager();
    let executed = false;
    await expect(
      n.run({ ...input, signal: call.signal }, async () => {
        executed = true;
        return 1;
      }),
    ).rejects.toBe("call stop");
    expect(executed).toBe(false);
    expect(n.records()[0]).toMatchObject({ state: "cancelled", finishedAt: expect.any(Number) });
    expect(n.records()[0]?.startedAt).toBeUndefined();
  });

  it.each(["resolve", "reject"] as const)(
    "bounded drain classifies unsettled work and safely observes late %s",
    async (outcome) => {
      vi.useFakeTimers();
      const m = manager({ abortGraceMs: 10 });
      let resolve!: (value: string) => void;
      let reject!: (reason: unknown) => void;
      const pending = m.run(
        input,
        async () =>
          new Promise<string>((yes, no) => {
            resolve = yes;
            reject = no;
          }),
      );
      const observed = pending.then(
        (value) => ({ value }),
        (error) => ({ error }),
      );
      const disposal = m.dispose();
      expect(m.records()[0]).toMatchObject({ state: "running", signal: { aborted: true } });
      await vi.advanceTimersByTimeAsync(10);
      await disposal;
      expect(m.records()[0]).toMatchObject({
        state: "running",
        owned: false,
        settlement: "unsettled",
        detachedAt: expect.any(Number),
      });
      expect(m.records()[0]?.finishedAt).toBeUndefined();
      if (outcome === "resolve") resolve("late");
      else reject(new Error("late failure"));
      const result = await observed;
      if (outcome === "resolve") expect(result).toEqual({ value: "late" });
      else expect(result).toMatchObject({ error: { message: "late failure" } });
      expect(m.records()[0]).toMatchObject({
        state: "cancelled",
        owned: false,
        settlement: "settled",
        finishedAt: expect.any(Number),
      });
      await m.dispose();
    },
  );

  it("uses the existing default 5000ms grace and rejects nonfinite or negative grace", async () => {
    for (const abortGraceMs of [Number.NaN, Number.POSITIVE_INFINITY, -1])
      expect(() => manager({ abortGraceMs })).toThrow("finite");
    vi.useFakeTimers();
    const m = manager();
    const op = m.create(input);
    m.start(op.id);
    let disposed = false;
    const pending = m.dispose().then(() => {
      disposed = true;
    });
    await vi.advanceTimersByTimeAsync(4999);
    expect(disposed).toBe(false);
    await vi.advanceTimersByTimeAsync(1);
    await pending;
    expect(m.get(op.id).detachedAt).toEqual(expect.any(Number));
  });

  it("snapshots cannot rewrite lifecycle state or resource claims", () => {
    const m = manager();
    const claim = {
      resource: { kind: "workspace" as const, path: "/workspace/a", scope: "exact" as const },
      mode: "shared" as const,
    };
    const op = m.create({ ...input, resourceClaims: [claim] });
    claim.resource.path = "/changed";
    const snapshot = m.get(op.id);
    (snapshot as { state: string }).state = "failed";
    const exposed = snapshot.resourceClaims?.[0];
    if (!exposed) throw new Error("missing claim");
    exposed.resource.path = "/changed-snapshot";
    expect(m.get(op.id).resourceClaims?.[0]?.resource.path).toBe("/workspace/a");
    expect(m.get(op.id).state).toBe("queued");
  });
});
