import { mkdir, realpath, symlink, writeFile } from "node:fs/promises";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it, type TestContext } from "vitest";
import type { CheaperError } from "../../src/errors.js";
import {
  canonicalizeWorkspaceRoot,
  isInsideWorkspaceRoot,
  resolveWorkspacePath,
  WorkspaceEscapeError,
} from "../../src/workspace/paths.js";
import { cleanupTmpDir, makeTmpDir } from "../fixtures/tmp.js";

describe("canonicalizeWorkspaceRoot", () => {
  let dir: string;
  beforeEach(async () => {
    dir = await makeTmpDir();
  });
  afterEach(async () => {
    await cleanupTmpDir(dir);
  });

  it("returns the realpath of an existing directory", async () => {
    const root = await canonicalizeWorkspaceRoot(dir);
    expect(root).toBe(await realpath(dir));
  });

  it("rejects a non-existent cwd with INVALID_REQUEST", async () => {
    try {
      await canonicalizeWorkspaceRoot(path.join(dir, "nope"));
      expect.unreachable();
    } catch (e) {
      const err = e as CheaperError;
      expect(err.code).toBe("INVALID_REQUEST");
      expect(err.exitCode).toBe(2);
    }
  });
});

describe("resolveWorkspacePath", () => {
  let dir: string;
  let root: string;
  beforeEach(async () => {
    dir = await makeTmpDir();
    await mkdir(path.join(dir, "sub", "deep"), { recursive: true });
    await writeFile(path.join(dir, "sub", "file.txt"), "x");
    root = await canonicalizeWorkspaceRoot(dir);
  });
  afterEach(async () => {
    await cleanupTmpDir(dir);
  });

  it("rejects input containing NUL", async () => {
    await expect(resolveWorkspacePath(root, "a\0b", "deny")).rejects.toBeInstanceOf(
      WorkspaceEscapeError,
    );
  });

  it("rejects ../ escape", async () => {
    await expect(resolveWorkspacePath(root, "../outside.txt", "deny")).rejects.toBeInstanceOf(
      WorkspaceEscapeError,
    );
    await expect(
      resolveWorkspacePath(root, "sub/../../outside.txt", "within-workspace"),
    ).rejects.toBeInstanceOf(WorkspaceEscapeError);
  });

  it("rejects absolute path outside root", async () => {
    const outside = path.join(path.dirname(root), "definitely-outside-cheaper.txt");
    await expect(resolveWorkspacePath(root, outside, "deny")).rejects.toBeInstanceOf(
      WorkspaceEscapeError,
    );
  });

  it("accepts absolute path inside root", async () => {
    const inside = path.join(root, "sub", "file.txt");
    const resolved = await resolveWorkspacePath(root, inside, "deny");
    expect(resolved.display).toBe("sub/file.txt");
  });

  it("does not expand ~ (treated as a literal in-workspace path)", async () => {
    const resolved = await resolveWorkspacePath(root, "~/x", "deny");
    expect(resolved.real).toBe(path.join(root, "~", "x"));
    expect(resolved.display).toBe("~/x");
  });

  it("allows non-existent tail for create (deepest existing ancestor inside root)", async () => {
    const resolved = await resolveWorkspacePath(root, "sub/deep/new-file.txt", "deny");
    expect(resolved.real).toBe(path.join(root, "sub", "deep", "new-file.txt"));
    expect(resolved.display).toBe("sub/deep/new-file.txt");
  });

  it("allows a fully non-existent new subtree under root", async () => {
    const resolved = await resolveWorkspacePath(root, "a/b/c.txt", "deny");
    expect(resolved.real).toBe(path.join(root, "a", "b", "c.txt"));
  });

  it("normalizes . and interior .. within workspace", async () => {
    const resolved = await resolveWorkspacePath(root, "sub/../sub/./file.txt", "deny");
    expect(resolved.display).toBe("sub/file.txt");
    const dot = await resolveWorkspacePath(root, ".", "deny");
    expect(dot.real).toBe(root);
    expect(dot.display).toBe(".");
  });
});

describe("isInsideWorkspaceRoot", () => {
  const root = path.join("tmp", "work");

  it("accepts the root itself and descendants", () => {
    expect(isInsideWorkspaceRoot(root, root)).toBe(true);
    expect(isInsideWorkspaceRoot(root, path.join(root, "a"))).toBe(true);
  });

  it("rejects siblings and prefix-confusable paths", () => {
    expect(isInsideWorkspaceRoot(root, path.join("tmp", "workx", "a"))).toBe(false);
    expect(isInsideWorkspaceRoot(root, path.join("tmp", "other"))).toBe(false);
  });

  it("is case-sensitive by default and with caseInsensitive: false", () => {
    const candidate = path.join("tmp", "Work", "secret");
    expect(isInsideWorkspaceRoot(root, candidate)).toBe(false);
    expect(isInsideWorkspaceRoot(root, candidate, { caseInsensitive: false })).toBe(false);
  });

  it("accepts case-differing descendants with caseInsensitive: true", () => {
    expect(
      isInsideWorkspaceRoot(root, path.join("tmp", "Work", "secret"), {
        caseInsensitive: true,
      }),
    ).toBe(true);
  });
});

