import { access, mkdir, mkdtemp, readFile, realpath, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { compileSchema, createPathDenylist, type ProcessPolicy } from "@cheaper/core";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as api from "../src/index.js";

const script = fileURLToPath(new URL("./fixtures/process-child.mjs", import.meta.url));
let root: string;
const runtimes: Array<{ dispose(): Promise<void> }> = [];
beforeEach(async () => {
  root = await realpath(await mkdtemp(path.join(process.cwd(), ".tmp-process-runtime-")));
});
afterEach(async () => {
  for (const runtime of runtimes.splice(0)) await runtime.dispose();
  vi.unstubAllEnvs();
  expect(path.dirname(root)).toBe(await realpath(process.cwd()));
  await rm(root, { recursive: true, force: true });
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
  );

  it("keeps FIFO across a saturated pool and releases queued abort with the original reason", async () => {
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
    await vi.waitFor(() =>
      expect(inv.operations().map((o) => o.state)).toEqual(["running", "queued", "queued"]),
    );
    expect(listed).toBe(0);
    const reason = new Error("queued cancellation");
    controller.abort(reason);
    await expect(queued).rejects.toBe(reason);
    await list;
    expect(listed).toBe(1);
    await writeFile(gate, "release");
    await first;
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
  });
});
