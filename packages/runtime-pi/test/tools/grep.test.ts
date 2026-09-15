import childProcess from "node:child_process";
import { mkdir, mkdtemp, realpath, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createGrepTool } from "../../src/tools/grep.js";

let dir: string;
let root: string;

beforeEach(async () => {
  dir = await mkdtemp(path.join(tmpdir(), "cheaper-grep-"));
  root = await realpath(dir);
});
afterEach(async () => {
  await rm(dir, { recursive: true, force: true });
  vi.restoreAllMocks();
});

function tool(policy: "deny" | "within-workspace" = "deny") {
  return createGrepTool({ root, symlinkPolicy: policy });
}

async function run(t: ReturnType<typeof tool>, params: Record<string, unknown>): Promise<string> {
  const result = await t.execute(
    "call-1",
    params as never,
    undefined,
    undefined,
    undefined as never,
  );
  return result.content
    .filter((c): c is { type: "text"; text: string } => c.type === "text")
    .map((c) => c.text)
    .join("\n");
}

describe("grep tool", () => {
  it("finds matches as display:line: text", async () => {
    await writeFile(path.join(root, "a.txt"), "foo\nbar\nfoo bar\n");
    const out = await run(tool(), { pattern: "foo" });
    expect(out).toBe("a.txt:1: foo\na.txt:3: foo bar");
  });

  it("honors ignoreCase", async () => {
    await writeFile(path.join(root, "a.txt"), "Foo\nfOO\n");
    const out = await run(tool(), { pattern: "foo", ignoreCase: true });
    expect(out).toBe("a.txt:1: Foo\na.txt:2: fOO");
  });

  it("filters by glob", async () => {
    await writeFile(path.join(root, "a.txt"), "hit\n");
    await writeFile(path.join(root, "a.md"), "hit\n");
    const out = await run(tool(), { pattern: "hit", glob: "*.txt" });
    expect(out).toBe("a.txt:1: hit");
  });

  it("searches a subdirectory via path param", async () => {
    await mkdir(path.join(root, "sub"));
    await writeFile(path.join(root, "sub", "s.txt"), "needle\n");
    await writeFile(path.join(root, "top.txt"), "needle\n");
    const out = await run(tool(), { pattern: "needle", path: "sub" });
    expect(out).toBe("sub/s.txt:1: needle");
  });

  it("caps results at maxResults", async () => {
    const lines = Array.from({ length: 10 }, (_, i) => `m${i}`).join("\n");
    await writeFile(path.join(root, "many.txt"), `${lines}\n`);
    const out = await run(tool(), { pattern: "m\\d", maxResults: 3 });
    expect(out).toBe("many.txt:1: m0\nmany.txt:2: m1\nmany.txt:3: m2\n[truncated: more matches]");
  });

  it("emits context lines around matches", async () => {
    await writeFile(path.join(root, "c.txt"), "l1\nl2\nhit\nl4\nl5\n");
    const out = await run(tool(), { pattern: "hit", context: 1 });
    expect(out).toBe("c.txt-2- l2\nc.txt:3: hit\nc.txt-4- l4");
  });

  it("skips node_modules and binary files", async () => {
    await mkdir(path.join(root, "node_modules"));
    await writeFile(path.join(root, "node_modules", "n.txt"), "hit\n");
    const bin = Buffer.from([0x68, 0x69, 0x00, 0x74]);
    await writeFile(path.join(root, "b.dat"), bin);
    await writeFile(path.join(root, "t.txt"), "hit\n");
    const out = await run(tool(), { pattern: "hit" });
    expect(out).toBe("t.txt:1: hit");
  });

  it("returns a no-matches marker when nothing matched", async () => {
    await writeFile(path.join(root, "a.txt"), "zzz\n");
    const out = await run(tool(), { pattern: "qqq" });
    expect(out).toBe("(no matches)");
  });

  it("rejects invalid regex patterns", async () => {
    await expect(run(tool(), { pattern: "(((" })).rejects.toThrow(/pattern/i);
  });

  it("denies path outside the workspace", async () => {
    await expect(run(tool(), { pattern: "x", path: ".." })).rejects.toThrow(
      "Access denied: path is outside the workspace",
    );
  });

  it("never spawns an external process", async () => {
    const spy = vi.spyOn(childProcess, "spawn");
    await writeFile(path.join(root, "a.txt"), "hit\n");
    await run(tool(), { pattern: "hit" });
    expect(spy).not.toHaveBeenCalled();
  });
});
