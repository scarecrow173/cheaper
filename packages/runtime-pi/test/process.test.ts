import { mkdtemp, realpath, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { createToolRuntime, MutationJournal } from "@cheaper/tool-runtime";
import { expect, it, vi } from "vitest";
import { createSandboxedTools } from "../src/tools/registry.js";

it("Pi exposes the canonical named process with shared accounting and policy", async () => {
  const root = await realpath(await mkdtemp(path.join(process.cwd(), ".tmp-pi-process-")));
  await writeFile(path.join(root, "child.mjs"), 'console.log("canonical")');
  const options = {
    root,
    symlinkPolicy: "deny" as const,
    names: ["process" as const],
    journal: new MutationJournal(),
    maxToolCalls: 10,
    onLimit: vi.fn(),
    process: {
      maxConcurrent: 1,
      tasks: { check: { command: ["node", "child.mjs"], inputs: ["child.mjs"], outputs: [] } },
    },
  };
  const direct = createToolRuntime(options);
  const pi = createSandboxedTools(options);
  try {
    expect(pi.toolNames).toEqual(["process"]);
    const result = await pi.tools[0]!.execute(
      "call",
      { task: "check" } as never,
      undefined,
      undefined,
      undefined as never,
    );
    expect(result).toEqual(await direct.execute("process", { task: "check" }));
    expect(pi.budget.toolCalls).toBe(1);
    expect(pi.runtime.operations().map((o) => o.state)).toEqual(["completed"]);
    await expect(pi.runtime.execute("process", { task: "unknown" })).rejects.toMatchObject({
      code: "POLICY_VIOLATION",
    });
    expect(() =>
      createSandboxedTools({ ...options, process: { maxConcurrent: 1, tasks: {} } }),
    ).toThrow();
  } finally {
    await pi.runtime.dispose();
    await direct.dispose();
    await rm(root, { recursive: true, force: true });
  }
});
