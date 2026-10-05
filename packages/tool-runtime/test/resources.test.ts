import { mkdtemp, rm, symlink, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";
import * as api from "../src/index.js";

const result: api.CanonicalToolResult = { content: [{ type: "text", text: "ok" }] };
function gate() {
  let resolve!: () => void;
  const promise = new Promise<void>((yes) => {
    resolve = yes;
  });
  return { promise, resolve };
}
const disposals: (() => Promise<void>)[] = [];
afterEach(async () => {
  for (const dispose of disposals.splice(0).reverse()) await dispose();
});
async function fixture() {
  const root = await mkdtemp(path.join(os.tmpdir(), "cheaper-leases-"));
  await Promise.all(["a", "b", "c"].map((p) => writeFile(path.join(root, p), "x")));
  const invocation = api.createInvocationToolRuntime({
    root,
    symlinkPolicy: "within-workspace",
    abortGraceMs: 0,
    concurrency: { maxOperations: 2, mutations: "per-resource" },
  });
  const pending: ReturnType<typeof gate>[] = [];
  const started: string[] = [];
  const scope = (ownerId: string, extra: Partial<api.WorkerScopeOptions> = {}) =>
    invocation.createWorkerScope({
      root,
      symlinkPolicy: "within-workspace",
      names: ["write", "edit", "ls"],
      maxToolCalls: 100,
      onLimit: vi.fn(),
      ownerId,
      resourceManifest: {
        reads: [{ path: ".", scope: "subtree" }],
        writes: [{ path: ".", scope: "subtree" }],
        produces: [],
      },
      handlers: Object.fromEntries(
        ["write", "edit", "ls"].map((name) => [
          name,
          async (input: unknown) => {
            started.push(`${ownerId}:${name}:${(input as { path: string }).path}`);
            const g = gate();
            pending.push(g);
            await g.promise;
            return result;
          },
        ]),
      ),
      ...extra,
    });
  disposals.push(async () => {
    pending.forEach((g) => {
      g.resolve();
    });
    await invocation.dispose();
    await rm(root, { recursive: true, force: true });
  });
  return { root, invocation, scope, started, pending };
}
describe("workflow caller-owned parent lifetime", () => {
  it.each([false, true])(
    "retains the parent through invocation dispose (child token: %s)",
    async (childToken) => {
      const f = await fixture();
      const parentSpy = vi.spyOn(api.ResourceManager.prototype, "parent");
      try {
        const parent = await f.invocation.acquireJobLease({
          ownerId: "holder",
          names: ["write"],
          resourceManifest: narrow("a"),
          readStrictness: "reject",
        });
        const child = f.scope("holder", {
          parentLease: parent,
          resourceManifest: narrow("a"),
          readStrictness: "reject",
        });
        const manager = parentSpy.mock.contexts[0] as api.ResourceManager | undefined;
        if (!manager) throw new Error("missing resource manager");
        const prepared = manager.parent(parent, "holder");
        const operation = childToken ? write(child, "a").catch((error) => error) : undefined;
        if (childToken) await vi.waitFor(() => expect(f.started).toHaveLength(1));
        await f.invocation.dispose();
        expect(() => manager.parent(parent, "holder")).not.toThrow();
        let started = false;
        const contender = manager.request({
          ownerId: "contender",
          kind: "job",
          prepare: () => prepared,
          start: (lease) => {
            started = true;
            return lease;
          },
        });
        await Promise.resolve();
        expect(started).toBe(false);
        f.invocation.releaseJobLease(parent);
        if (childToken) {
          expect(started).toBe(false);
          f.pending[0]?.resolve();
          await operation;
        }
        const contenderLease = await contender;
        expect(started).toBe(true);
        manager.release(contenderLease);
      } finally {
        parentSpy.mockRestore();
      }
    },
  );
  it("narrows mkdir safety ancestors to canonical literal exact write claims", async () => {
    const f = await fixture();
    const safety = {
      reads: [{ path: "a", scope: "exact" as const }],
      writes: [{ path: ".", scope: "subtree" as const }],
      produces: [{ path: "a", scope: "exact" as const }],
    };
    const access = new api.ResourceAccess({
      root: f.root,
      symlinkPolicy: "within-workspace",
      names: ["write"],
      writeScope: new Set(["a"]),
      readStrictness: "reject",
    });
    const prepared = await access.prepare();
    const manifest = await access.prepareWorkerManifest(safety);
    expect(manifest.writes).toEqual(prepared.writeClaims);
    expect(manifest.produces).toEqual(prepared.writeClaims);
    expect(manifest.reads).toEqual([
      {
        resource: { kind: "workspace", path: path.join(f.root, "a"), scope: "exact" },
        mode: "shared",
      },
    ]);
    const lease = await f.invocation.acquireJobLease({
      ownerId: "narrow",
      names: ["write"],
      resourceManifest: safety,
      readStrictness: "reject",
    });
    const child = f.scope("narrow", {
      parentLease: lease,
      resourceManifest: manifest,
      writeScope: new Set(["a"]),
      readStrictness: "reject",
    });
    await child.ready();
    await expect(write(child, "b")).rejects.toThrow("outside the write scope");
    expect(f.started).toEqual([]);
    await child.dispose();
    f.invocation.releaseJobLease(lease);
  });
  it("preserves safety without writeScope and removes permissions for explicit empty scope", async () => {
    const f = await fixture();
    const safety = narrow("a");
    const base = {
      root: f.root,
      symlinkPolicy: "within-workspace" as const,
      names: ["write"] as const,
    };
    const unrestricted = new api.ResourceAccess(base);
    const fallback = await unrestricted.prepareWorkerManifest();
    expect(fallback).toEqual((await unrestricted.prepare()).manifest);
    expect(fallback.writes).toEqual([
      { resource: { kind: "workspace", path: f.root, scope: "subtree" }, mode: "exclusive" },
    ]);
    expect(await unrestricted.prepareWorkerManifest(safety)).toEqual(
      (await new api.ResourceAccess({ ...base, resourceManifest: safety }).prepare()).manifest,
    );
    const empty = await new api.ResourceAccess({
      ...base,
      writeScope: new Set(),
    }).prepareWorkerManifest(safety);
    expect(empty.writes).toEqual([]);
    expect(empty.produces).toEqual([]);
    expect(empty.reads).toHaveLength(1);
  });
  it("retains literal metacharacters and intersects each safety bucket separately", async () => {
    const f = await fixture();
    await writeFile(path.join(f.root, "literal[1].txt"), "x");
    const access = new api.ResourceAccess({
      root: f.root,
      symlinkPolicy: "within-workspace",
      names: ["write"],
      writeScope: new Set(["literal[1].txt", "a", "b"]),
    });
    const manifest = await access.prepareWorkerManifest({
      reads: [],
      writes: [{ path: ".", scope: "subtree" }],
      produces: [{ path: "a", scope: "exact" }],
    });
    expect(manifest.writes.map((claim) => claim.resource)).toContainEqual({
      kind: "workspace",
      path: path.join(f.root, "literal[1].txt"),
      scope: "exact",
    });
    expect(manifest.produces).toHaveLength(1);
    expect(manifest.produces[0]?.resource).toMatchObject({
      path: path.join(f.root, "a"),
      scope: "exact",
    });
    expect(manifest.reads).toEqual([]);
  });
  it("continues releasing leases acquired by scope.ready itself", async () => {
    const f = await fixture();
    const requestSpy = vi.spyOn(api.ResourceManager.prototype, "request");
    try {
      const child = f.scope("self", { resourceManifest: narrow("a"), readStrictness: "reject" });
      const lease = await child.ready();
      const manager = requestSpy.mock.contexts[0] as api.ResourceManager | undefined;
      if (!manager) throw new Error("missing resource manager");
      expect(() => manager.parent(lease, "self")).not.toThrow();
      await f.invocation.dispose();
      expect(() => manager.parent(lease, "self")).toThrow("released or closing");
    } finally {
      requestSpy.mockRestore();
    }
  });
});

const write = (s: api.ToolRuntime, p: string, signal?: AbortSignal) =>
  s.execute("write", { path: p, content: "y" }, { signal });
const narrow = (p: string) => ({
  reads: [{ path: p, scope: "exact" as const }],
  writes: [{ path: p, scope: "exact" as const }],
  produces: [],
});

describe("resource lease contracts", () => {
  it("pumps nested synchronous shared requests without unrelated settle events", async () => {
    const f = await fixture(),
      manager = new api.OperationManager({ abortGraceMs: 0 }),
      scheduler = new api.ToolScheduler(
        manager,
        { maxOperations: 2 },
        new api.ResourceManager(f.root, "deny"),
      );
    const p = scheduler.run({ ownerId: "outer", kind: "tool" }, { access: "shared" }, () =>
      scheduler.run({ ownerId: "inner", kind: "tool" }, { access: "shared" }, async () => result),
    );
    await expect(p).resolves.toEqual(result);
    expect(manager.records().map((o) => o.state)).toEqual(["completed", "completed"]);
    await manager.dispose();
  });
  it("records resource preparation failure as queued to cancelled with the original reason", async () => {
    const manager = new api.OperationManager(),
      scheduler = new api.ToolScheduler(manager),
      reason = new Error("invalid declaration");
    await expect(
      scheduler.run(
        { ownerId: "owner", kind: "process" },
        {
          access: "exclusive",
          prepare: () => {
            throw reason;
          },
        },
        async () => result,
      ),
    ).rejects.toBe(reason);
    const record = manager.records()[0];
    expect(record?.state).toBe("cancelled");
    expect(record?.startedAt).toBeUndefined();
    expect(record?.signal.reason).toBe(reason);
    await manager.dispose();
  });
  it("holds root exclusive for a mutating job without manifest or writeScope", async () => {
    const f = await fixture(),
      a = f.scope("a", { resourceManifest: undefined });
    await a.ready();
    const b = f.scope("b", { resourceManifest: narrow("b"), readStrictness: "reject" });
    let admitted = false;
    const p = b.ready().then(() => {
      admitted = true;
    });
    await new Promise((resolve) => setTimeout(resolve, 30));
    expect(admitted).toBe(false);
    await a.dispose();
    await p;
    expect(admitted).toBe(true);
  });
  it("preserves the workspace-denied message and rejected-call accounting during preparation", async () => {
    const f = await fixture(),
      a = f.scope("a");
    await expect(write(a, "../outside.txt")).rejects.toThrow(api.ACCESS_DENIED_MESSAGE);
    expect(a.budget.rejectedToolCalls).toBe(1);
    expect(f.started).toEqual([]);
  });
  it("canonicalizes a symlink workspace root before global fallback conflicts", async () => {
    const f = await fixture();
    const alias = path.join(f.root, "root-alias");
    await symlink(f.root, alias, process.platform === "win32" ? "junction" : "dir");
    const invocation = api.createInvocationToolRuntime({
      root: alias,
      symlinkPolicy: "within-workspace",
      abortGraceMs: 0,
      concurrency: { maxOperations: 2, mutations: "per-resource" },
    });
    const blocked = gate();
    disposals.push(async () => {
      blocked.resolve();
      await invocation.dispose();
    });
    const legacy = invocation.createWorkerScope({
      root: alias,
      symlinkPolicy: "within-workspace",
      ownerId: "legacy",
      names: ["write"],
      maxToolCalls: 10,
      onLimit: vi.fn(),
      handlers: {
        write: async () => {
          await blocked.promise;
          return result;
        },
      },
    });
    const p = write(legacy, "a");
    let admitted = false;
    const ready = invocation
      .acquireJobLease({
        ownerId: "job",
        names: ["write"],
        resourceManifest: narrow("a"),
        readStrictness: "reject",
      })
      .then((lease) => {
        admitted = true;
        return lease;
      });
    await new Promise((resolve) => setTimeout(resolve, 30));
    expect(admitted).toBe(false);
    blocked.resolve();
    await p;
    const lease = await ready;
    invocation.releaseJobLease(lease);
  });
  it("preserves same-parent sibling FIFO while bypassing external waiters", async () => {
    const f = await fixture(),
      a = f.scope("a", {
        readStrictness: "reject",
        resourceManifest: {
          reads: [{ path: "a", scope: "subtree" }],
          writes: [{ path: "a", scope: "subtree" }],
          produces: [],
        },
      });
    await a.ready();
    const p = a.execute("ls", { path: "a" });
    await vi.waitFor(() => expect(f.started).toHaveLength(1));
    const writer = write(a, "a"),
      tail = a.execute("ls", { path: "a" });
    await new Promise((resolve) => setTimeout(resolve, 30));
    expect(f.started).toEqual(["a:ls:a"]);
    f.pending[0]?.resolve();
    await p;
    await vi.waitFor(() => expect(f.started).toEqual(["a:ls:a", "a:write:a"]));
    f.pending[1]?.resolve();
    await writer;
    await vi.waitFor(() => expect(f.started).toHaveLength(3));
    f.pending[2]?.resolve();
    await tail;
  });
  it("unknown manifest writes hold a conservative root-exclusive job lease", async () => {
    const f = await fixture(),
      a = f.scope("a", {
        resourceManifest: { reads: [], writes: null, produces: [] },
        readStrictness: "reject",
      });
    await a.ready();
    const b = f.scope("b", { resourceManifest: narrow("b"), readStrictness: "reject" });
    let started = false;
    const ready = b.ready().then(() => {
      started = true;
    });
    await new Promise((resolve) => setTimeout(resolve, 30));
    expect(started).toBe(false);
    await a.dispose();
    await ready;
    expect(started).toBe(true);
  });
  it("fails fast when a borrowed operation meets another job token", async () => {
    const f = await fixture(),
      resources = new api.ResourceManager(f.root, "within-workspace");
    const preparation = {
      claims: [
        {
          resource: {
            kind: "workspace" as const,
            path: path.join(f.root, "a"),
            scope: "exact" as const,
          },
          mode: "exclusive" as const,
        },
      ],
      readStrictness: "reject" as const,
    };
    const parent = await resources.request({
      ownerId: "owner",
      kind: "job",
      prepare: () => preparation,
      start: (lease) => lease,
    });
    // external binding seam が二つ目の job token を作った状態でも子を queue で待たせない。
    const otherJob = await resources.request({
      ownerId: "owner",
      kind: "job",
      parentLease: parent,
      prepare: () => preparation,
      start: (lease) => lease,
    });
    await expect(
      resources.request({
        ownerId: "owner",
        kind: "operation",
        parentLease: parent,
        prepare: () => preparation,
        start: () => {
          throw new Error("must not start");
        },
      }),
    ).rejects.toMatchObject({
      code: "INVALID_REQUEST",
      message: expect.stringContaining("foreign job token"),
    });
    resources.release(otherJob);
    resources.release(parent);
  });
  it("queues calls made while ready is pending without consuming operation capacity", async () => {
    const f = await fixture(),
      parent = await f.invocation.acquireJobLease({
        ownerId: "parent",
        names: ["write"],
        resourceManifest: narrow("a"),
        readStrictness: "reject",
      });
    const a = f.scope("a", { resourceManifest: narrow("a"), readStrictness: "reject" });
    const ready = a.ready(),
      p = write(a, "a");
    expect(a.budget.toolCalls).toBe(1);
    expect(a.operations()[0]?.state).toBe("queued");
    const independent = write(f.scope("independent"), "b");
    await vi.waitFor(() => expect(f.started).toEqual(["independent:write:b"]));
    f.invocation.releaseJobLease(parent);
    await ready;
    await vi.waitFor(() => expect(f.started).toHaveLength(2));
    f.pending.forEach((g) => {
      g.resolve();
    });
    await Promise.all([p, independent]);
  });
  it("overlaps independent mutations but holds same-file read/write until settlement", async () => {
    const f = await fixture(),
      a = f.scope("a"),
      b = f.scope("b");
    const p = write(a, "a"),
      q = write(b, "b"),
      r = b.execute("ls", { path: "a" });
    await vi.waitFor(() => expect(f.started).toEqual(["a:write:a", "b:write:b"]));
    expect(b.operations()[1]?.state).toBe("queued");
    f.pending[0]?.resolve();
    await p;
    await vi.waitFor(() => expect(f.started).toHaveLength(3));
    f.pending.forEach((g) => {
      g.resolve();
    });
    await Promise.all([q, r]);
  });
  it("retains a job lease between operations and bypasses its blocked foreign job and operation waiters", async () => {
    const f = await fixture(),
      a = f.scope("a", {
        resourceManifest: narrow("a"),
        writeScope: new Set(["a"]),
        readStrictness: "reject",
      });
    await a.ready();
    const b = f.scope("b", {
      resourceManifest: narrow("a"),
      writeScope: new Set(["a"]),
      readStrictness: "reject",
    });
    let jobReady = false;
    const job = b.ready().then(() => {
      jobReady = true;
    });
    const c = f.scope("c");
    const foreign = write(c, "a");
    const child = write(a, "a");
    await vi.waitFor(() => expect(f.started).toEqual(["a:write:a"]));
    expect(jobReady).toBe(false);
    f.pending[0]?.resolve();
    await child;
    expect(jobReady).toBe(false);
    await a.dispose();
    await job;
    expect(f.started).toHaveLength(1);
    await b.dispose();
    await vi.waitFor(() => expect(f.started.at(-1)).toBe("c:write:a"));
    f.pending.at(-1)?.resolve();
    await foreign;
  });
  it("serializes record-only mutation jobs at ready and preserves the original queued abort reason", async () => {
    const f = await fixture(),
      a = f.scope("a", { resourceManifest: undefined, writeScope: new Set(["a"]) }),
      controller = new AbortController();
    await a.ready();
    const b = f.scope("b", {
      resourceManifest: undefined,
      writeScope: new Set(["b"]),
      signal: controller.signal,
    });
    const p = b.ready();
    const reason = new Error("stop waiting");
    controller.abort(reason);
    await expect(p).rejects.toBe(reason);
  });
  it("validates parent ownership and inherits readStrictness without allowing an override", async () => {
    const f = await fixture();
    const parent = await f.invocation.acquireJobLease({
      ownerId: "a",
      names: ["write"],
      resourceManifest: narrow("a"),
      readStrictness: "reject",
    });
    expect(() => f.scope("wrong", { parentLease: parent })).toThrow(/owner/);
    expect(() => f.scope("a", { parentLease: parent, readStrictness: "record" })).toThrow(
      /strictness/,
    );
    const child = f.scope("a", { parentLease: parent, resourceManifest: narrow("a") });
    await child.ready();
    await child.dispose();
    expect(() => f.scope("forged", { parentLease: {} as api.ResourceLease })).toThrow();
    f.invocation.releaseJobLease(parent);
  });
  it("canonicalizes aliases and directory overlap before admission", async () => {
    const f = await fixture();
    await symlink(
      f.root,
      path.join(f.root, "alias"),
      process.platform === "win32" ? "junction" : "dir",
    );
    const a = f.scope("a"),
      b = f.scope("b");
    const p = write(a, "a"),
      q = write(b, "alias/a");
    await vi.waitFor(() => expect(f.started).toEqual(["a:write:a"]));
    f.pending[0]?.resolve();
    await p;
    await vi.waitFor(() => expect(f.started).toHaveLength(2));
    f.pending[1]?.resolve();
    await q;
  });
  it("acquires reverse multi-claim jobs all-or-none and restores FIFO after parent release", async () => {
    const f = await fixture();
    const options = (ownerId: string, paths: string[]) => ({
      ownerId,
      names: ["write"] as const,
      readStrictness: "reject" as const,
      resourceManifest: {
        reads: [],
        writes: paths.map((p) => ({ path: p, scope: "exact" as const })),
        produces: [],
      },
    });
    const parent = await f.invocation.acquireJobLease(options("first", ["a"]));
    const order: string[] = [];
    const second = f.invocation.acquireJobLease(options("second", ["b", "a"])).then((lease) => {
      order.push("second");
      return lease;
    });
    const third = f.invocation.acquireJobLease(options("third", ["a", "b"])).then((lease) => {
      order.push("third");
      return lease;
    });
    // second は b を部分取得しない。独立した c は先行する。
    const independent = await f.invocation.acquireJobLease(options("other", ["c"]));
    expect(order).toEqual([]);
    f.invocation.releaseJobLease(independent);
    f.invocation.releaseJobLease(parent);
    const lease = await second;
    expect(order).toEqual(["second"]);
    f.invocation.releaseJobLease(lease);
    const last = await third;
    expect(order).toEqual(["second", "third"]);
    f.invocation.releaseJobLease(last);
  });
  it("retains capacity and the closing parent through late settlement after bounded detach", async () => {
    const f = await fixture(),
      a = f.scope("a", { resourceManifest: narrow("a"), readStrictness: "reject" });
    await a.ready();
    const p = write(a, "a");
    await vi.waitFor(() => expect(f.started).toHaveLength(1));
    await a.dispose();
    expect(a.operations()[0]?.settlement).toBe("unsettled");
    const b = f.scope("b", { resourceManifest: narrow("a"), readStrictness: "reject" });
    let admitted = false;
    const ready = b.ready().then(() => {
      admitted = true;
    });
    await new Promise((yes) => setTimeout(yes, 20));
    expect(admitted).toBe(false);
    f.pending[0]?.resolve();
    await p;
    await ready;
    expect(admitted).toBe(true);
  });
  it("snapshots submitted input, manifest and writeScope while waiting and reserves budget synchronously", async () => {
    const f = await fixture(),
      manifest = narrow("a"),
      targets = new Set(["a"]),
      controller = new AbortController();
    const lease = await f.invocation.acquireJobLease({
      ownerId: "parent",
      names: ["write"],
      resourceManifest: manifest,
      readStrictness: "reject",
    });
    const seen: string[] = [];
    const child = f.scope("child", {
      resourceManifest: manifest,
      writeScope: targets,
      signal: controller.signal,
      handlers: {
        write: async (args) => {
          seen.push((args as { path: string }).path);
          return result;
        },
      },
    });
    const input = { path: "a", content: "original" };
    const p = child.execute("write", input);
    expect(child.budget.toolCalls).toBe(1);
    input.path = "b";
    const target = manifest.writes[0];
    if (!target) throw new Error("missing fixture target");
    target.path = "b";
    targets.clear();
    targets.add("b");
    f.invocation.releaseJobLease(lease);
    await p;
    expect(seen).toEqual(["a"]);
  });
  it("holds process declared output resources for the complete returned promise", async () => {
    const f = await fixture(),
      manager = new api.OperationManager({ abortGraceMs: 0 }),
      resources = new api.ResourceManager(f.root, "within-workspace"),
      scheduler = new api.ToolScheduler(
        manager,
        { maxOperations: 2, mutations: "per-resource" },
        resources,
      ),
      blocked = gate();
    const p = scheduler.run(
      { ownerId: "process", kind: "process" },
      {
        access: "exclusive",
        resources: { reads: [], writes: [], produces: [{ path: "a", scope: "exact" }] },
      },
      async () => {
        await blocked.promise;
        return 1;
      },
    );
    let started = false;
    const q = scheduler.run(
      { ownerId: "read", kind: "tool" },
      {
        access: "shared",
        resources: { reads: [{ path: "a", scope: "exact" }], writes: [], produces: [] },
      },
      async () => {
        started = true;
        return 2;
      },
    );
    await vi.waitFor(() => expect(manager.records()[0]?.state).toBe("running"));
    expect(started).toBe(false);
    expect(manager.records()[1]?.state).toBe("queued");
    blocked.resolve();
    await p;
    await q;
    expect(started).toBe(true);
    await manager.dispose();
  });
});
