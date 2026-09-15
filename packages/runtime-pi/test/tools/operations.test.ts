import {
  mkdtemp,
  readdir,
  readFile,
  realpath,
  rm,
  stat,
  symlink,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { MutationJournal } from "../../src/journal.js";
import {
  createEditOperations,
  createLsOperations,
  createReadOperations,
  createWriteOperations,
} from "../../src/tools/operations.js";

const DENIED = "Access denied: path is outside the workspace";

let dir: string;
let root: string;

beforeEach(async () => {
  dir = await mkdtemp(path.join(tmpdir(), "cheaper-ops-"));
  root = await realpath(dir);
});
afterEach(async () => {
  await rm(dir, { recursive: true, force: true });
});

function ops(policy: "deny" | "within-workspace" = "deny") {
  const journal = new MutationJournal();
  const opts = { root, symlinkPolicy: policy, journal };
  return {
    journal,
    read: createReadOperations(opts),
    write: createWriteOperations(opts),
    edit: createEditOperations(opts),
    ls: createLsOperations(opts),
  };
}

describe("boundary-checked operations", () => {
  it("reads a file inside the workspace", async () => {
    const { read } = ops();
    await writeFile(path.join(root, "a.txt"), "hello", "utf8");
    const buf = await read.readFile(path.join(root, "a.txt"));
    expect(buf.toString("utf8")).toBe("hello");
  });

  it("denies read of ../ and absolute paths outside the workspace", async () => {
    const { read } = ops();
    await expect(read.access(path.join(root, "..", "x.txt"))).rejects.toThrow(DENIED);
    await expect(read.readFile(path.join(root, "..", "x.txt"))).rejects.toThrow(DENIED);
    await expect(read.readFile(path.join(tmpdir(), "outside-of-ws.txt"))).rejects.toThrow(DENIED);
  });

  it("denies write/mkdir outside the workspace", async () => {
    const { write } = ops();
    await expect(write.writeFile(path.join(root, "..", "evil.txt"), "x")).rejects.toThrow(DENIED);
    await expect(write.mkdir(path.join(root, "..", "evil-dir"))).rejects.toThrow(DENIED);
    await expect(write.mkdir(path.join(tmpdir(), "abs-outside"))).rejects.toThrow(DENIED);
  });

  it("creates a missing parent dir inside the workspace via mkdir", async () => {
    const { write } = ops();
    const target = path.join(root, "sub", "deep");
    await write.mkdir(target);
    expect((await stat(target)).isDirectory()).toBe(true);
  });

  it("writes a new file atomically and records created in journal", async () => {
    const { write, journal } = ops();
    const target = path.join(root, "created.txt");
    await write.writeFile(target, "one\ntwo\n");
    expect(await readFile(target, "utf8")).toBe("one\ntwo\n");
    const changes = await journal.finalize();
    expect(changes.files).toEqual([
      { path: "created.txt", kind: "created", linesAdded: 2, linesDeleted: 0 },
    ]);
  });

  it("records modified content in journal on overwrite", async () => {
    const { write, journal } = ops();
    const target = path.join(root, "mod.txt");
    await writeFile(target, "old\n", "utf8");
    await write.writeFile(target, "new\nline\n");
    const changes = await journal.finalize();
    expect(changes.files).toEqual([
      { path: "mod.txt", kind: "modified", linesAdded: 2, linesDeleted: 1 },
    ]);
  });

  it("leaves no tmp files behind after write/edit", async () => {
    const { write, edit } = ops();
    const target = path.join(root, "f.txt");
    await write.writeFile(target, "v1\n");
    await edit.writeFile(target, "v2\n");
    const entries = await readdir(root);
    expect(entries.filter((e) => e.includes("cheaper-tmp"))).toEqual([]);
    expect(await readFile(target, "utf8")).toBe("v2\n");
  });

  it("edit operations read/access/write inside the workspace", async () => {
    const { edit } = ops();
    const target = path.join(root, "e.txt");
    await writeFile(target, "abc", "utf8");
    await edit.access(target);
    expect((await edit.readFile(target)).toString("utf8")).toBe("abc");
  });

  it("denies edit operations outside the workspace", async () => {
    const { edit } = ops();
    const outside = path.join(root, "..", "e.txt");
    await expect(edit.access(outside)).rejects.toThrow(DENIED);
    await expect(edit.readFile(outside)).rejects.toThrow(DENIED);
    await expect(edit.writeFile(outside, "x")).rejects.toThrow(DENIED);
  });

  it("ls operations work inside and deny outside", async () => {
    const { ls } = ops();
    await writeFile(path.join(root, "l.txt"), "x", "utf8");
    expect(await ls.exists(root)).toBe(true);
    expect((await ls.stat(root)).isDirectory()).toBe(true);
    expect(await ls.readdir(root)).toContain("l.txt");
    await expect(ls.exists(path.join(root, ".."))).rejects.toThrow(DENIED);
    await expect(ls.readdir(path.join(root, ".."))).rejects.toThrow(DENIED);
  });

  it("returns false for exists() on missing paths inside the workspace", async () => {
    const { ls } = ops();
    expect(await ls.exists(path.join(root, "nope"))).toBe(false);
  });

  it("denies symlink escapes under deny policy", async (ctx) => {
    const outside = await mkdtemp(path.join(tmpdir(), "cheaper-outside-"));
    try {
      const link = path.join(root, "link-out");
      try {
        await symlink(outside, link, "junction");
      } catch (e) {
        console.warn(`symlink unavailable, skipping: ${String(e)}`);
        ctx.skip();
      }
      const { read } = ops("deny");
      await expect(read.readFile(path.join(link, "f.txt"))).rejects.toThrow(DENIED);
    } finally {
      await rm(outside, { recursive: true, force: true });
    }
  });

  it("within-workspace policy allows inside symlinks and denies outside targets", async (ctx) => {
    const outside = await mkdtemp(path.join(tmpdir(), "cheaper-outside-"));
    try {
      const inner = path.join(root, "inner");
      await writeFile(path.join(root, "target.txt"), "t", "utf8");
      const linkIn = path.join(root, "link-in.txt");
      const linkOut = path.join(root, "link-out.txt");
      try {
        await symlink(path.join(root, "target.txt"), linkIn);
        await symlink(path.join(outside, "secret.txt"), linkOut);
        await mkdirSafe(inner);
      } catch (e) {
        console.warn(`symlink unavailable, skipping: ${String(e)}`);
        ctx.skip();
      }
      const { read } = ops("within-workspace");
      expect((await read.readFile(linkIn)).toString("utf8")).toBe("t");
      await expect(read.readFile(linkOut)).rejects.toThrow(DENIED);
    } finally {
      await rm(outside, { recursive: true, force: true });
    }
  });
});

async function mkdirSafe(p: string): Promise<void> {
  const { mkdir } = await import("node:fs/promises");
  await mkdir(p, { recursive: true });
}
