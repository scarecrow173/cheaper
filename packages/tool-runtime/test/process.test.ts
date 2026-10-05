import { access, mkdir, mkdtemp, readFile, realpath, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  compileSchema,
  createPathDenylist,
  type ProcessPolicy,
  resolveProcessPolicy,
} from "@cheaper/core";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as api from "../src/index.js";
import { createProcessTool } from "../src/process.js";
import * as treeBackend from "../src/process-tree.js";

const script = fileURLToPath(new URL("./fixtures/process-child.mjs", import.meta.url));
let root: string;
const runtimes: Array<{ dispose(): Promise<void> }> = [];
beforeEach(async () => {
  root = await realpath(await mkdtemp(path.join(process.cwd(), ".tmp-process-runtime-")));
});
afterEach(async () => {
  for (const runtime of runtimes.splice(0)) await runtime.dispose();
  vi.unstubAllEnvs();
  vi.restoreAllMocks();
  expect(path.dirname(root)).toBe(await realpath(process.cwd()));
  await rm(root, { recursive: true, force: true });
});

describe("bounded process lifecycle", () => {
  it("preserves outer reason when tree cleanup fails", async () => {
    vi.spyOn(treeBackend, "createProcessTree").mockReturnValue({
      terminate: async () => {
        throw new Error("cleanup failure");
      },
    });
    const diagnostic = vi.spyOn(process.stderr, "write").mockReturnValue(true);
    const pidPath = path.join(root, "pid");
    const controller = new AbortController(),
      reason = new Error("outer reason");
    const r = runtime(config({ stubborn: task("stubborn", [pidPath]) }), {
      signal: controller.signal,
    });
    const pending = r.execute("process", { task: "stubborn" });
    void pending.catch(() => {});
    await vi.waitFor(async () => expect(await readFile(pidPath, "utf8")).toMatch(/^\d+$/));
    controller.abort(reason);
    await expect(pending).rejects.toBe(reason);
    expect(r.operations()[0]?.state).toBe("cancelled");
    expect(await alive(Number(await readFile(pidPath, "utf8")))).toBe(false);
    expect(
      diagnostic.mock.calls.some(([line]) => String(line).includes("process.cleanup-failed")),
    ).toBe(true);
  });

  it("drops heartbeat under diagnostic backpressure", async () => {
    const gate = path.join(root, "gate");
    const descriptor = Object.getOwnPropertyDescriptor(process.stderr, "writableNeedDrain");
    Object.defineProperty(process.stderr, "writableNeedDrain", { value: true, configurable: true });
    const diagnostic = vi.spyOn(process.stderr, "write").mockReturnValue(false);
    const r = runtime({
      ...config({ hold: task("hold", [gate]) }),
      heartbeat: { interval: "20ms" },
    });
    const pending = r.execute("process", { task: "hold" });
    void pending.catch(() => {});
    try {
      await new Promise((resolve) => setTimeout(resolve, 150));
      expect(diagnostic).not.toHaveBeenCalled();
      await writeFile(gate, "release");
      const result = await pending;
      expect(result.details).toMatchObject({ stderr: { bytes: 0 } });
    } finally {
      if (descriptor) Object.defineProperty(process.stderr, "writableNeedDrain", descriptor);
      else Reflect.deleteProperty(process.stderr, "writableNeedDrain");
    }
  });
  it("classifies native spawn failure and clears diagnostics timers", async () => {
    const executable = path.join(root, process.platform === "win32" ? "bad.exe" : "bad");
    await writeFile(
      executable,
      Buffer.from(process.platform === "win32" ? "4d5a0000" : "7f454c46", "hex"),
      { mode: 0o755 },
    );
    const diagnostic = vi.spyOn(process.stderr, "write").mockReturnValue(true);
    const r = runtime({
      ...config({ bad: { command: [executable], inputs: [], outputs: [] } }),
      heartbeat: { interval: "20ms" },
    });
    await expect(r.execute("process", { task: "bad" })).rejects.toMatchObject({
      code: "TOOL_EXECUTION_FAILURE",
    });
    const count = diagnostic.mock.calls.length;
    await new Promise((resolve) => setTimeout(resolve, 60));
    expect(diagnostic.mock.calls).toHaveLength(count);
    expect(r.operations()[0]?.state).toBe("failed");
  });
  it("marks executor timeout timed_out while excluding queue wait", async () => {
    const marker = path.join(root, "first.pid");
    const inv = invocation({
      ...config({ first: task("stubborn", [marker]), second: task("inspect") }),
      timeout: "1s",
      killGrace: "1s",
    });
    const a = scope(inv, "a"),
      b = scope(inv, "b");
    const first = a.execute("process", { task: "first" });
    void first.catch(() => {});
    await vi.waitFor(async () => expect(await readFile(marker, "utf8")).toMatch(/^\d+$/));
    const second = b.execute("process", { task: "second" });
    void second.catch(() => {});
    await expect(first).rejects.toMatchObject({ code: "TIMEOUT" });
    await second;
    expect(inv.operations().map((o) => o.state)).toEqual(["timed_out", "completed"]);
    const queued = inv.operations()[1];
    expect((queued?.startedAt ?? 0) - (queued?.createdAt ?? 0)).toBeGreaterThan(1000);
    expect(await alive(Number(await readFile(marker, "utf8")))).toBe(false);
  }, 20000);

  it("owner dispose with zero abort grace waits for tree settlement and preserves another owner", async () => {
    const idsPath = path.join(root, "ids"),
      otherPid = path.join(root, "other");
    const inv = invocation({
      ...config({ tree: task("deep-tree", [idsPath]), other: task("stubborn", [otherPid]) }, 2),
      killGrace: "150ms",
    });
    const a = inv.createWorkerScope({
      root,
      symlinkPolicy: "deny",
      ownerId: "a",
      names: ["process"],
      maxToolCalls: 100,
      onLimit: vi.fn(),
      abortGraceMs: 0,
    });
    const b = scope(inv, "b");
    const first = a.execute("process", { task: "tree" });
    const second = b.execute("process", { task: "other" });
    void first.catch(() => {});
    void second.catch(() => {});
    let ids!: Record<string, number>;
    await vi.waitFor(
      async () => {
        ids = JSON.parse(await readFile(idsPath, "utf8"));
        expect(await readFile(otherPid, "utf8")).toMatch(/^\d+$/);
      },
      { timeout: 5000 },
    );
    await a.dispose();
    for (const pid of Object.values(ids)) expect(await alive(pid)).toBe(false);
    expect(a.operations()[0]).toMatchObject({ state: "cancelled", settlement: "settled" });
    expect(b.operations()[0]?.state).toBe("running");
    expect(await alive(Number(await readFile(otherPid, "utf8")))).toBe(true);
    await inv.dispose();
    await expect(first).rejects.toBeDefined();
    await expect(second).rejects.toBeDefined();
    expect(await alive(Number(await readFile(otherPid, "utf8")))).toBe(false);
  }, 20000);

  it("keeps process pool capacity through cleanup and preserves outer abort first cause", async () => {
    const pidPath = path.join(root, "pid"),
      marker = path.join(root, "marker");
    const inv = invocation({
      ...config({ first: task("stubborn", [pidPath]), next: task("touch", [marker]) }),
      timeout: "1s",
      killGrace: "400ms",
    });
    const a = scope(inv, "a"),
      b = scope(inv, "b");
    const controller = new AbortController(),
      reason = new Error("outer wins");
    const first = a.execute("process", { task: "first" }, { signal: controller.signal });
    void first.catch(() => {});
    await vi.waitFor(async () => expect(await readFile(pidPath, "utf8")).toMatch(/^\d+$/));
    const next = b.execute("process", { task: "next" });
    void next.catch(() => {});
    controller.abort(reason);
    await new Promise((resolve) => setTimeout(resolve, 100));
    expect(inv.operations().map((o) => o.state)).toEqual(["running", "queued"]);
    await expect(access(marker)).rejects.toMatchObject({ code: "ENOENT" });
    await expect(first).rejects.toBe(reason);
    await next;
    expect(inv.operations().map((o) => o.state)).toEqual(["cancelled", "completed"]);
  }, 20000);
  it.each([31, 98304])(
    "uses the policy combined output cap %i and validates the result",
    async (cap) => {
      const r = runtime({ ...config({ output: task("output") }), maxOutputBytes: cap });
      const result = await r.execute("process", { task: "output" });
      const details = result.details as unknown as {
        stdout: { bytes: number; capturedBytes: number; text: string };
        stderr: { bytes: number; capturedBytes: number; text: string };
      };
      expect(details.stdout.bytes + details.stderr.bytes).toBe(262144);
      expect(details.stdout.capturedBytes + details.stderr.capturedBytes).toBe(cap);
      expect(
        Buffer.byteLength(details.stdout.text) + Buffer.byteLength(details.stderr.text),
      ).toBeLessThanOrEqual(cap);
      expect(
        compileSchema(r.catalog.get("process")!.outputSchema, "output").validate(result).ok,
      ).toBe(true);
    },
  );

  it("emits heartbeat only to runtime diagnostics and stops after settle", async () => {
    const gate = path.join(root, "gate");
    const diagnostic = vi.spyOn(process.stderr, "write").mockReturnValue(true);
    const r = runtime({
      ...config({ hold: task("hold", [gate]) }),
      heartbeat: { interval: "30ms" },
    });
    const pending = r.execute("process", { task: "hold" });
    void pending.catch(() => {});
    await vi.waitFor(() => expect(diagnostic).toHaveBeenCalled());
    await writeFile(gate, "release");
    const result = await pending;
    expect(result.details).toMatchObject({
      stdout: { text: "released\n" },
      stderr: { bytes: 0, text: "" },
    });
    expect(JSON.stringify(result)).not.toContain("heartbeat");
    expect(r.operations()).toHaveLength(1);
    const count = diagnostic.mock.calls.length;
    await new Promise((resolve) => setTimeout(resolve, 80));
    expect(diagnostic.mock.calls).toHaveLength(count);
  });

  it.each(["exit-tree", "fail-tree", "deep-tree", "deep-stubborn"])(
    "cleans live descendants on %s",
    async (mode) => {
      const pidsPath = path.join(root, "pids.json");
      const tool = createProcessTool({
        root,
        symlinkPolicy: "deny",
        process: resolveProcessPolicy({
          ...config({ tree: task(mode, [pidsPath], ["pids.json"]) }),
          timeout: "2s",
          killGrace: "100ms",
        }),
      });
      const controller = new AbortController();
      const reason = new Error("outer tree cancellation");
      const pending = tool.execute({ task: "tree" }, controller.signal);
      void pending.catch(() => {});
      let pids!: Record<string, number>;
      await vi.waitFor(
        async () => {
          pids = JSON.parse(await readFile(pidsPath, "utf8"));
        },
        { timeout: 5000 },
      );
      if (mode === "deep-tree") controller.abort(reason);
      if (mode === "exit-tree") await pending;
      else if (mode === "deep-tree") await expect(pending).rejects.toBe(reason);
      else if (mode === "deep-stubborn")
        await expect(pending).rejects.toMatchObject({ code: "TIMEOUT" });
      else
        await expect(pending).rejects.toMatchObject({
          code: "TOOL_EXECUTION_FAILURE",
          details: { exitCode: 7 },
        });
      for (const pid of Object.values(pids)) expect(await alive(pid)).toBe(false);
    },
    15000,
  );

  it("times out a native process after spawn and cleans its handles", async () => {
    const pidsPath = path.join(root, "pid");
    const tool = createProcessTool({
      root,
      symlinkPolicy: "deny",
      process: resolveProcessPolicy({
        ...config({ stubborn: task("stubborn", [pidsPath], ["pid"]) }),
        timeout: "500ms",
        killGrace: "100ms",
      }),
    });
    await expect(tool.execute({ task: "stubborn" })).rejects.toMatchObject({ code: "TIMEOUT" });
    expect(await alive(Number(await readFile(pidsPath, "utf8")))).toBe(false);
  }, 15000);

  it.skipIf(process.platform === "win32")(
    "allows SIGTERM exit within grace before force kill",
    async () => {
      const pidsPath = path.join(root, "pid");
      const tool = createProcessTool({
        root,
        symlinkPolicy: "deny",
        process: resolveProcessPolicy({
          ...config({ graceful: task("graceful", [pidsPath], ["pid"]) }),
          timeout: "500ms",
          killGrace: "1s",
        }),
      });
      await expect(tool.execute({ task: "graceful" })).rejects.toMatchObject({ code: "TIMEOUT" });
      expect(await readFile(`${pidsPath}.terminated`, "utf8")).toBe("graceful");
      expect(await alive(Number(await readFile(pidsPath, "utf8")))).toBe(false);
    },
  );
});
const task = (mode: string, args: string[] = [], outputs: string[] = []) => ({
  command: ["node", script, mode, ...args],
  inputs: [],
  outputs,
});
const config = (tasks: ProcessPolicy["tasks"], maxConcurrent = 1) => ({ tasks, maxConcurrent });
function runtime(policy: ProcessPolicy, extra: Partial<api.ToolRuntimeOptions> = {}) {
  const result = api.createToolRuntime({
    root,
    symlinkPolicy: "deny",
    names: ["process"],
    maxToolCalls: 100,
    onLimit: vi.fn(),
    process: policy,
    ...extra,
  });
  runtimes.push(result);
  return result;
}
function invocation(policy: ProcessPolicy, maxOperations = 8) {
  const result = api.createInvocationToolRuntime({
    root,
    symlinkPolicy: "deny",
    process: policy,
    concurrency: { maxOperations, mutations: "serialized" },
  });
  runtimes.push(result);
  return result;
}
const scope = (inv: api.InvocationToolRuntime, ownerId: string) =>
  inv.createWorkerScope({
    root,
    symlinkPolicy: "deny",
    ownerId,
    names: ["process"],
    maxToolCalls: 100,
    onLimit: vi.fn(),
  });
