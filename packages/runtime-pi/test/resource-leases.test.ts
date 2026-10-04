import { mkdir, mkdtemp, readFile, realpath, rm, stat, symlink, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import {
  compileSchema,
  createPathDenylist,
  type RuntimePolicy,
  type WorkerDefinition,
} from "@cheaper/core";
import {
  createInvocationToolRuntime,
  createToolRuntime,
  MAX_ACCESS_OBSERVATIONS,
  MutationJournal,
  type ResourceScopeOptions,
} from "@cheaper/tool-runtime";
import { ModelRuntime } from "@earendil-works/pi-coding-agent";
import { afterEach, describe, expect, it, vi } from "vitest";
import { runWorker } from "../src/run.js";
import { createSandboxedTools } from "../src/tools/registry.js";
import { createFakePi, fauxAssistantMessage, fauxToolCall } from "./fixtures/fake-stream.js";

const cleanup: (() => Promise<void>)[] = [];
afterEach(async () => {
  for (const fn of cleanup.splice(0).reverse()) await fn();
});
function gate() {
  let resolve!: () => void;
  const promise = new Promise<void>((yes) => {
    resolve = yes;
  });
  return { promise, resolve };
}
async function workspace() {
  const dir = await mkdtemp(path.join(os.tmpdir(), "cheaper-observed-")),
    root = await realpath(dir);
  await Promise.all(["a", "b", "secret"].map((p) => writeFile(path.join(root, p), "original\n")));
  cleanup.push(() => rm(dir, { recursive: true, force: true }));
  return root;
}
async function sandbox(extra: ResourceScopeOptions = {}) {
  const root = await workspace(),
    journal = new MutationJournal();
  const tools = createSandboxedTools({
    root,
    symlinkPolicy: "deny",
    journal,
    names: ["read", "write", "edit", "ls", "find", "grep", "count_tokens"],
    maxToolCalls: 100,
    onLimit: vi.fn(),
    resourceManifest: {
      reads: [{ path: "a", scope: "exact" }],
      writes: [{ path: "a", scope: "exact" }],
      produces: [],
    },
    ...extra,
  });
  cleanup.push(() => tools.runtime.dispose());
  return { root, journal, ...tools };
}
describe("actual resource access and Worker leases", () => {
  it("skips symlink entries resolving to privacy-denied files and continues walking", async () => {
    const root = await workspace();
    await mkdir(path.join(root, "protected"));
    await writeFile(path.join(root, "protected", "nested"), "secret");
    await symlink(
      path.join(root, "protected"),
      path.join(root, "alias"),
      process.platform === "win32" ? "junction" : "dir",
    );
    const tools = createSandboxedTools({
      root,
      symlinkPolicy: "within-workspace",
      journal: new MutationJournal(),
      names: ["find"],
      maxToolCalls: 10,
      onLimit: vi.fn(),
      denylist: createPathDenylist(["secret", "protected/nested"]),
      resourceManifest: { reads: [{ path: ".", scope: "subtree" }], writes: [], produces: [] },
    });
    cleanup.push(() => tools.runtime.dispose());
    const output = await tools.runtime.execute("find", { path: ".", pattern: "**/*" });
    const text = output.content.map((c) => c.text).join("\n");
    expect(text).toContain("a");
    expect(text).toContain("b");
    expect(text).not.toMatch(/secret|nested|unreadable|Access denied/);
    expect(
      tools.runtime
        .accesses()
        .observations.every((row) => row.path !== "secret" && row.path !== "protected/nested"),
    ).toBe(true);
  });
  it("records actual reads and counts, with record-only undeclared reads and immutable snapshots", async () => {
    const s = await sandbox();
    await s.runtime.execute("read", { path: "a" });
    await s.runtime.execute("count_tokens", { path: "b" });
    const rows = s.runtime.accesses().observations;
    expect(rows).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ path: "a", declared: true, allowed: true }),
        expect.objectContaining({ path: "b", declared: false, allowed: true }),
      ]),
    );
    expect(rows.every((r) => r.allowed)).toBe(true);
    expect(rows.every((r) => r.operationId)).toBe(true);
    const first = rows[0];
    if (first) {
      (first as { path: string }).path = "changed";
      first.claim.resource.path = "changed";
    }
    expect(s.runtime.accesses().observations[0]?.path).toBe("a");
  });
  it("rejects strict undeclared reads before returning data and preserves accounting", async () => {
    const s = await sandbox({ readStrictness: "reject" });
    await expect(s.runtime.execute("read", { path: "b" })).rejects.toThrow("Access denied");
    expect(s.budget.rejectedToolCalls).toBe(1);
    expect(s.runtime.accesses().observations[0]).toMatchObject({
      path: "b",
      declared: false,
      allowed: false,
    });
  });
  it("rejects out-of-scope writes before mkdir and permits authorized ancestor mkdir with journal", async () => {
    const s = await sandbox({ resourceManifest: undefined, writeScope: new Set(["new/target"]) });
    await expect(
      s.runtime.execute("write", { path: "outside/target", content: "bad" }),
    ).rejects.toThrow("write scope");
    await expect(stat(path.join(s.root, "outside"))).rejects.toMatchObject({ code: "ENOENT" });
    await s.runtime.execute("write", { path: "new/target", content: "good" });
    expect(await readFile(path.join(s.root, "new/target"), "utf8")).toBe("good");
    expect((await s.journal.finalize()).files).toMatchObject([
      { path: "new/target", kind: "created" },
    ]);
    expect(s.runtime.accesses().observations.some((r) => r.support && r.allowed)).toBe(true);
  });
  it("keeps privacy-denied paths out of observations and rejects dishonest operation reads", async () => {
    const root = await workspace(),
      journal = new MutationJournal();
    const tools = createSandboxedTools({
      root,
      symlinkPolicy: "deny",
      journal,
      names: ["read", "write"],
      maxToolCalls: 10,
      onLimit: vi.fn(),
      denylist: createPathDenylist(["secret"]),
    });
    cleanup.push(() => tools.runtime.dispose());
    await expect(tools.runtime.execute("read", { path: "secret" })).rejects.toThrow("policy");
    expect(tools.runtime.accesses().observations).toEqual([]);
  });
  it("bounds retained access rows and still validates every access after the cap", async () => {
    const root = await workspace();
    await mkdir(path.join(root, "many"));
    await Promise.all(
      Array.from({ length: MAX_ACCESS_OBSERVATIONS + 8 }, (_, i) =>
        writeFile(path.join(root, "many", String(i)), "x"),
      ),
    );
    const tools = createSandboxedTools({
      root,
      symlinkPolicy: "deny",
      journal: new MutationJournal(),
      names: ["find", "read"],
      maxToolCalls: 10,
      onLimit: vi.fn(),
      resourceManifest: { reads: [{ path: "many", scope: "subtree" }], writes: [], produces: [] },
      readStrictness: "reject",
    });
    cleanup.push(() => tools.runtime.dispose());
    // 一致しない pattern でも実際の全 entry を調べる。
    await tools.runtime.execute("find", { path: "many", pattern: "never-matches" });
    const snapshot = tools.runtime.accesses();
    expect(snapshot.observations).toHaveLength(MAX_ACCESS_OBSERVATIONS);
    expect(snapshot.total).toBeGreaterThan(MAX_ACCESS_OBSERVATIONS);
    expect(snapshot.dropped).toBe(snapshot.total - MAX_ACCESS_OBSERVATIONS);
    await expect(tools.runtime.execute("read", { path: "b" })).rejects.toThrow("Access denied");
    expect(tools.runtime.accesses().observations).toHaveLength(MAX_ACCESS_OBSERVATIONS);
    expect(tools.runtime.accesses().total).toBeGreaterThan(snapshot.total);
  });
  it("rejects an actual read outside the admitted operation and rejects observations after settlement", async () => {
    const root = await workspace();
    let reachedIO = false,
      late: () => Promise<void> = async () => {};
    const runtime = createToolRuntime({
      root,
      symlinkPolicy: "deny",
      names: ["ls"],
      maxToolCalls: 10,
      onLimit: vi.fn(),
      resourceManifest: { reads: [{ path: ".", scope: "subtree" }], writes: [], produces: [] },
      handlers: {
        ls: async (args) => {
          if ((args as { path: string }).path === "a") {
            await runtime.observeAccess("b", "shared");
            reachedIO = true;
            await readFile(path.join(root, "b"));
          } else {
            const delayed = gate();
            const observation = delayed.promise.then(() => runtime.observeAccess("a", "shared"));
            void observation.catch(() => {});
            late = () => {
              delayed.resolve();
              return observation;
            };
          }
          return { content: [{ type: "text", text: "ok" }] };
        },
      },
    });
    cleanup.push(() => runtime.dispose());
    await expect(runtime.execute("ls", { path: "a" })).rejects.toThrow("Access denied");
    expect(reachedIO).toBe(false);
    await runtime.execute("ls", { path: "." });
    await expect(late()).rejects.toThrow("settled");
  });
  it("authorizes canonical writeScope through symlink aliases and preserves journal display paths", async () => {
    const root = await workspace();
    await symlink(
      root,
      path.join(root, "alias"),
      process.platform === "win32" ? "junction" : "dir",
    );
    const journal = new MutationJournal(),
      tools = createSandboxedTools({
        root,
        symlinkPolicy: "within-workspace",
        journal,
        names: ["write"],
        maxToolCalls: 10,
        onLimit: vi.fn(),
        writeScope: new Set(["a"]),
      });
    cleanup.push(() => tools.runtime.dispose());
    await tools.runtime.execute("write", { path: "alias/a", content: "alias" });
    expect(await readFile(path.join(root, "a"), "utf8")).toBe("alias");
    expect((await journal.finalize()).files).toMatchObject([{ path: "a", kind: "modified" }]);
  });
  it("holds the Worker writeScope across a read, model wait and edit until scope disposal", async () => {
    const root = await workspace(),
      paused = gate(),
      resume = gate();
    const fake = createFakePi({
      responses: [
        fauxAssistantMessage(fauxToolCall("read", { path: "a" }), { stopReason: "toolUse" }),
        async () => {
          paused.resolve();
          await resume.promise;
          return fauxAssistantMessage(
            fauxToolCall("edit", {
              path: "a",
              edits: [{ oldText: "original", newText: "worker" }],
            }),
            { stopReason: "toolUse" },
          );
        },
        fauxAssistantMessage('{"ok":true}'),
      ],
    });
    const modelRuntime = await ModelRuntime.create({ refreshOnCreate: false });
    modelRuntime.registerNativeProvider(fake.provider);
    await modelRuntime.refresh({ allowNetwork: false });
    const worker: WorkerDefinition = {
      name: "lease-worker",
      description: "test",
      tools: ["read", "edit"],
      cheaper: {
        model: { tier: "low" },
        output: { schema: { inline: {} } },
        writeScope: "targets",
      },
      instructions: "Read and edit.",
      sourcePath: "test.md",
      origin: "explicit",
      outputSchema: compileSchema(
        { type: "object", required: ["ok"], properties: { ok: { type: "boolean" } } },
        "output",
      ),
    };
    const policy: RuntimePolicy = {
      execution: {
        timeoutMs: 10000,
        maxTurns: 10,
        maxToolCalls: 20,
        concurrency: { maxOperations: 2, mutations: "per-resource" },
      },
      tools: { allow: ["read", "edit", "write"] },
      filesystem: { symlinks: "deny", denyPaths: [] },
      model: { maxTier: "high", tierFallback: "none" },
      output: { maxTokens: 1024, repairAttempts: 1 },
      retry: { transientAttempts: 0 },
    };
    const invocation = createInvocationToolRuntime({
      root,
      symlinkPolicy: "deny",
      concurrency: policy.execution.concurrency,
      abortGraceMs: 0,
    });
    cleanup.push(async () => {
      resume.resolve();
      await invocation.dispose();
    });
    const p = runWorker({
      root,
      worker,
      input: { targets: ["a"] },
      policy,
      catalog: modelRuntime,
      modelRuntime,
      modelConfig: { overrides: { "cheaper-fake/fake-model": "low" }, preferences: {} },
      streamFn: fake.streamFn,
      execution: { runtime: invocation, ownerId: "worker" },
    });
    void p.catch(() => {});
    await paused.promise;
    let changed = false;
    const foreign = invocation.createWorkerScope({
      root,
      symlinkPolicy: "deny",
      ownerId: "foreign",
      names: ["write"],
      maxToolCalls: 10,
      onLimit: vi.fn(),
      handlers: {
        write: async () => {
          changed = true;
          await writeFile(path.join(root, "a"), "foreign\n");
          return { content: [{ type: "text", text: "ok" }] };
        },
      },
    });
    const q = foreign.execute("write", { path: "a", content: "foreign" });
    void q.catch(() => {});
    await new Promise((resolve) => setTimeout(resolve, 30));
    expect(changed).toBe(false);
    expect(invocation.operations("foreign")[0]?.state).toBe("queued");
    resume.resolve();
    const output = await p;
    expect(output.result).toEqual({ ok: true });
    expect(output.meta.changes.files).toMatchObject([{ path: "a", kind: "modified" }]);
    expect(
      invocation.operations("worker").map((operation) => [operation.toolName, operation.state]),
    ).toEqual([
      ["read", "completed"],
      ["edit", "completed"],
    ]);
    await q;
    expect(await readFile(path.join(root, "a"), "utf8")).toBe("foreign\n");
  });
});
