import { mkdir, mkdtemp, realpath, rm, symlink } from "node:fs/promises";
import path from "node:path";
import { compileSchema, type ResourceClaim } from "@cheaper/core";
import { createToolCatalog } from "@cheaper/tool-runtime";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  type ResourceInferenceJob,
  type WorkflowResources,
  WorkspaceResourceInference,
} from "../src/index.js";
import { fixtures } from "./fixtures.js";

let root: string;
beforeEach(async () => {
  root = await realpath(await mkdtemp(path.join(process.cwd(), ".tmp-inference-")));
});
afterEach(async () => {
  expect(path.dirname(root)).toBe(await realpath(process.cwd()));
  await rm(root, { recursive: true, force: true });
});
const exact = (resourcePath: string) => ({ path: resourcePath, scope: "exact" as const });
const subtree = (resourcePath: string) => ({ path: resourcePath, scope: "subtree" as const });
const tool = (
  name: string,
  input: ResourceInferenceJob["job"]["input"],
  resources?: WorkflowResources,
): ResourceInferenceJob => ({
  job: { kind: "tool", tool: name, input, ...(resources ? { resources } : {}) },
});
const claim = (
  resourcePath: string,
  mode: ResourceClaim["mode"],
  scope: "exact" | "subtree" = "exact",
): ResourceClaim => ({
  resource: { kind: "workspace", path: path.join(root, resourcePath), scope },
  mode,
});
const inference = () =>
  new WorkspaceResourceInference({
    workspaceRoot: root,
    symlinkPolicy: "within-workspace",
    tools: createToolCatalog(),
  });

describe("catalog resources and fallback", () => {
  it.each(["read", "count_tokens", "write", "edit"])(
    "catalog-read-write-edit-scopes: %s",
    async (name) => {
      const result = await inference().infer({
        a: tool(name, { path: "src/a.ts" }, { produces: [] }),
      });
      expect(result.manifests.a).toMatchObject({
        reads: name === "write" ? [] : [claim("src/a.ts", "shared")],
        writes: name === "write" || name === "edit" ? [claim("src/a.ts", "exclusive")] : [],
        produces: [],
      });
    },
  );
  it.each(["grep", "find", "ls"])("catalog-subtree-and-default: %s", async (name) => {
    const result = await inference().infer({
      a: tool(name, { path: "src" }, { produces: [] }),
      b: tool(name, {}, { produces: [] }),
    });
    expect(result.manifests.a?.reads).toEqual([claim("src", "shared", "subtree")]);
    expect(result.manifests.b?.reads).toEqual([claim(".", "shared", "subtree")]);
    expect(result.manifests.a?.writes).toEqual([]);
  });
  it("metadata-absent-fallback and declared-empty-read-array", async () => {
    const adapter = new WorkspaceResourceInference({
      workspaceRoot: root,
      symlinkPolicy: "deny",
      tools: fixtures().ports.tools,
    });
    const result = await adapter.infer({
      unknown: tool("grep", {}),
      noRead: tool("grep", {}, { reads: [] }),
    });
    expect(result.manifests.unknown).toEqual({
      reads: [claim(".", "shared", "subtree")],
      writes: [claim(".", "exclusive", "subtree")],
      produces: [],
    });
    expect(result.manifests.noRead?.reads).toEqual([]);
    expect(result.edges).toEqual([]);
  });
  it("omitted-empty-object-fallback retains known consumer dependencies", async () => {
    const result = await inference().infer({
      producer: tool(
        "write",
        { path: "generated/a.json" },
        { produces: [exact("generated/a.json")] },
      ),
      consumer: tool("read", { path: "generated/a.json" }),
      empty: tool("read", { path: "unrelated.json" }, {}),
    });
    expect(result.manifests.consumer?.reads).toEqual([claim(".", "shared", "subtree")]);
    expect(result.manifests.empty?.writes).toEqual([claim(".", "exclusive", "subtree")]);
    expect(result.edges).toEqual([{ from: "producer", to: "consumer" }]);
  });
  it("known access underclaim is rejected; absolute/deferred paths use fallback", async () => {
    await expect(
      inference().infer({ a: tool("read", { path: "a" }, { reads: [] }) }),
    ).rejects.toMatchObject({ code: "INVALID_REQUEST" });
    const result = await inference().infer({
      absolute: tool("read", { path: path.join(root, "a") }, { produces: [] }),
      deferred: tool(
        "read",
        { path: { $from: "source", path: "/details/path" } },
        { produces: [] },
      ),
    });
    expect(result.manifests.absolute?.reads).toEqual([claim(".", "shared", "subtree")]);
    expect(result.manifests.deferred?.reads).toEqual([claim(".", "shared", "subtree")]);
    expect(result.edges).toEqual([]);
  });
});

