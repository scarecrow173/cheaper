import { mkdtemp, readFile, realpath, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import type { ToolDefinition } from "@earendil-works/pi-coding-agent";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MutationJournal } from "../../src/journal.js";
import { createSandboxedTools } from "../../src/tools/registry.js";
import { ACCESS_DENIED_MESSAGE } from "../../src/tools/walker.js";

function first(tools: ToolDefinition[]): ToolDefinition {
  const t = tools[0];
  if (t === undefined) throw new Error("no tools");
  return t;
}

let dir: string;
let root: string;

beforeEach(async () => {
  dir = await mkdtemp(path.join(tmpdir(), "cheaper-reg-"));
  root = await realpath(dir);
});
afterEach(async () => {
  await rm(dir, { recursive: true, force: true });
});

function setup(
  names: Array<"read" | "grep" | "find" | "ls" | "write" | "edit" | "count_tokens">,
  maxToolCalls = 100,
) {
  const journal = new MutationJournal();
  const onLimit = vi.fn();
  const reg = createSandboxedTools({
    root,
    symlinkPolicy: "deny",
    journal,
    names,
    maxToolCalls,
    onLimit,
  });
  return { journal, onLimit, reg };
}

describe("createSandboxedTools", () => {
  it("creates one ToolDefinition per vocabulary name with matching allowlist", () => {
    const { reg } = setup(["read", "grep", "find", "ls", "write", "edit", "count_tokens"]);
    expect(reg.tools.map((t) => t.name)).toEqual([
      "read",
      "grep",
      "find",
      "ls",
      "write",
      "edit",
      "count_tokens",
    ]);
    expect(reg.toolNames).toEqual(["read", "grep", "find", "ls", "write", "edit", "count_tokens"]);
  });

  it("creates a subset in the requested order", () => {
    const { reg } = setup(["grep", "read"]);
    expect(reg.tools.map((t) => t.name)).toEqual(["grep", "read"]);
  });

  it("write tool writes through boundary ops and records journal", async () => {
    const { reg, journal } = setup(["write"]);
    const write = first(reg.tools);
    await write.execute(
      "c1",
      { path: "out.txt", content: "hi\n" } as never,
      undefined,
      undefined,
      undefined as never,
    );
    expect(await readFile(path.join(root, "out.txt"), "utf8")).toBe("hi\n");
    const changes = await journal.finalize();
    expect(changes.files[0]?.kind).toBe("created");
  });

  it("read tool reads via boundary ops", async () => {
    await writeFile(path.join(root, "r.txt"), "body\n", "utf8");
    const { reg } = setup(["read"]);
    const result = await first(reg.tools).execute(
      "c1",
      { path: "r.txt" } as never,
      undefined,
      undefined,
      undefined as never,
    );
    const text = result.content
      .filter((c): c is { type: "text"; text: string } => c.type === "text")
      .map((c) => c.text)
      .join("");
    expect(text).toContain("body");
  });

  it("edit tool applies edits via boundary ops", async () => {
    await writeFile(path.join(root, "e.txt"), "abc", "utf8");
    const { reg, journal } = setup(["edit"]);
    await first(reg.tools).execute(
      "c1",
      { path: "e.txt", edits: [{ oldText: "abc", newText: "xyz" }] } as never,
      undefined,
      undefined,
      undefined as never,
    );
    expect(await readFile(path.join(root, "e.txt"), "utf8")).toBe("xyz");
    expect((await journal.finalize()).files[0]?.kind).toBe("modified");
  });

  it("ls tool lists via boundary ops", async () => {
    await writeFile(path.join(root, "l.txt"), "x", "utf8");
    const { reg } = setup(["ls"]);
    const result = await first(reg.tools).execute(
      "c1",
      {} as never,
      undefined,
      undefined,
      undefined as never,
    );
    const text = result.content
      .filter((c): c is { type: "text"; text: string } => c.type === "text")
      .map((c) => c.text)
      .join("");
    expect(text).toContain("l.txt");
  });

  it("denies escapes through the Pi write tool path resolution", async () => {
    const { reg } = setup(["write"]);
    await expect(
      first(reg.tools).execute(
        "c1",
        { path: "../evil.txt", content: "x" } as never,
        undefined,
        undefined,
        undefined as never,
      ),
    ).rejects.toThrow(ACCESS_DENIED_MESSAGE);
  });

  it("wraps every tool with the budget (over-budget call rejects and triggers onLimit)", async () => {
    const { reg, onLimit } = setup(["write"], 1);
    const write = first(reg.tools);
    await write.execute(
      "c1",
      { path: "a.txt", content: "1" } as never,
      undefined,
      undefined,
      undefined as never,
    );
    await expect(
      write.execute(
        "c2",
        { path: "b.txt", content: "2" } as never,
        undefined,
        undefined,
        undefined as never,
      ),
    ).rejects.toThrow(/maxToolCalls/);
    expect(onLimit).toHaveBeenCalledTimes(1);
    expect(reg.budget.limitHit).toBe("maxToolCalls");
    expect(reg.budget.rejectedToolCalls).toBe(1);
  });
});
