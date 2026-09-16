import { readFile, symlink, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  canonicalizeWorkspaceRoot,
  countFileTokens,
  TOKENIZER_IDENTITY,
  TOKENIZER_VERSION,
  WorkspaceEscapeError,
} from "../../src/index.js";
import { cleanupTmpDir, makeTmpDir } from "../fixtures/tmp.js";

describe("countFileTokens", () => {
  let dir: string;
  let root: string;
  beforeEach(async () => {
    dir = await makeTmpDir();
    root = await canonicalizeWorkspaceRoot(dir);
  });
  afterEach(async () => {
    await cleanupTmpDir(dir);
  });

  it("returns a deterministic count with tokenizer identity", async () => {
    await writeFile(path.join(dir, "a.txt"), "hello world\n");
    const first = await countFileTokens({
      root,
      path: "a.txt",
      symlinkPolicy: "within-workspace",
    });
    const second = await countFileTokens({
      root,
      path: "a.txt",
      symlinkPolicy: "within-workspace",
    });
    expect(first.tokens).toBeGreaterThan(0);
    expect(first.tokens).toBe(second.tokens);
    expect(first.path).toBe("a.txt");
    expect(first.bytes).toBe(12);
    expect(first.tokenizer).toEqual(TOKENIZER_IDENTITY);
    expect(first.tokenizer.name).toBe("o200k_base");
    expect(first.tokenizer.implementation).toBe("js-tiktoken");
  });

  it("counts an empty file as zero tokens", async () => {
    await writeFile(path.join(dir, "empty.txt"), "");
    const r = await countFileTokens({ root, path: "empty.txt", symlinkPolicy: "deny" });
    expect(r.tokens).toBe(0);
    expect(r.bytes).toBe(0);
  });

  it("counts multibyte utf8 and unicode text deterministically", async () => {
    const text = "日本語のテキスト。🚀 emoji と éàü 混在。\n";
    await writeFile(path.join(dir, "uni.txt"), text, "utf8");
    const first = await countFileTokens({ root, path: "uni.txt", symlinkPolicy: "deny" });
    const second = await countFileTokens({ root, path: "uni.txt", symlinkPolicy: "deny" });
    expect(first.tokens).toBeGreaterThan(0);
    expect(first.tokens).toBe(second.tokens);
    expect(first.bytes).toBe(Buffer.byteLength(text, "utf8"));
  });

  it("counts a binary file deterministically via lossy utf8 decode", async () => {
    const buf = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x00, 0xff, 0xfe, 0x01]);
    await writeFile(path.join(dir, "bin.dat"), buf);
    const first = await countFileTokens({ root, path: "bin.dat", symlinkPolicy: "deny" });
    const second = await countFileTokens({ root, path: "bin.dat", symlinkPolicy: "deny" });
    expect(first.tokens).toBe(second.tokens);
    expect(first.bytes).toBe(8);
  });

  it("resolves absolute in-workspace paths to workspace-relative display", async () => {
    await writeFile(path.join(dir, "b.txt"), "abc");
    const r = await countFileTokens({
      root,
      path: path.join(root, "b.txt"),
      symlinkPolicy: "deny",
    });
    expect(r.path).toBe("b.txt");
  });

  it("rejects paths outside the workspace", async () => {
    await writeFile(path.join(dir, "in.txt"), "x");
    await expect(
      countFileTokens({ root, path: "../outside.txt", symlinkPolicy: "deny" }),
    ).rejects.toThrowError(WorkspaceEscapeError);
    await expect(
      countFileTokens({ root, path: "/etc/passwd", symlinkPolicy: "deny" }),
    ).rejects.toThrowError(WorkspaceEscapeError);
  });

  it("rejects symlinks escaping the workspace under within-workspace policy", async (t) => {
    const outside = await makeTmpDir();
    try {
      await writeFile(path.join(outside, "secret.txt"), "secret");
      await symlink(path.join(outside, "secret.txt"), path.join(dir, "link.txt"));
      await expect(
        countFileTokens({ root, path: "link.txt", symlinkPolicy: "within-workspace" }),
      ).rejects.toThrowError(WorkspaceEscapeError);
    } catch (e) {
      t.skip(`symlink not permitted on this platform: ${String(e)}`);
    } finally {
      await cleanupTmpDir(outside);
    }
  });

  it("propagates readFile failure for missing files", async () => {
    await expect(
      countFileTokens({ root, path: "missing.txt", symlinkPolicy: "deny" }),
    ).rejects.toThrowError();
  });
});

describe("tokenizer version", () => {
  it("matches the installed js-tiktoken version", async () => {
    const pkgPath = path.join(
      path.dirname(fileURLToPath(import.meta.url)),
      "..",
      "..",
      "node_modules",
      "js-tiktoken",
      "package.json",
    );
    const pkg = JSON.parse(await readFile(pkgPath, "utf8")) as { version: string };
    expect(TOKENIZER_VERSION).toBe(pkg.version);
    expect(TOKENIZER_IDENTITY.version).toBe(pkg.version);
  });
});