describe("Worker authoritative scope", () => {
  const workerJob = (input: ResourceInferenceJob["job"]["input"], resources: WorkflowResources) => {
    const worker = fixtures().worker;
    worker.tools = ["read", "write"];
    worker.cheaper.writeScope = "targets";
    worker.inputSchema = compileSchema({}, "input");
    return {
      job: { kind: "worker", worker: worker.name, input, resources },
      worker,
    } as ResourceInferenceJob;
  };
  it("worker-write-scope-targets and worker-ancestor-mkdir-safety", async () => {
    const result = await inference().infer({
      a: workerJob(
        { targets: ["src/a.ts", { path: "src/b.ts" }] },
        { reads: [exact("input.json")] },
      ),
    });
    expect(result.manifests.a?.reads).toEqual([claim("input.json", "shared")]);
    expect(result.manifests.a?.writes).toEqual(
      expect.arrayContaining([
        claim("src/a.ts", "exclusive"),
        claim("src/b.ts", "exclusive"),
        claim(".", "exclusive", "subtree"),
      ]),
    );
    expect(result.diagnostics).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ kind: "fallback", job: "a", reason: "worker-ancestor-mkdir" }),
      ]),
    );
  });
  it("worker-underclaim-rejection", async () => {
    for (const resources of [
      { reads: [], writes: [exact("outside.ts")] },
      { reads: [], writes: [] },
      { reads: [], produces: [subtree("src")] },
    ]) {
      await expect(
        inference().infer({ a: workerJob({ targets: ["src/a.ts"] }, resources) }),
      ).rejects.toMatchObject({ code: "INVALID_REQUEST" });
    }
    for (const targets of [null, 1, [2]]) {
      await expect(
        inference().infer({ a: workerJob({ targets }, { reads: [] }) }),
      ).rejects.toMatchObject({ code: "INVALID_INPUT" });
    }
  });
  it("deferred writeScope is unknown and cannot validate narrow produces", async () => {
    const item = { $from: "source", path: "/details/targets" };
    const result = await inference().infer({ a: workerJob({ targets: item }, { reads: [] }) });
    expect(result.manifests.a?.writes).toEqual([claim(".", "exclusive", "subtree")]);
    await expect(
      inference().infer({ a: workerJob({ targets: item }, { produces: [exact("src/a.ts")] }) }),
    ).rejects.toMatchObject({ code: "INVALID_REQUEST" });
  });
  it("worker-read-only-and-unknown-mutation", async () => {
    const worker = fixtures().worker;
    const job = {
      kind: "worker" as const,
      worker: worker.name,
      input: { task: "x" },
      resources: { reads: [] },
    };
    const result = await inference().infer({ a: { job, worker } });
    expect(result.manifests.a).toEqual({ reads: [], writes: [], produces: [] });
    await expect(
      inference().infer({ a: { job: { ...job, resources: { produces: [exact("a")] } }, worker } }),
    ).rejects.toMatchObject({ code: "INVALID_REQUEST" });
    worker.tools.push("write");
    const mutable = await inference().infer({ a: { job, worker } });
    expect(mutable.manifests.a?.writes).toEqual([claim(".", "exclusive", "subtree")]);
  });
});