async function alive(pid: number): Promise<boolean> {
  try {
    process.kill(pid, 0);
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ESRCH") return false;
    throw error;
  }
  if (process.platform === "linux") {
    try {
      const status = await readFile(`/proc/${pid}/stat`, "utf8");
      if (/^\d+ \(.*\) [ZX] /.test(status)) return false;
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code === "ENOENT") return false;
      throw error;
    }
  }
  return true;
}

describe("canonical managed process", () => {
  it("executes literal argv in a validated cwd with minimal env and ignored stdin", async () => {
    vi.stubEnv("CHEAPER_PROCESS_TEST_SECRET", "do-not-inherit");
    await mkdir(path.join(root, "sub"));
    const policy = config({
      inspect: {
        ...task("inspect", ["literal space", "*.txt"]),
        cwd: "sub",
        env: { ONLY_THIS: "yes" },
      },
    });
    const r = runtime(policy);
    policy.tasks.inspect!.command[2] = "fail";
    const result = await r.execute("process", { task: "inspect" });
    const details = result.details as unknown as {
      state: string;
      exitCode: number;
      stdout: { text: string };
      stderr: { bytes: number };
    };
    expect(details).toMatchObject({ state: "completed", exitCode: 0, stderr: { bytes: 0 } });
    expect(JSON.parse(details.stdout.text)).toEqual({
      cwd: path.join(root, "sub"),
      args: ["literal space", "*.txt"],
      explicit: "yes",
      secret: null,
      path: process.platform === "win32" ? "" : null,
      stdin: "",
    });
    expect(
      compileSchema(r.catalog.get("process")!.outputSchema, "output").validate(result).ok,
    ).toBe(true);
    expect(r.operations().map((o) => o.state)).toEqual(["completed"]);
    expect(r.budget.toolCalls).toBe(1);
  });

  it("rejects arbitrary command input and unknown tasks before admission", async () => {
    const r = runtime(config({ check: task("inspect") }));
    await expect(r.execute("process", { task: "check", command: ["node"] })).rejects.toMatchObject({
      code: "INVALID_INPUT",
    });
    await expect(r.execute("process", { task: "missing" })).rejects.toMatchObject({
      code: "POLICY_VIOLATION",
    });
    expect(r.budget.toolCalls).toBe(0);
    expect(r.operations()).toHaveLength(0);
    expect(() => runtime(config({}))).toThrow(
      expect.objectContaining({ code: "POLICY_VIOLATION" }),
    );
  });

  it("caps combined stdout/stderr and UTF-8 text while draining all bytes", async () => {
    const r = runtime(config({ output: task("output") }));
    const result = await r.execute("process", { task: "output" });
    const details = result.details as unknown as {
      stdout: { bytes: number; capturedBytes: number; text: string; truncated: boolean };
      stderr: { bytes: number; capturedBytes: number; text: string; truncated: boolean };
    };
    expect(details.stdout.bytes).toBe(131072);
    expect(details.stderr.bytes).toBe(131072);
    expect(details.stdout.capturedBytes + details.stderr.capturedBytes).toBeLessThanOrEqual(65536);
    expect(
      Buffer.byteLength(details.stdout.text) + Buffer.byteLength(details.stderr.text),
    ).toBeLessThanOrEqual(65536);
    expect(details.stdout.truncated || details.stderr.truncated).toBe(true);
  });

  it("fails a nonzero exit with bounded structured diagnostics", async () => {
    const r = runtime(config({ fail: task("fail") }));
    await expect(r.execute("process", { task: "fail" })).rejects.toMatchObject({
      code: "TOOL_EXECUTION_FAILURE",
      details: {
        task: "fail",
        state: "failed",
        exitCode: 7,
        stdout: { text: "before failure\n" },
        stderr: { text: "failure detail\n" },
      },
    });
    expect(r.operations().map((o) => o.state)).toEqual(["failed"]);
    expect(r.budget.toolCalls).toBe(1);
  });

  it("validates cwd existence, declared deny paths, output ownership and writeScope before spawn", async () => {
    const marker = path.join(root, "out");
    const cases = [
      [{ ...task("touch", [marker], ["out"]), cwd: "missing" }, {}],
      [
        task("touch", [marker], [".cheaper/out"]),
        { denylist: createPathDenylist([".cheaper/**"]) },
      ],
      [task("touch", [marker], ["out/*.json"]), {}],
      [task("touch", [marker], ["out"]), { writeScope: new Set(["allowed"]) }],
    ] as const;
    for (const [configured, extra] of cases) {
      const r = runtime(config({ check: configured }), extra);
      await expect(r.execute("process", { task: "check" })).rejects.toBeDefined();
      await expect(access(marker)).rejects.toMatchObject({ code: "ENOENT" });
    }
    const r = runtime(config({ check: task("touch", [marker], ["out"]) }), {
      writeScope: new Set(["out"]),
    });
    await r.execute("process", { task: "check" });
    expect(await readFile(marker, "utf8")).toBe("generated");
  });
});