describe("resolveWorkspacePath case-sensitivity", () => {
  // case-sensitive FS でのみ成立する迂回パターン。win32 では FS 自体が
  // case-insensitive なため区別しない（比較も caseInsensitive: true で動く）。
  it.skipIf(process.platform === "win32")(
    "treats a differently-cased real dir sibling as outside the workspace",
    async (ctx) => {
      const dir = await makeTmpDir();
      try {
        const ws = path.join(dir, "Work");
        const sibling = path.join(dir, "work");
        await mkdir(ws, { recursive: true });
        await mkdir(sibling, { recursive: true });
        // macOS 等の case-insensitive FS では同一 dir になるため skip
        if ((await realpath(ws)) === (await realpath(sibling))) ctx.skip();
        await writeFile(path.join(sibling, "secret.txt"), "s");
        const root = await canonicalizeWorkspaceRoot(ws);
        await expect(
          resolveWorkspacePath(root, path.join("..", "work", "secret.txt"), "deny"),
        ).rejects.toBeInstanceOf(WorkspaceEscapeError);
      } finally {
        await cleanupTmpDir(dir);
      }
    },
  );

  it.skipIf(process.platform === "win32")(
    "rejects a within-workspace symlink resolving to a differently-cased sibling",
    async (ctx) => {
      const dir = await makeTmpDir();
      try {
        const ws = path.join(dir, "Work");
        const sibling = path.join(dir, "work");
        await mkdir(ws, { recursive: true });
        await mkdir(sibling, { recursive: true });
        if ((await realpath(ws)) === (await realpath(sibling))) ctx.skip();
        await writeFile(path.join(sibling, "secret.txt"), "s");
        try {
          await symlink(sibling, path.join(ws, "link"), "dir");
        } catch {
          ctx.skip(); // symlink 非対応環境
        }
        const root = await canonicalizeWorkspaceRoot(ws);
        await expect(
          resolveWorkspacePath(root, path.join("link", "secret.txt"), "within-workspace"),
        ).rejects.toBeInstanceOf(WorkspaceEscapeError);
      } finally {
        await cleanupTmpDir(dir);
      }
    },
  );
});

describe("resolveWorkspacePath symlink policy", () => {
  let dir: string;
  let root: string;
  let symlinkOk = false;

  function skipIfNoSymlink(ctx: TestContext) {
    if (!symlinkOk) {
      console.warn("symlink tests skipped: could not create symlink (EPERM or unsupported)");
      ctx.skip();
    }
  }

  beforeEach(async () => {
    dir = await makeTmpDir();
    // setup: <dir>/ws (workspace), <dir>/outside (escape target)
    const ws = path.join(dir, "ws");
    const outside = path.join(dir, "outside");
    await mkdir(path.join(ws, "real"), { recursive: true });
    await mkdir(outside, { recursive: true });
    await writeFile(path.join(ws, "real", "in.txt"), "in");
    await writeFile(path.join(outside, "out.txt"), "out");
    root = await canonicalizeWorkspaceRoot(ws);
    try {
      await symlink(outside, path.join(ws, "link-out"), "dir");
      await symlink(path.join(ws, "real"), path.join(ws, "link-in"), "dir");
      await symlink(path.join(outside, "out.txt"), path.join(ws, "link-file-out"), "file");
      await symlink(path.join(ws, "real", "in.txt"), path.join(ws, "link-file-in"), "file");
      symlinkOk = true;
    } catch {
      symlinkOk = false;
    }
  });
  afterEach(async () => {
    await cleanupTmpDir(dir);
  });

  it("deny policy rejects in-workspace symlink", async (ctx) => {
    skipIfNoSymlink(ctx);
    await expect(resolveWorkspacePath(root, "link-in/in.txt", "deny")).rejects.toBeInstanceOf(
      WorkspaceEscapeError,
    );
  });

  it("within-workspace rejects symlink to outside dir", async (ctx) => {
    skipIfNoSymlink(ctx);
    await expect(
      resolveWorkspacePath(root, "link-out/out.txt", "within-workspace"),
    ).rejects.toBeInstanceOf(WorkspaceEscapeError);
  });

  it("within-workspace rejects symlink to outside file", async (ctx) => {
    skipIfNoSymlink(ctx);
    await expect(
      resolveWorkspacePath(root, "link-file-out", "within-workspace"),
    ).rejects.toBeInstanceOf(WorkspaceEscapeError);
  });

  it("within-workspace allows symlink to inside dir", async (ctx) => {
    skipIfNoSymlink(ctx);
    const resolved = await resolveWorkspacePath(root, "link-in/in.txt", "within-workspace");
    expect(resolved.real).toBe(path.join(root, "real", "in.txt"));
    expect(resolved.display).toBe("real/in.txt");
  });

  it("within-workspace allows symlink to inside file", async (ctx) => {
    skipIfNoSymlink(ctx);
    const resolved = await resolveWorkspacePath(root, "link-file-in", "within-workspace");
    expect(resolved.real).toBe(path.join(root, "real", "in.txt"));
  });

  it("within-workspace allows non-existent tail under inside symlinked dir", async (ctx) => {
    skipIfNoSymlink(ctx);
    const resolved = await resolveWorkspacePath(root, "link-in/new-file.txt", "within-workspace");
    expect(resolved.real).toBe(path.join(root, "real", "new-file.txt"));
  });
});
