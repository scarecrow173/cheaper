import childProcess from "node:child_process";
import { mkdir, mkdtemp, realpath, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createFindTool } from "../../src/tools/find.js";

// path-only 化の検証用: open/readFile を呼び出し計数だけする spy に差し替える
// （実装は actual に透過。ESM namespace は spyOn 不可のため module mock を使う）。
vi.mock("node:fs/promises", async (importOriginal) => {
  const actual = await importOriginal<typeof import("node:fs/promises")>();
  return { ...actual, open: vi.fn(actual.open), readFile: vi.fn(actual.readFile) };
});

let dir: string;
let root: string;

beforeEach(async () => {
  dir = await mkdtemp(path.join(tmpdir(), "cheaper-find-"));
  root = await realpath(dir);
});
afterEach(async () => {
  await rm(dir, { recursive: true, force: true });
  vi.restoreAllMocks();
});

function tool(policy: "deny" | "within-workspace" = "deny") {
  return createFindTool({ root, symlinkPolicy: policy });
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

describe("find tool", () => {
  it("lists workspace-relative paths matching a glob", async () => {
    await mkdir(path.join(root, "sub"));
    await writeFile(path.join(root, "a.txt"), "x");
    await writeFile(path.join(root, "sub", "b.txt"), "x");
    await writeFile(path.join(root, "c.md"), "x");
    const out = await run(tool(), { pattern: "**/*.txt" });
    expect(out).toBe("a.txt\nsub/b.txt");
  });

  it("matches top-level glob without crossing directories", async () => {
    await mkdir(path.join(root, "sub"));
    await writeFile(path.join(root, "a.txt"), "x");
    await writeFile(path.join(root, "sub", "b.txt"), "x");
    const out = await run(tool(), { pattern: "*.txt" });
    expect(out).toBe("a.txt");
  });

  it("scopes search to path param", async () => {
    await mkdir(path.join(root, "sub"));
    await writeFile(path.join(root, "a.txt"), "x");
    await writeFile(path.join(root, "sub", "b.txt"), "x");
    const out = await run(tool(), { pattern: "**/*.txt", path: "sub" });
    expect(out).toBe("sub/b.txt");
  });

  it("caps results at maxResults", async () => {
    for (let i = 0; i < 5; i++) await writeFile(path.join(root, `f${i}.txt`), "x");
    const out = await run(tool(), { pattern: "**/*.txt", maxResults: 2 });
    expect(out.split("\n")).toHaveLength(3); // 2 entries + truncation note
    expect(out).toContain("[truncated");
  });

  it("skips .git and node_modules", async () => {
    await mkdir(path.join(root, ".git"));
    await mkdir(path.join(root, "node_modules"));
    await writeFile(path.join(root, ".git", "x.txt"), "x");
    await writeFile(path.join(root, "node_modules", "y.txt"), "x");
    await writeFile(path.join(root, "ok.txt"), "x");
    const out = await run(tool(), { pattern: "**/*.txt" });
    expect(out).toBe("ok.txt");
  });

  it("denies path outside the workspace", async () => {
    await expect(run(tool(), { pattern: "*", path: ".." })).rejects.toThrow(
      "Access denied: path is outside the workspace",
    );
  });

  it("lists files regardless of size or content", async () => {
    await writeFile(path.join(root, "big.txt"), Buffer.alloc(1024 * 1024 + 1, 0x61));
    const bin = Buffer.alloc(64, 0x61);
    bin[10] = 0;
    await writeFile(path.join(root, "bin.dat"), bin);
    await writeFile(path.join(root, "small.txt"), "x");
    const out = await run(tool(), { pattern: "**/*" });
    expect(out).toContain("big.txt");
    expect(out).toContain("bin.dat");
    expect(out).toContain("small.txt");
  });

  it("performs no content I/O on enumerated files (path-only)", async () => {
    const fsp = await import("node:fs/promises");
    vi.mocked(fsp.open).mockClear();
    vi.mocked(fsp.readFile).mockClear();
    await writeFile(path.join(root, "a.txt"), "x");
    const out = await run(tool(), { pattern: "*" });
    // 列挙自体が成立している（空 walk による vacuous pass ではない）こと
    expect(out).toBe("a.txt");
    expect(fsp.open).not.toHaveBeenCalled();
    expect(fsp.readFile).not.toHaveBeenCalled();
  });

  it("reports enumeration failures in a bounded skipped footer", async (ctx) => {
    try {
      await symlink(path.join(root, "no-such-target"), path.join(root, "dangling"), "junction");
    } catch (e) {
      console.warn(`symlink unavailable, skipping: ${String(e)}`);
      ctx.skip();
    }
    await writeFile(path.join(root, "ok.txt"), "x");
    const out = await run(tool("within-workspace"), { pattern: "**/*" });
    expect(out).toBe("ok.txt\n[skipped: unreadable — dangling]");
  });

  it("emits the skipped footer after (no matches)", async (ctx) => {
    try {
      await symlink(path.join(root, "no-such-target"), path.join(root, "dangling"), "junction");
    } catch (e) {
      console.warn(`symlink unavailable, skipping: ${String(e)}`);
      ctx.skip();
    }
    const out = await run(tool("within-workspace"), { pattern: "**/*.txt" });
    expect(out).toBe("(no matches)\n[skipped: unreadable — dangling]");
  });

  it("never spawns an external process", async () => {
    const spy = vi.spyOn(childProcess, "spawn");
    await writeFile(path.join(root, "a.txt"), "x");
    await run(tool(), { pattern: "*" });
    expect(spy).not.toHaveBeenCalled();
  });
});
