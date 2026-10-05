import { CheaperError } from "@cheaper/core";
import { afterEach, describe, expect, it, vi } from "vitest";
import { createInvocationToolRuntime, OperationManager } from "../src/index.js";

afterEach(() => vi.useRealTimers());

const states = {
  queued: 0,
  running: 0,
  completed: 0,
  failed: 0,
  cancelled: 0,
  timed_out: 0,
};
const operation = { ownerId: "a", kind: "tool", toolName: "read" };

describe("execution snapshots", () => {
  it("returns a zero snapshot and all six states", () => {
    const m = new OperationManager({ clock: () => 0 });
    expect(m.executionMetrics()).toEqual({
      operations: 0,
      byState: states,
      byTool: {},
      maxConcurrency: 0,
      operationMs: 0,
      activeMs: 0,
      overlapMs: 0,
      queueMs: 0,
    });
  });

  it.each([false, true])("measures serial versus overlapping execution: overlap=%s", (overlap) => {
    let now = 0;
    const m = new OperationManager({ clock: () => now });
    const first = m.create(operation);
    now = 10;
    m.start(first.id);
    if (!overlap) {
      now = 30;
      m.complete(first.id, null);
    }
    now = overlap ? 10 : 35;
    const second = m.create(operation);
    now = overlap ? 20 : 45;
    m.start(second.id);
    if (overlap) {
      now = 40;
      m.complete(first.id, null);
    }
    now = overlap ? 50 : 65;
    m.complete(second.id, null);
    now = 100;
    expect(m.executionMetrics()).toEqual({
      operations: 2,
      byState: { ...states, completed: 2 },
      byTool: { read: 2 },
      maxConcurrency: overlap ? 2 : 1,
      operationMs: overlap ? 60 : 40,
      activeMs: 40,
      overlapMs: overlap ? 20 : 0,
      queueMs: 20,
    });
  });

  it("keeps owner-local union and maximum separate from invocation totals", () => {
    let now = 0;
    const m = new OperationManager({ clock: () => now });
    const a = m.create(operation);
    m.start(a.id);
    now = 10;
    const b = m.create({ ...operation, ownerId: "b", toolName: "grep" });
    m.start(b.id);
    now = 20;
    m.complete(a.id, null);
    now = 30;
    m.complete(b.id, null);
    expect(m.executionMetrics("a")).toMatchObject({
      operations: 1,
      maxConcurrency: 1,
      activeMs: 20,
    });
    expect(m.executionMetrics("b")).toMatchObject({
      operations: 1,
      maxConcurrency: 1,
      activeMs: 20,
    });
    expect(m.executionMetrics()).toMatchObject({
      operations: 2,
      maxConcurrency: 2,
      operationMs: 40,
      activeMs: 30,
      overlapMs: 10,
    });
  });

  it.each([false, true])(
    "observes zero-duration concurrency rather than guessing clock tie order: %s",
    (overlap) => {
      const m = new OperationManager({ clock: () => 0 });
      const a = m.create(operation);
      m.start(a.id);
      if (!overlap) m.complete(a.id, null);
      const b = m.create(operation);
      m.start(b.id);
      if (overlap) m.complete(a.id, null);
      m.complete(b.id, null);
      expect(m.executionMetrics()).toMatchObject({
        maxConcurrency: overlap ? 2 : 1,
        operationMs: 0,
        activeMs: 0,
        overlapMs: 0,
      });
    },
  );

  it("counts failed, queued cancelled/timed-out and pending wait intervals", () => {
    let now = 0;
    const m = new OperationManager({ clock: () => now });
    const queued = m.create(operation);
    const timeout = m.create({ ...operation, toolName: "grep" });
    const failed = m.create({ ...operation, toolName: "write" });
    m.create({ ownerId: "a", kind: "process" });
    now = 5;
    m.start(failed.id);
    now = 10;
    m.cancel(queued.id);
    m.cancel(timeout.id, new CheaperError("TIMEOUT", "timeout"));
    now = 15;
    m.fail(failed.id, new Error("failed"));
    now = 20;
    expect(m.executionMetrics()).toEqual({
      operations: 4,
      byState: { ...states, queued: 1, cancelled: 1, timed_out: 1, failed: 1 },
      byTool: { read: 1, grep: 1, write: 1, process: 1 },
      maxConcurrency: 1,
      operationMs: 10,
      activeMs: 10,
      overlapMs: 0,
      queueMs: 45,
    });
  });

  it("clamps running to snapshot, keeps cancelled/detached work active until actual late settle", async () => {
    vi.useFakeTimers();
    let now = 0;
    const m = new OperationManager({ clock: () => now, abortGraceMs: 10 });
    const a = m.create(operation);
    m.start(a.id);
    now = 20;
    m.cancel(a.id, new CheaperError("TIMEOUT", "timeout"));
    expect(m.executionMetrics()).toMatchObject({
      byState: { ...states, running: 1 },
      operationMs: 20,
    });
    const draining = m.drain();
    now = 30;
    await vi.advanceTimersByTimeAsync(10);
    await draining;
    expect(m.get(a.id).settlement).toBe("unsettled");
    now = 50;
    const detached = m.executionMetrics();
    expect(detached).toMatchObject({
      byState: { ...states, running: 1 },
      operationMs: 50,
      activeMs: 50,
    });
    now = 80;
    m.complete(a.id, null);
    now = 100;
    expect(m.executionMetrics()).toMatchObject({
      byState: { ...states, timed_out: 1 },
      operationMs: 80,
      activeMs: 80,
    });
    expect(detached.operationMs).toBe(50);
    expect(() => m.create(operation)).toThrow("admission closed");
    expect(m.executionMetrics().operations).toBe(1);
  });

  it("preserves prototype-like tool names and independent snapshot objects", () => {
    const m = new OperationManager({ clock: () => 0 });
    for (const toolName of ["__proto__", "constructor", "read"]) {
      const a = m.create({ ...operation, toolName });
      m.start(a.id);
      m.complete(a.id, null);
    }
    const snap = m.executionMetrics();
    expect(JSON.parse(JSON.stringify(snap.byTool))).toEqual({
      ["__proto__"]: 1,
      constructor: 1,
      read: 1,
    });
    snap.byState.completed = 99;
    snap.byTool.read = 99;
    expect(m.executionMetrics()).toMatchObject({
      operations: 3,
      byState: { ...states, completed: 3 },
      byTool: { read: 1 },
    });
  });

  it("uses the invocation clock and records real scheduler wait without changing admission", async () => {
    let now = 0;
    let release!: () => void;
    const waiting = new Promise<void>((resolve) => {
      release = resolve;
    });
    let started!: () => void;
    const firstStarted = new Promise<void>((resolve) => {
      started = resolve;
    });
    const invocation = createInvocationToolRuntime({
      root: "/workspace",
      symlinkPolicy: "deny",
      clock: () => now,
      concurrency: { maxOperations: 1 },
    });
    const common = {
      root: "/workspace",
      symlinkPolicy: "deny" as const,
      names: ["ls"] as const,
      maxToolCalls: 10,
      onLimit: () => {},
    };
    const first = invocation.createWorkerScope({
      ...common,
      ownerId: "first",
      handlers: {
        ls: async () => {
          started();
          await waiting;
          return { content: [] };
        },
      },
    });
    const second = invocation.createWorkerScope({
      ...common,
      ownerId: "second",
      handlers: {
        ls: async () => {
          now = 30;
          return { content: [] };
        },
      },
    });
    const a = first.execute("ls", { path: "." });
    await firstStarted;
    now = 5;
    const b = second.execute("ls", { path: "." });
    await Promise.resolve();
    now = 10;
    expect(invocation.executionMetrics("second")).toMatchObject({
      byState: { ...states, queued: 1 },
      queueMs: 5,
    });
    now = 20;
    release();
    await Promise.all([a, b]);
    expect(invocation.executionMetrics()).toMatchObject({
      operations: 2,
      maxConcurrency: 1,
      operationMs: 30,
      activeMs: 30,
      overlapMs: 0,
      queueMs: 15,
    });
    await invocation.dispose();
  });
});
