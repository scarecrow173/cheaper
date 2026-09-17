import { mkdtemp, readFile, realpath, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { createPathDenylist } from "@cheaper/core";
import type { ToolDefinition } from "@earendil-works/pi-coding-agent";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MutationJournal } from "../../src/journal.js";
import { createSandboxedTools } from "../../src/tools/registry.js";
import { ACCESS_DENIED_MESSAGE, POLICY_DENIED_MESSAGE } from "../../src/tools/walker.js";

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
  denyPaths?: string[],
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
    ...(denyPaths !== undefined ? { denylist: createPathDenylist(denyPaths) } : {}),
  });
  return { journal, onLimit, reg };
}

function textOf(result: { content: Array<{ type: string; text?: string }> }): string {
  return result.content
    .filter((c): c is { type: "text"; text: string } => c.type === "text")
    .map((c) => c.text)
    .join("\n");
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

  it("read tool returns bounded verbatim content with details", async () => {
    await writeFile(path.join(root, "r.txt"), "body\n", "utf8");
    const { reg } = setup(["read"]);
    const result = await first(reg.tools).execute(
      "c1",
      { path: "r.txt" } as never,
      undefined,
      undefined,
      undefined as never,
    );
    const textBlocks = result.content.filter(
      (c): c is { type: "text"; text: string } => c.type === "text",
    );
    // 応答は verbatim テキスト 1 ブロック（DESIGN-0019 の新契約）。
    expect(textBlocks).toHaveLength(1);
    expect(textBlocks[0]?.text).toContain("body");
    expect(result.details).toMatchObject({
      startLine: 1,
      endLine: 2,
      eof: true,
      truncatedBy: null,
      totalLines: 2,
    });
  });

  it("read tool denies workspace escapes", async () => {
    const { reg } = setup(["read"]);
    await expect(
      first(reg.tools).execute(
        "c1",
        { path: "../evil.txt" } as never,
        undefined,
        undefined,
        undefined as never,
      ),
    ).rejects.toThrow(ACCESS_DENIED_MESSAGE);
  });

  it("read tool pages with offset/limit continuation via the registry", async () => {
    await writeFile(path.join(root, "p.txt"), "l1\nl2\nl3\nl4\nl5\n", "utf8");
    const { reg } = setup(["read"]);
    const read = first(reg.tools);
    const page1 = await read.execute(
      "c1",
      { path: "p.txt", limit: 2 } as never,
      undefined,
      undefined,
      undefined as never,
    );
    expect(textOf(page1)).toBe("l1\nl2\n[Showing lines 1-2. Use offset=3 to continue.]");
    expect(page1.details).toMatchObject({
      startLine: 1,
      endLine: 2,
      eof: false,
      truncatedBy: "limit",
      continuation: { offset: 3 },
    });
    const page2 = await read.execute(
      "c2",
      { path: "p.txt", offset: 3 } as never,
      undefined,
      undefined,
      undefined as never,
    );
    expect(textOf(page2)).toBe("l3\nl4\nl5\n");
    expect(page2.details).toMatchObject({
      startLine: 3,
      endLine: 6,
      eof: true,
      truncatedBy: null,
      totalLines: 6,
    });
  });

  it("read tool enforces built-in and custom deny paths via the registry", async () => {
    await writeFile(path.join(root, ".env"), "SECRET=1\n", "utf8");
    await writeFile(path.join(root, "ok.txt"), "x", "utf8");
    const { reg } = setup(["read"], 100, ["ok.txt"]);
    const read = first(reg.tools);
    await expect(
      read.execute("c1", { path: ".env" } as never, undefined, undefined, undefined as never),
    ).rejects.toThrow(POLICY_DENIED_MESSAGE);
    await expect(
      read.execute("c2", { path: "ok.txt" } as never, undefined, undefined, undefined as never),
    ).rejects.toThrow(POLICY_DENIED_MESSAGE);
  });

  it("grep tool returns matches with a bounded skipped footer via the registry", async () => {
    await writeFile(path.join(root, "hit.txt"), "needle\n", "utf8");
    await writeFile(path.join(root, "b.dat"), Buffer.from([0x61, 0x00, 0x62]));
    const { reg } = setup(["grep"]);
    const result = await first(reg.tools).execute(
      "c1",
      { pattern: "needle" } as never,
      undefined,
      undefined,
      undefined as never,
    );
    expect(textOf(result)).toBe("hit.txt:1: needle\n[skipped: binary — b.dat]");
  });

  it("grep tool hides denied files from results and footer via the registry", async () => {
    await writeFile(path.join(root, ".env"), "needle\n", "utf8");
    await writeFile(path.join(root, "ok.txt"), "needle\n", "utf8");
    const { reg } = setup(["grep"], 100, []);
    const result = await first(reg.tools).execute(
      "c1",
      { pattern: "needle" } as never,
      undefined,
      undefined,
      undefined as never,
    );
    expect(textOf(result)).toBe("ok.txt:1: needle");
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
