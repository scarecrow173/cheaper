import { mkdtemp, realpath, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { compileSchema, countTextTokens, createPathDenylist } from "@cheaper/core";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createToolCatalog, createToolRuntime, MutationJournal } from "../src/index.js";

let root: string;
beforeEach(async () => {
  root = await realpath(await mkdtemp(path.join(tmpdir(), "cheaper-direct-")));
  await writeFile(path.join(root, "a.txt"), "hello world\nsecond\n");
});
afterEach(async () => {
  await rm(root, { recursive: true, force: true });
});
const options = () => ({
  root,
  symlinkPolicy: "deny" as const,
  names: ["read", "grep", "find", "count_tokens"] as const,
  maxToolCalls: 100,
  onLimit: vi.fn(),
});

describe("Pi-free canonical runtime", () => {
  it("exposes seven schemas without a workspace or handlers", () => {
    const catalog = createToolCatalog();
    expect(catalog.list().map((t) => t.name)).toEqual([
      "read",
      "write",
      "edit",
      "ls",
      "grep",
      "find",
      "count_tokens",
    ]);
    expect(catalog.get("missing")).toBeUndefined();
    for (const t of catalog.list()) {
      expect(t.inputSchema.type).toBe("object");
      expect(t.outputSchema.type).toBe("object");
    }
  });
  it("executes owned tools and retains schema defaults in handlers", async () => {
    const r = createToolRuntime(options());
    expect((await r.execute("read", { path: "a.txt" })).content[0]?.text).toBe(
      "hello world\nsecond\n",
    );
    expect((await r.execute("grep", { pattern: "hello" })).content[0]?.text).toContain("a.txt:1:");
    expect((await r.execute("find", { pattern: "*.txt" })).content[0]?.text).toBe("a.txt");
    const counted = await r.execute("count_tokens", { path: "a.txt" });
    const text = counted.content[0]?.text;
    if (text === undefined) throw new Error("count result has no text");
    expect(counted.details).toEqual(JSON.parse(text));
    const output = r.catalog.get("count_tokens")?.outputSchema;
    if (!output) throw new Error("missing count output schema");
    expect(compileSchema(output, "output").validate(counted).ok).toBe(true);
    await r.dispose();
  });
  it.each([
    ["read", { path: 1 }],
    ["read", { path: "a.txt", limit: 2001 }],
    ["grep", { pattern: [] }],
    ["find", {}],
    ["count_tokens", { path: false }],
  ])("rejects malformed %s at the canonical entrance", async (name, input) => {
    const r = createToolRuntime(options());
    await expect(r.execute(name as string, input)).rejects.toMatchObject({ code: "INVALID_INPUT" });
    expect(r.budget.toolCalls).toBe(0);
    await r.dispose();
  });
  it.each(["write", "edit", "ls"] as const)("rejects unregistered %s", async (name) => {
    const r = createToolRuntime({ ...options(), names: [name] });
    await expect(r.execute(name, {})).rejects.toMatchObject({ code: "UNKNOWN_TOOL" });
    await r.dispose();
  });
  it("reserves sibling budget once and meters successful text once", async () => {
    const onLimit = vi.fn();
    const handler = vi.fn(async () => ({ content: [{ type: "text" as const, text: "done" }] }));
    const r = createToolRuntime({
      ...options(),
      names: ["write"],
      maxToolCalls: 1,
      onLimit,
      handlers: { write: handler },
    });
    const results = await Promise.allSettled([
      r.execute("write", { path: "a.txt", content: "x" }),
      r.execute("write", { path: "a.txt", content: "y" }),
    ]);
    expect(results.map((x) => x.status)).toEqual(["fulfilled", "rejected"]);
    expect(handler).toHaveBeenCalledTimes(1);
    expect(r.budget.toolCalls).toBe(1);
    expect(r.budget.rejectedToolCalls).toBe(1);
    expect(r.meter.sourceTokens).toBe(countTextTokens("done"));
    expect(onLimit).toHaveBeenCalledTimes(1);
    await r.dispose();
  });
  it("counts access rejects once without output tokens", async () => {
    const r = createToolRuntime({
      ...options(),
      denylist: createPathDenylist(["a.txt"]),
      journal: new MutationJournal(),
    });
    await expect(r.execute("read", { path: "a.txt" })).rejects.toThrow("Access denied");
    expect(r.budget.toolCalls).toBe(1);
    expect(r.budget.rejectedToolCalls).toBe(1);
    expect(r.meter.sourceTokens).toBe(0);
    await r.dispose();
  });
});