describe("dependency, ownership and conflict", () => {
  const inference = () =>
    new WorkspaceResourceInference({
      workspaceRoot: root,
      symlinkPolicy: "within-workspace",
      tools: fixtures().ports.tools,
    });
  const declared = (resources: WorkflowResources) => tool("grep", {}, resources);
  it("produces-to-read and no-self-edge", async () => {
    const result = await inference().infer({
      a: declared({ reads: [exact("out/a")], writes: [], produces: [exact("out/a")] }),
      b: declared({ reads: [subtree("out")], writes: [] }),
    });
    expect(result.edges).toEqual([{ from: "a", to: "b" }]);
    expect(result.diagnostics).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          kind: "dependency",
          jobs: ["a", "b"],
          claims: [claim("out/a", "exclusive"), claim("out", "shared", "subtree")],
        }),
      ]),
    );
  });
  it("writers-conflict-without-edge and component-boundary", async () => {
    const result = await inference().infer({
      a: declared({ reads: [], writes: [exact("src/a")] }),
      b: declared({ reads: [], writes: [exact("src/a")] }),
      c: declared({ reads: [], writes: [exact("src/ab")] }),
    });
    expect(result.edges).toEqual([]);
    expect(result.conflicts).toEqual([
      { jobs: ["a", "b"], claims: [claim("src/a", "exclusive"), claim("src/a", "exclusive")] },
    ]);
  });
  it("unknown-read-does-not-order; exclusive-read-write-overlap remains a conflict", async () => {
    const result = await inference().infer({
      a: declared({ reads: [], writes: [], produces: [exact("out/a")] }),
      b: tool("read", { path: { $from: "a", path: "/details/path" } }),
    });
    expect(result.edges).toEqual([]);
    expect(result.conflicts?.length).toBeGreaterThan(0);
  });
  it.each([
    [exact("out/a"), exact("out/./a")],
    [subtree("out"), exact("out/a")],
    [subtree("out"), subtree("out/sub")],
  ])("duplicate exact/subtree ownership is deterministic", async (left, right) => {
    const jobs = {
      a: declared({ reads: [], writes: [], produces: [left] }),
      b: declared({ reads: [], writes: [], produces: [right] }),
    };
    let first: unknown;
    try {
      await inference().infer(jobs);
    } catch (error) {
      first = error;
    }
    expect(first).toMatchObject({
      code: "INVALID_REQUEST",
      details: { kind: "duplicate-producer", jobs: ["a", "b"] },
    });
    await expect(inference().infer({ b: jobs.b, a: jobs.a })).rejects.toMatchObject(
      first as object,
    );
  });
  it("canonical-alias-and-case-output", async () => {
    await mkdir(path.join(root, "real"));
    await symlink(
      path.join(root, "real"),
      path.join(root, "alias"),
      process.platform === "win32" ? "junction" : "dir",
    );
    await expect(
      inference().infer({
        a: declared({ produces: [exact("real/out")] }),
        b: declared({ produces: [exact("alias/out")] }),
      }),
    ).rejects.toMatchObject({ code: "INVALID_REQUEST" });
    const result = inference().infer({
      a: declared({ produces: [exact("Case")] }),
      b: declared({ produces: [exact("case")] }),
    });
    if (process.platform === "win32")
      await expect(result).rejects.toMatchObject({ code: "INVALID_REQUEST" });
    else await expect(result).resolves.toBeDefined();
  });
  it("core-boundary-escape and glob-safety-and-ownership", async () => {
    await expect(
      inference().infer({ a: declared({ produces: [exact("out/*.json")] }) }),
    ).rejects.toMatchObject({ code: "INVALID_REQUEST" });
    await expect(
      inference().infer({ a: declared({ reads: [exact("../escape")] }) }),
    ).rejects.toThrow();
    const result = await inference().infer({
      a: declared({ reads: [exact("src/*.ts")], writes: [], produces: [subtree("out/**")] }),
    });
    expect(result.manifests.a?.reads).toEqual([claim(".", "shared", "subtree")]);
    expect(result.manifests.a?.produces).toEqual([claim("out", "exclusive", "subtree")]);
  });
  it("reversed-job-and-claim-byte-equality and no-input-mutation", async () => {
    const jobs = {
      b: declared({ reads: [exact("z"), exact("out/a")], writes: [exact("state")] }),
      a: declared({
        reads: [],
        writes: [exact("state")],
        produces: [exact("out/b"), exact("out/a"), exact("out/a")],
      }),
    };
    const snapshot = structuredClone(jobs);
    const reversed = structuredClone(jobs);
    for (const item of Object.values(reversed))
      for (const list of Object.values(item.job.resources!)) list.reverse();
    const first = await inference().infer(jobs);
    const second = await inference().infer({ a: reversed.a, b: reversed.b });
    expect(JSON.stringify(first)).toBe(JSON.stringify(second));
    expect(jobs).toEqual(snapshot);
    expect(Object.keys(first.manifests)).toEqual(["a", "b"]);
  });
});

