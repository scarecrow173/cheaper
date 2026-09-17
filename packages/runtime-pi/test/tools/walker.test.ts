import { mkdir, mkdtemp, realpath, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { createPathDenylist } from "@cheaper/core";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { sniffTextFile, type WalkIssue, walkFiles } from "../../src/tools/walker.js";

let dir: string;
let root: string;

beforeEach(async () => {
  dir = await mkdtemp(path.join(tmpdir(), "cheaper-walk-"));
  root = await realpath(dir);
});
afterEach(async () => {
  await rm(dir, { recursive: true, force: true });
});

async function collect(
  startAbs: string,
  policy: "deny" | "within-workspace" = "deny",
  onIssue?: (issue: WalkIssue) => void,
) {
  const out: string[] = [];
  for await (const f of walkFiles({ root, symlinkPolicy: policy }, startAbs, onIssue)) {
    out.push(f.display);
  }
  return out.sort();
}

describe("walkFiles", () => {
  it("yields files recursively with workspace-relative POSIX display paths", async () => {
    await mkdir(path.join(root, "sub", "deep"), { recursive: true });
    await writeFile(path.join(root, "top.txt"), "x");
    await writeFile(path.join(root, "sub", "mid.txt"), "x");
    await writeFile(path.join(root, "sub", "deep", "low.txt"), "x");
    const files = await collect(root);
    expect(files).toEqual(["sub/deep/low.txt", "sub/mid.txt", "top.txt"]);
  });

  it("yields a single file when start is a file", async () => {
    await writeFile(path.join(root, "one.txt"), "x");
    expect(await collect(path.join(root, "one.txt"))).toEqual(["one.txt"]);
  });

  it("rejects a start path outside the workspace", async () => {
    await expect(collect(path.join(root, ".."))).rejects.toThrow(/outside|escape/i);
  });

  it("skips .git and node_modules directories", async () => {
    await mkdir(path.join(root, ".git"), { recursive: true });
    await mkdir(path.join(root, "node_modules", "pkg"), { recursive: true });
    await writeFile(path.join(root, ".git", "config"), "x");
    await writeFile(path.join(root, "node_modules", "pkg", "index.js"), "x");
    await writeFile(path.join(root, "src.txt"), "x");
    expect(await collect(root)).toEqual(["src.txt"]);
  });

  it("yields files regardless of size (over 1MB)", async () => {
    await writeFile(path.join(root, "big.bin"), Buffer.alloc(1024 * 1024 + 1, 0x61));
    await writeFile(path.join(root, "small.txt"), "x");
    expect(await collect(root)).toEqual(["big.bin", "small.txt"]);
  });

  it("yields binary files (path-only enumeration)", async () => {
    const bin = Buffer.alloc(64, 0x61);
    bin[10] = 0;
    await writeFile(path.join(root, "bin.dat"), bin);
    await writeFile(path.join(root, "text.txt"), "no nul here");
    expect(await collect(root)).toEqual(["bin.dat", "text.txt"]);
  });

  it("reports dangling symlinks via onIssue under within-workspace", async (ctx) => {
    try {
      await symlink(path.join(root, "no-such-target"), path.join(root, "dangling"), "junction");
    } catch (e) {
      console.warn(`symlink unavailable, skipping: ${String(e)}`);
      ctx.skip();
    }
    await writeFile(path.join(root, "f.txt"), "x");
    const issues: WalkIssue[] = [];
    const files = await collect(root, "within-workspace", (i) => issues.push(i));
    expect(files).toEqual(["f.txt"]);
    expect(issues).toEqual([{ display: "dangling", reason: "unreadable" }]);
  });

  it("never surfaces denied paths in results or onIssue", async () => {
    // built-in deny pattern（.env）に match する file を置く
    await writeFile(path.join(root, ".env"), "SECRET=1\n");
    await writeFile(path.join(root, "ok.txt"), "x");
    const issues: WalkIssue[] = [];
    const out: string[] = [];
    const opts = { root, symlinkPolicy: "deny" as const, denylist: createPathDenylist() };
    for await (const f of walkFiles(opts, root, (i) => issues.push(i))) {
      out.push(f.display);
    }
    expect(out).toEqual(["ok.txt"]);
    expect(issues).toEqual([]);
  });

  it("does not follow symlinks under deny policy", async (ctx) => {
    const outside = await mkdtemp(path.join(tmpdir(), "cheaper-walk-out-"));
    try {
      await mkdir(path.join(root, "real-dir"), { recursive: true });
      await writeFile(path.join(root, "real-dir", "f.txt"), "x");
      try {
        await symlink(path.join(root, "real-dir"), path.join(root, "link-dir"), "junction");
        await symlink(path.join(root, "real-dir", "f.txt"), path.join(root, "link-file.txt"));
      } catch (e) {
        console.warn(`symlink unavailable, skipping: ${String(e)}`);
        ctx.skip();
      }
      const files = await collect(root, "deny");
      expect(files).toEqual(["real-dir/f.txt"]);
      void outside;
    } finally {
      await rm(outside, { recursive: true, force: true });
    }
  });

  it("follows inside symlinks and skips outside symlinks under within-workspace", async (ctx) => {
    const outside = await mkdtemp(path.join(tmpdir(), "cheaper-walk-out-"));
    try {
      await writeFile(path.join(outside, "out.txt"), "x");
      await mkdir(path.join(root, "real-dir"), { recursive: true });
      await writeFile(path.join(root, "real-dir", "f.txt"), "x");
      try {
        await symlink(path.join(root, "real-dir"), path.join(root, "link-in"), "junction");
        await symlink(outside, path.join(root, "link-out"), "junction");
      } catch (e) {
        console.warn(`symlink unavailable, skipping: ${String(e)}`);
        ctx.skip();
      }
      const files = await collect(root, "within-workspace");
      expect(files).toContain("real-dir/f.txt");
      // inside symlink dir は辿られる（display は symlink 経由 path）
      expect(files).toContain("link-in/f.txt");
      // outside 指しは skip
      expect(files.some((f) => f.startsWith("link-out"))).toBe(false);
    } finally {
      await rm(outside, { recursive: true, force: true });
    }
  });

  it("does not loop on symlink cycles under within-workspace", async (ctx) => {
    try {
      await symlink(root, path.join(root, "self"), "junction");
    } catch (e) {
      console.warn(`symlink unavailable, skipping: ${String(e)}`);
      ctx.skip();
    }
    await writeFile(path.join(root, "f.txt"), "x");
    const files = await collect(root, "within-workspace");
    expect(files).toEqual(["f.txt"]);
  });
});

describe("sniffTextFile", () => {
  it("returns text for a plain text file", async () => {
    await writeFile(path.join(root, "t.txt"), "hello\n");
    expect(await sniffTextFile(path.join(root, "t.txt"))).toBe("text");
  });

  it("returns binary when NUL appears within the first 8KiB", async () => {
    const bin = Buffer.alloc(64, 0x61);
    bin[10] = 0;
    await writeFile(path.join(root, "b.dat"), bin);
    expect(await sniffTextFile(path.join(root, "b.dat"))).toBe("binary");
  });

  it("returns text when NUL exists only beyond the first 8KiB (bounded sniff)", async () => {
    const buf = Buffer.alloc(10 * 1024, 0x61);
    buf[9 * 1024] = 0;
    await writeFile(path.join(root, "late-nul.bin"), buf);
    expect(await sniffTextFile(path.join(root, "late-nul.bin"))).toBe("text");
  });

  it("returns unreadable for a missing file", async () => {
    expect(await sniffTextFile(path.join(root, "missing.txt"))).toBe("unreadable");
  });
});