describe("invocation process pool and lifecycle", () => {
  it("pins a queued task input to its admitted resource declaration", async () => {
    const gate = path.join(root, "release"),
      marker = path.join(root, "changed");
    const inv = invocation(
      config({
        hold: task("hold", [gate]),
        inspect: task("inspect"),
        mutate: task("touch", [marker], ["changed"]),
      }),
    );
    const a = scope(inv, "a"),
      b = scope(inv, "b");
    const first = a.execute("process", { task: "hold" });
    const input = { task: "inspect" };
    const second = b.execute("process", input);
    await vi.waitFor(() =>
      expect(inv.operations().map((o) => o.state)).toEqual(["running", "queued"]),
    );
    input.task = "mutate";
    await writeFile(gate, "release");
    await first;
    expect((await second).details).toMatchObject({ task: "inspect" });
    await expect(access(marker)).rejects.toMatchObject({ code: "ENOENT" });
  });
  it.each([
    [1, 8, 1],
    [3, 2, 2],
    [3, 8, 3],
  ])(
    "shares pool=%i and global=%i across owners",
    async (maxConcurrent, maxOperations, admitted) => {
      const gate = path.join(root, "release");
      const inv = invocation(config({ hold: task("hold", [gate]) }, maxConcurrent), maxOperations);
      const a = scope(inv, "a"),
        b = scope(inv, "b");
      const promises = [
        a.execute("process", { task: "hold" }),
        b.execute("process", { task: "hold" }),
        a.execute("process", { task: "hold" }),
      ];
      for (const p of promises) void p.catch(() => {});
      await vi.waitFor(() =>
        expect(inv.operations().filter((o) => o.state === "running")).toHaveLength(admitted),
      );
      expect(inv.operations().filter((o) => o.state === "queued")).toHaveLength(3 - admitted);
      await writeFile(gate, "release");
      await Promise.all(promises);
      expect(inv.operations().every((o) => o.state === "completed")).toBe(true);
      expect(a.budget.toolCalls + b.budget.toolCalls).toBe(3);
    },
    20000,
  );

  it("keeps pool capacity FIFO and releases queued abort with the original reason", async () => {
    const gate = path.join(root, "release");
    const inv = invocation(config({ hold: task("hold", [gate]) }));
    const a = scope(inv, "a");
    let listed = 0;
    const b = inv.createWorkerScope({
      root,
      symlinkPolicy: "deny",
      ownerId: "b",
      names: ["process", "ls"],
      maxToolCalls: 100,
      onLimit: vi.fn(),
      handlers: {
        ls: async () => {
          listed++;
          return { content: [{ type: "text", text: "listed" }] };
        },
      },
    });
    const first = a.execute("process", { task: "hold" });
    void first.catch(() => {});
    const controller = new AbortController();
    const queued = b.execute("process", { task: "hold" }, { signal: controller.signal });
    void queued.catch(() => {});
    const list = b.execute("ls", {});
    // resource conflict の無い ls は pool 飽和の process を追い越して進む（#61 ordering）。
    await vi.waitFor(() => {
      const states = inv.operations().map((o) => o.state);
      expect(states.slice(0, 2)).toEqual(["running", "queued"]);
      expect(states[2]).not.toBe("queued");
      expect(listed).toBe(1);
    });
    const reason = new Error("queued cancellation");
    controller.abort(reason);
    await expect(queued).rejects.toBe(reason);
    await list;
    expect(listed).toBe(1);
    // pool slot が空いた後の process は提出順で admission される。
    const second = b.execute("process", { task: "hold" });
    void second.catch(() => {});
    const third = a.execute("process", { task: "hold" });
    void third.catch(() => {});
    await vi.waitFor(() =>
      expect(inv.operations().map((o) => o.state)).toEqual([
        "running",
        "cancelled",
        "completed",
        "queued",
        "queued",
      ]),
    );
    await writeFile(gate, "release");
    await Promise.all([first, second, third]);
    expect(inv.operations().map((o) => o.state)).toEqual([
      "completed",
      "cancelled",
      "completed",
      "completed",
      "completed",
    ]);
  });

  it("serializes an output task against reads even with spare pool capacity", async () => {
    const gate = path.join(root, "release"),
      marker = path.join(root, "out");
    const inv = invocation(
      config({ hold: task("hold", [gate]), write: task("touch", [marker], ["out"]) }, 4),
    );
    const a = scope(inv, "a"),
      b = scope(inv, "b");
    const first = a.execute("process", { task: "hold" });
    void first.catch(() => {});
    const second = b.execute("process", { task: "write" });
    void second.catch(() => {});
    await vi.waitFor(() =>
      expect(inv.operations().map((o) => o.state)).toEqual(["running", "queued"]),
    );
    await expect(access(marker)).rejects.toMatchObject({ code: "ENOENT" });
    await writeFile(gate, "release");
    await Promise.all([first, second]);
  });

  it("inherits invocation policy but rejects an explicit scope policy mismatch", () => {
    const policy = config({ inspect: task("inspect") });
    const inv = invocation(policy);
    expect(scope(inv, "inherited").catalog.get("process")).toBeDefined();
    expect(() =>
      inv.createWorkerScope({
        root,
        symlinkPolicy: "deny",
        ownerId: "mismatch",
        names: ["process"],
        maxToolCalls: 10,
        onLimit: vi.fn(),
        process: { ...policy, maxConcurrent: 100 },
      }),
    ).toThrow(expect.objectContaining({ code: "INVALID_REQUEST" }));
  });

  it("kills the managed parent and descendant before a running cancellation settles", async () => {
    const pidsPath = path.join(root, "pids.json");
    const r = runtime(config({ tree: task("tree", [pidsPath], ["pids.json"]) }));
    const controller = new AbortController();
    const reason = new Error("stop managed tree");
    const running = r.execute("process", { task: "tree" }, { signal: controller.signal });
    void running.catch(() => {});
    let pids!: { parent: number; child: number };
    await vi.waitFor(
      async () => {
        pids = JSON.parse(await readFile(pidsPath, "utf8"));
        expect(await alive(pids.child)).toBe(true);
      },
      { timeout: 5000 },
    );
    controller.abort(reason);
    await expect(running).rejects.toBe(reason);
    await expect.poll(() => alive(pids.parent), { timeout: 3000 }).toBe(false);
    await expect.poll(() => alive(pids.child), { timeout: 3000 }).toBe(false);
    expect(r.operations().map((o) => o.state)).toEqual(["cancelled"]);
  }, 15000);
});
