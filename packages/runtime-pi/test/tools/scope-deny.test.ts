import { mkdir, mkdtemp, readFile, realpath, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { createPathDenylist } from "@cheaper/core";
import type { ToolDefinition } from "@earendil-works/pi-coding-agent";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MutationJournal } from "../../src/journal.js";
import { createSandboxedTools } from "../../src/tools/registry.js";
import { POLICY_DENIED_MESSAGE, SCOPE_DENIED_MESSAGE } from "../../src/tools/walker.js";

function byName(tools: ToolDefinition[], name: string): ToolDefinition {
  const t = tools.find((x) => x.name === name);
  if (t === undefined) throw new Error(`tool not found: ${name}`);
  return t;
}

let dir: string;
let root: string;

beforeEach(async () => {
  dir = await mkdtemp(path.join(tmpdir(), "cheaper-scope-"));
  root = await realpath(dir);
});
afterEach(async () => {
  await rm(dir, { recursive: true, force: true });
});

const ALL = ["read", "grep", "find", "ls", "write", "edit", "count_tokens"] as const;

function setup(opts?: { writeScope?: string[]; denyPaths?: string[] }) {
  const journal = new MutationJournal();
  const reg = createSandboxedTools({
    root,
    symlinkPolicy: "within-workspace",
    journal,
    names: [...ALL],
    maxToolCalls: 100,
    onLimit: vi.fn(),
    denylist: createPathDenylist(opts?.denyPaths ?? []),
    writeScope: opts?.writeScope === undefined ? undefined : new Set(opts.writeScope),
  });
  return { journal, reg };
}

async function call(tool: ToolDefinition, params: unknown) {
  return tool.execute("c1", params as never, undefined, undefined, undefined as never);
}

describe("write scope", () => {
  it("allows writes to scope paths and records the mutation", async () => {
    const { reg, journal } = setup({ writeScope: ["out/a.txt"] });
    await call(byName(reg.tools, "write"), { path: "out/a.txt", content: "hi\n" });
    expect(await readFile(path.join(root, "out", "a.txt"), "utf8")).toBe("hi\n");
    expect((await journal.finalize()).files[0]?.kind).toBe("created");
  });

  it("rejects writes outside the scope and counts the rejection", async () => {
    const { reg } = setup({ writeScope: ["out/a.txt"] });
    await expect(
      call(byName(reg.tools, "write"), { path: "other.txt", content: "x" }),
    ).rejects.toThrow(SCOPE_DENIED_MESSAGE);
    expect(reg.budget.rejectedToolCalls).toBe(1);
  });

  it("rejects edit writes outside the scope but allows reads", async () => {
    await writeFile(path.join(root, "keep.txt"), "abc", "utf8");
    const { reg } = setup({ writeScope: ["keep.txt"] });
    await call(byName(reg.tools, "edit"), {
      path: "keep.txt",
      edits: [{ oldText: "abc", newText: "xyz" }],
    });
    expect(await readFile(path.join(root, "keep.txt"), "utf8")).toBe("xyz");
    await writeFile(path.join(root, "other.txt"), "abc", "utf8");
    await expect(
      call(byName(reg.tools, "edit"), {
        path: "other.txt",
        edits: [{ oldText: "abc", newText: "zzz" }],
      }),
    ).rejects.toThrow(SCOPE_DENIED_MESSAGE);
    // read outside scope stays allowed
    const r = await call(byName(reg.tools, "read"), { path: "other.txt" });
    expect(JSON.stringify(r.content)).toContain("abc");
  });

  it("rejects mkdir that is not an ancestor of a scope path", async () => {
    const { reg } = setup({ writeScope: ["out/a.txt"] });
    await expect(
      call(byName(reg.tools, "write"), { path: "out/a.txt", content: "x" }),
    ).resolves.toBeDefined(); // mkdir(out) allowed
    const { reg: reg2 } = setup({ writeScope: ["out/a.txt"] });
    const write = byName(reg2.tools, "write");
    await expect(call(write, { path: "nope/../deep/b.txt", content: "x" })).rejects.toThrow(
      SCOPE_DENIED_MESSAGE,
    );
  });

  it("leaves writes unrestricted when no scope is set", async () => {
    const { reg } = setup();
    await call(byName(reg.tools, "write"), { path: "anywhere/x.txt", content: "1" });
    expect(await readFile(path.join(root, "anywhere", "x.txt"), "utf8")).toBe("1");
  });
});

describe("privacy deny list", () => {
  beforeEach(async () => {
    await writeFile(path.join(root, ".env"), "SECRET=1\n", "utf8");
    await mkdir(path.join(root, ".ssh"), { recursive: true });
    await writeFile(path.join(root, ".ssh", "id_rsa"), "KEY\n", "utf8");
    await writeFile(path.join(root, "normal.txt"), "hello secret\n", "utf8");
  });

  it("rejects read of denied files", async () => {
    const { reg } = setup();
    await expect(call(byName(reg.tools, "read"), { path: ".env" })).rejects.toThrow(
      POLICY_DENIED_MESSAGE,
    );
    await expect(call(byName(reg.tools, "read"), { path: ".ssh/id_rsa" })).rejects.toThrow(
      POLICY_DENIED_MESSAGE,
    );
  });

  it("rejects count_tokens on denied files", async () => {
    const { reg } = setup();
    await expect(call(byName(reg.tools, "count_tokens"), { path: ".env" })).rejects.toThrow(
      POLICY_DENIED_MESSAGE,
    );
  });

  it("rejects writes to denied paths even inside the write scope", async () => {
    const { reg } = setup({ writeScope: [".env"] });
    await expect(call(byName(reg.tools, "write"), { path: ".env", content: "x" })).rejects.toThrow(
      POLICY_DENIED_MESSAGE,
    );
  });

  it("hides denied entries from ls listings", async () => {
    const { reg } = setup();
    const result = await call(byName(reg.tools, "ls"), {});
    const text = JSON.stringify(result.content);
    expect(text).toContain("normal.txt");
    expect(text).not.toContain(".env");
    expect(text).not.toContain(".ssh");
  });

  it("excludes denied files from find results", async () => {
    const { reg } = setup();
    const result = await call(byName(reg.tools, "find"), { pattern: "**/*" });
    const text = JSON.stringify(result.content);
    expect(text).toContain("normal.txt");
    expect(text).not.toContain(".env");
    expect(text).not.toContain("id_rsa");
  });

  it("does not search inside denied files with grep", async () => {
    const { reg } = setup();
    const result = await call(byName(reg.tools, "grep"), { pattern: "SECRET" });
    const text = JSON.stringify(result.content);
    expect(text).not.toContain("SECRET=1");
    // normal.txt contains 'secret' (case differs) — ensure allowed files still searched
    const result2 = await call(byName(reg.tools, "grep"), { pattern: "secret" });
    expect(JSON.stringify(result2.content)).toContain("normal.txt");
  });

  it("applies extra policy denyPaths on top of built-ins", async () => {
    await mkdir(path.join(root, "internal"), { recursive: true });
    await writeFile(path.join(root, "internal", "plan.md"), "x", "utf8");
    const { reg } = setup({ denyPaths: ["internal/**"] });
    const result = await call(byName(reg.tools, "find"), { pattern: "**/*" });
    const text = JSON.stringify(result.content);
    expect(text).not.toContain("internal");
    expect(text).toContain("normal.txt");
  });
});
