import { mkdtemp, open, realpath, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { countTextTokens, createPathDenylist, type ToolName } from "@cheaper/core";
import {
  MutationJournal as CanonicalJournal,
  createToolCatalog,
  createToolRuntime,
} from "@cheaper/tool-runtime";
import {
  createEditToolDefinition,
  createLsToolDefinition,
  createWriteToolDefinition,
} from "@earendil-works/pi-coding-agent";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MutationJournal as PublicJournal } from "../src/index.js";
import { MutationJournal } from "../src/journal.js";
import { createPiToolHandlers, createSandboxedTools } from "../src/tools/registry.js";

let root: string;
beforeEach(async () => {
  root = await realpath(await mkdtemp(path.join(tmpdir(), "cheaper-parity-")));
  await writeFile(path.join(root, "a.txt"), "alpha\nbeta\ngamma\n");
  await writeFile(path.join(root, "huge.txt"), "abcdefghi ".repeat(7 * 1024));
});
afterEach(async () => {
  await rm(root, { recursive: true, force: true });
});
function pair(names: ToolName[], restrict = false) {
  const opts = {
    root,
    symlinkPolicy: "deny" as const,
    names,
    maxToolCalls: 100,
    onLimit: vi.fn(),
    denylist: createPathDenylist(restrict ? ["a.txt"] : []),
    writeScope: new Set(["allowed.txt"]),
  };
  const journal = new MutationJournal();
  const direct = createToolRuntime({
    ...opts,
    journal,
    handlers: createPiToolHandlers({ ...opts, journal }),
  });
  const pi = createSandboxedTools({ ...opts, journal: new MutationJournal() });
  const executePi = (name: string, input: unknown) => {
    const tool = pi.tools.find((t) => t.name === name);
    if (!tool) throw new Error(`missing Pi tool: ${name}`);
    return tool.execute("c1", input as never, undefined, undefined, undefined as never);
  };
  return { direct, pi, executePi };
}
describe("canonical / Pi adapter contract on the same real workspace", () => {
  it.each([
    ["read", { path: "a.txt", limit: 1 }],
    ["read", { path: "huge.txt" }],
    ["grep", { pattern: ".", path: "a.txt", maxResults: 1 }],
    ["find", { pattern: "*.txt", maxResults: 1 }],
    ["count_tokens", { path: "a.txt" }],
    ["ls", { path: ".", limit: 1 }],
  ] as const)("preserves content/details for %s", async (name, input) => {
    const p = pair([name]);
    const direct = await p.direct.execute(name, input);
    const pi = await p.executePi(name, input);
    expect(pi).toEqual(direct);
    expect(p.pi.budget.toolCalls).toBe(1);
    expect(p.pi.meter.sourceTokens).toBe(
      direct.content.reduce((n, c) => n + countTextTokens(c.text), 0),
    );
    await p.direct.dispose();
    await p.pi.runtime.dispose();
  });
  it.each(["read", "grep", "find", "count_tokens", "ls", "write", "edit"] as const)(
    "preserves boundary rejects for %s",
    async (name) => {
      const p = pair([name]);
      const input = {
        path: "../escape.txt",
        pattern: ".",
        content: "x",
        edits: [{ oldText: "a", newText: "b" }],
      };
      const [a, b] = await Promise.all([
        p.direct.execute(name, input).catch((e) => e.message),
        p.executePi(name, input).catch((e) => e.message),
      ]);
      expect(a).toContain("Access denied");
      expect(b).toBe(a);
      expect(p.direct.budget.rejectedToolCalls).toBe(1);
      expect(p.pi.budget.rejectedToolCalls).toBe(1);
      await p.direct.dispose();
      await p.pi.runtime.dispose();
    },
  );
  it.each(["write", "edit"] as const)("preserves writeScope rejects for %s", async (name) => {
    const p = pair([name]);
    const input = { path: "a.txt", content: "x", edits: [{ oldText: "alpha", newText: "z" }] };
    const a = await p.direct.execute(name, input).catch((e) => e.message);
    const b = await p.executePi(name, input).catch((e) => e.message);
    expect(a).toContain("Access denied");
    expect(b).toBe(a);
    await p.direct.dispose();
    await p.pi.runtime.dispose();
  });
  it("preserves deny rejection and filter results", async () => {
    const p = pair(["read", "grep", "find", "count_tokens"], true);
    for (const name of ["read", "count_tokens"]) {
      const input = { path: "a.txt" };
      const a = await p.direct.execute(name, input).catch((e) => e.message);
      const b = await p.executePi(name, input).catch((e) => e.message);
      expect(a).toContain("Access denied");
      expect(b).toBe(a);
    }
    for (const name of ["find", "grep"])
      expect(await p.executePi(name, { pattern: "a.txt" })).toEqual(
        await p.direct.execute(name, { pattern: "a.txt" }),
      );
    await p.direct.dispose();
    await p.pi.runtime.dispose();
  });
  it("pins helper input schemas to the catalog and journal identity", () => {
    const catalog = createToolCatalog();
    for (const t of [
      createWriteToolDefinition(root),
      createEditToolDefinition(root),
      createLsToolDefinition(root),
    ])
      expect(catalog.get(t.name)?.inputSchema).toEqual(t.parameters);
    expect(MutationJournal).toBe(CanonicalJournal);
    expect(PublicJournal).toBe(CanonicalJournal);
    expect(new MutationJournal()).toBeInstanceOf(CanonicalJournal);
  });
  it("preserves symlink escape and token byte-cap rejection", async () => {
    await symlink(tmpdir(), path.join(root, "escape"), "junction");
    const p = pair(["read", "grep", "find", "count_tokens"]);
    for (const name of ["read", "grep", "find", "count_tokens"]) {
      const input = { path: "escape/a.txt", pattern: "." };
      const a = await p.direct.execute(name, input).catch((e) => e.message);
      const b = await p.executePi(name, input).catch((e) => e.message);
      expect(a).toContain("Access denied");
      expect(b).toBe(a);
    }
    const handle = await open(path.join(root, "capped.txt"), "w");
    try {
      await handle.truncate(16 * 1024 * 1024 + 1);
    } finally {
      await handle.close();
    }
    const input = { path: "capped.txt" };
    const a = await p.direct.execute("count_tokens", input).catch((e) => e.message);
    const b = await p.executePi("count_tokens", input).catch((e) => e.message);
    expect(a).toContain("byte cap");
    expect(b).toBe(a);
    await p.direct.dispose();
    await p.pi.runtime.dispose();
  });
  it("rejects malformed helper inputs before mutation on both paths", async () => {
    const p = pair(["write", "edit"]);
    for (const name of ["write", "edit"]) {
      await expect(
        p.direct.execute(name, {
          path: "allowed.txt",
          content: 3,
          edits: [{ oldText: 2, newText: "x" }],
        }),
      ).rejects.toMatchObject({ code: "INVALID_INPUT" });
      await expect(
        p.executePi(name, {
          path: "allowed.txt",
          content: 3,
          edits: [{ oldText: 2, newText: "x" }],
        }),
      ).rejects.toMatchObject({ code: "INVALID_INPUT" });
    }
    expect(p.direct.journal.isEmpty()).toBe(true);
    expect(p.pi.runtime.journal.isEmpty()).toBe(true);
    expect(p.direct.budget.toolCalls).toBe(0);
    expect(p.pi.budget.toolCalls).toBe(0);
    await p.direct.dispose();
    await p.pi.runtime.dispose();
  });
});