it("process-config-input-output-edge and undeclared-process-rejection", async () => {
  const processJob = tool("process", { task: "generate" });
  await expect(inference().infer({ a: processJob })).rejects.toMatchObject({
    code: "POLICY_VIOLATION",
  });
  const result = await inference().infer({
    a: { ...processJob, process: { task: "generate", inputs: [], outputs: [exact("out/a")] } },
    b: {
      ...tool("process", { task: "consume" }),
      process: { task: "consume", inputs: [exact("out/a")], outputs: [] },
    },
    empty: {
      ...tool("process", { task: "no-io" }),
      process: { task: "no-io", inputs: [], outputs: [] },
      job: { ...processJob.job, input: { task: "no-io" }, resources: { reads: [], writes: [] } },
    },
  });
  expect(result.edges).toEqual([{ from: "a", to: "b" }]);
  expect(result.manifests.a?.produces).toEqual([claim("out/a", "exclusive")]);
  expect(result.manifests.empty).toEqual({ reads: [], writes: [], produces: [] });
});

it("job IDs including prototype keys remain own manifests", async () => {
  const jobs = Object.fromEntries(
    ["__proto__", "constructor", "toString"].map((id) => [id, tool("read", { path: "a" })]),
  );
  const result = await inference().infer(jobs);
  expect(Object.keys(result.manifests)).toEqual(["__proto__", "constructor", "toString"]);
  expect(Object.hasOwn(result.manifests, "__proto__")).toBe(true);
});

it("worker wildcard targets cannot expand authoritative output ownership", async () => {
  const worker = fixtures().worker;
  worker.tools = ["read", "write"];
  worker.cheaper.writeScope = "targets";
  worker.inputSchema = compileSchema({}, "input");
  await expect(
    inference().infer({
      a: {
        job: {
          kind: "worker",
          worker: worker.name,
          input: { targets: ["src/*.ts"] },
          resources: { reads: [], produces: [exact("unrelated.json")] },
        },
        worker,
      },
    }),
  ).rejects.toMatchObject({ code: "INVALID_REQUEST" });
});

it("process source cannot replace another job kind authority", async () => {
  await expect(
    inference().infer({
      a: {
        ...tool("read", { path: "a" }),
        process: { task: "generate", inputs: [], outputs: [exact("b")] },
      },
    }),
  ).rejects.toMatchObject({ code: "INVALID_REQUEST" });
});

it("process subtree glob ownership cannot authorize additional outside outputs", async () => {
  const outputs = [subtree("out/**")];
  await expect(
    inference().infer({
      a: {
        ...tool("process", { task: "generate" }, { produces: [...outputs, exact("outside.json")] }),
        process: { task: "generate", inputs: [], outputs },
      },
    }),
  ).rejects.toMatchObject({ code: "INVALID_REQUEST" });
});
