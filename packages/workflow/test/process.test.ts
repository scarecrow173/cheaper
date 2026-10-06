import { mkdtemp, readFile, realpath, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { createInvocationToolRuntime, createToolCatalog } from "@cheaper/tool-runtime";
import { expect, it, vi } from "vitest";
import { WorkspaceResourceInference } from "../src/index.js";
import { definition, fixtures } from "./fixtures.js";

it("Workflow compiles a policy process producer and executes canonical materialized jobs", async () => {
  const root = await realpath(await mkdtemp(path.join(process.cwd(), ".tmp-workflow-process-")));
  await writeFile(
    path.join(root, "child.mjs"),
    'import {writeFileSync} from "node:fs"; writeFileSync("out.txt", "generated");',
  );
  const processPolicy = {
    maxConcurrent: 1,
    tasks: {
      generate: { command: ["node", "child.mjs"], inputs: ["child.mjs"], outputs: ["out.txt"] },
    },
  };
  const tools = createToolCatalog({ process: processPolicy });
  const resources = new WorkspaceResourceInference({
    workspaceRoot: root,
    symlinkPolicy: "deny",
    tools,
  });
  const raw = definition();
  raw.jobs.producer = { kind: "tool", tool: "process", input: { task: "generate" } };
  raw.jobs.consumer = { kind: "tool", tool: "read", input: { path: "out.txt" } };
  const runtime = createInvocationToolRuntime({
    root,
    symlinkPolicy: "deny",
    process: processPolicy,
  });
  try {
    const compiled = await fixtures({ tools, resources }).compiler.compile(raw);
    expect(compiled.edges).toContainEqual({ from: "producer", to: "consumer" });
    const result = new Map();
    expect(compiled.order).toEqual(["producer", "consumer"]);
    for (const id of compiled.order) {
      const scope = runtime.createWorkerScope({
        root,
        symlinkPolicy: "deny",
        ownerId: id,
        names: [id === "producer" ? "process" : "read"],
        maxToolCalls: 10,
        onLimit: vi.fn(),
      });
      result.set(
        id,
        await scope.execute(
          id === "producer" ? "process" : "read",
          compiled.materializeJobInput(id, result),
        ),
      );
    }
    expect(await readFile(path.join(root, "out.txt"), "utf8")).toBe("generated");
    expect(result.get("consumer").content[0].text).toBe("generated");
    for (const produces of [[], [{ path: "outside.txt", scope: "exact" as const }]]) {
      raw.jobs.producer.resources = { produces };
      await expect(fixtures({ tools, resources }).compiler.compile(raw)).rejects.toMatchObject({
        code: "INVALID_REQUEST",
      });
    }
    await expect(
      resources.infer({
        a: { job: { kind: "tool", tool: "process", input: { task: "unknown" } } },
      }),
    ).rejects.toThrow();
    await expect(
      resources.infer({
        a: {
          job: { kind: "tool", tool: "process", input: { task: "generate" } },
          process: { task: "different", inputs: [], outputs: [] },
        },
      }),
    ).rejects.toMatchObject({ code: "INVALID_REQUEST" });
  } finally {
    await runtime.dispose();
    await rm(root, { recursive: true, force: true });
  }
}, 20000);
