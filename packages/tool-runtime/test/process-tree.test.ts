import { EventEmitter } from "node:events";
import { PassThrough } from "node:stream";
import { afterEach, describe, expect, it, vi } from "vitest";
import { createProcessTree } from "../src/process-tree.js";

const state = vi.hoisted(() => ({
  rows: [] as Array<{ pid: number; parent: number; created: string }>,
  graceful: false,
  failQuery: false,
  snapshots: 0,
  calls: [] as string[][],
}));
vi.mock("node:child_process", () => ({
  spawn: (_executable: string, args: string[]) => {
    const child = Object.assign(new EventEmitter(), { stdout: new PassThrough(), kill: vi.fn() });
    queueMicrotask(() => {
      if (args.includes("-Command")) {
        state.snapshots++;
        if (state.failQuery) {
          child.emit("close", 1);
          return;
        }
        child.stdout.write(JSON.stringify(state.rows));
        child.emit("close", 0);
      } else {
        state.calls.push(args);
        if (state.graceful || args.includes("/F")) state.rows = [];
        child.emit("close", state.rows.length ? 1 : 0);
      }
    });
    return child;
  },
}));
const platform = Object.getOwnPropertyDescriptor(process, "platform")!;
afterEach(() => {
  Object.defineProperty(process, "platform", platform);
  vi.restoreAllMocks();
});
function setup() {
  Object.defineProperty(process, "platform", { ...platform, value: "win32" });
  state.calls = [];
  state.snapshots = 0;
  state.failQuery = false;
  state.graceful = false;
  const created = new Date(Date.now() - 1000).toISOString();
  state.rows = [
    { pid: 41, parent: 1, created },
    { pid: 42, parent: 41, created },
    { pid: 43, parent: 42, created },
  ];
}
describe("process tree backend contracts", { timeout: 30000 }, () => {
  it("does not force kill after graceful tree termination", async () => {
    setup();
    state.graceful = true;
    await createProcessTree(41).terminate(50);
    expect(state.calls).toHaveLength(1);
    expect(state.calls[0]).toContain("/T");
    expect(state.calls[0]).not.toContain("/F");
  });
  it("escalates remaining identities even when non-force taskkill rejects", async () => {
    setup();
    await createProcessTree(41).terminate(1);
    expect(state.calls).toHaveLength(2);
    expect(state.calls[1]).toEqual(["/PID", "41", "/PID", "42", "/PID", "43", "/T", "/F"]);
    expect(state.rows).toEqual([]);
  });
  it("does not target reused root or descendant identities", async () => {
    setup();
    const tree = createProcessTree(41);
    await new Promise((resolve) => setTimeout(resolve, 0));
    state.rows = state.rows.map((p) => ({
      ...p,
      parent: 999,
      created: new Date(Date.now() + 1000).toISOString(),
    }));
    await tree.terminate(1);
    expect(state.calls).toEqual([]);
  });
  it("fails closed on identity query failure", async () => {
    setup();
    state.failQuery = true;
    await expect(createProcessTree(41).terminate(1)).rejects.toThrow("helper failed");
    expect(state.calls).toEqual([]);
  });
  it("waits for POSIX group disappearance after force termination", async () => {
    Object.defineProperty(process, "platform", { ...platform, value: "darwin" });
    let remaining = 1;
    const kill = vi.spyOn(process, "kill").mockImplementation((_pid, signal) => {
      if (signal === "SIGKILL") remaining = 3;
      if (signal === 0 && --remaining <= 0)
        throw Object.assign(new Error("gone"), { code: "ESRCH" });
      return true;
    });
    // SIGTERM 後も残存する group を再現する。
    remaining = 100;
    await createProcessTree(41).terminate(1);
    expect(kill.mock.calls.some(([, signal]) => signal === "SIGKILL")).toBe(true);
    expect(remaining).toBeLessThanOrEqual(0);
  });
});
