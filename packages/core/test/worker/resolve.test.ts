import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { ConfigDirs } from "../../src/config/dirs.js";
import type { CheaperError } from "../../src/errors.js";
import { resolveWorker, resolveWorkerFile } from "../../src/worker/resolve.js";
import { cleanupTmpDir, makeTmpDir } from "../fixtures/tmp.js";

function workerMd(name: string): string {
  return `---
name: ${JSON.stringify(name)}
description: "d"
tools: ["read"]
cheaper: {"model":{"tier":"low"},"output":{"schema":{"inline":{"type":"object"}}}}
---
Body.
`;
}

describe("resolveWorker / resolveWorkerFile", () => {
  let dir: string;
  let dirs: ConfigDirs;
  beforeEach(async () => {
    dir = await makeTmpDir();
    dirs = {
      projectDir: path.join(dir, "proj", ".cheaper"),
      userConfigDir: path.join(dir, "user"),
      builtinDir: path.join(dir, "builtin"),
    };
    await mkdir(path.join(dirs.projectDir, "workers"), { recursive: true });
    await mkdir(path.join(dirs.userConfigDir, "workers"), { recursive: true });
    await mkdir(path.join(dirs.builtinDir, "workers"), { recursive: true });
  });
  afterEach(async () => {
    await cleanupTmpDir(dir);
  });

  it("resolves from project first", async () => {
    await writeFile(path.join(dirs.projectDir, "workers", "w.md"), workerMd("w"));
    await writeFile(path.join(dirs.userConfigDir, "workers", "w.md"), workerMd("w"));
    const def = await resolveWorker("w", dirs);
    expect(def.origin).toBe("project");
    expect(def.sourcePath).toBe(path.join(dirs.projectDir, "workers", "w.md"));
  });

  it("falls back to user then builtin when absent", async () => {
    await writeFile(path.join(dirs.userConfigDir, "workers", "w.md"), workerMd("w"));
    expect((await resolveWorker("w", dirs)).origin).toBe("user");
    await writeFile(path.join(dirs.builtinDir, "workers", "b.md"), workerMd("b"));
    expect((await resolveWorker("b", dirs)).origin).toBe("builtin");
  });

  it("does not fall back when existing file is invalid", async () => {
    await writeFile(path.join(dirs.projectDir, "workers", "w.md"), "no frontmatter");
    await writeFile(path.join(dirs.userConfigDir, "workers", "w.md"), workerMd("w"));
    await expect(resolveWorker("w", dirs)).rejects.toMatchObject({
      code: "INVALID_WORKER_DEFINITION",
    });
  });

  it("throws WORKER_NOT_FOUND with searched paths", async () => {
    try {
      await resolveWorker("missing", dirs);
      expect.unreachable();
    } catch (e) {
      const err = e as CheaperError;
      expect(err.code).toBe("WORKER_NOT_FOUND");
      const searched = err.details?.searched as string[];
      expect(searched).toHaveLength(3);
      expect(searched[0]).toContain("proj");
    }
  });

  it("rejects invalid worker names before searching", async () => {
    await expect(resolveWorker("Bad", dirs)).rejects.toMatchObject({
      code: "INVALID_WORKER_NAME",
    });
  });

  it("resolves explicit file with origin explicit", async () => {
    const p = path.join(dir, "explicit-w.md");
    await writeFile(p, workerMd("explicit-w"));
    const def = await resolveWorkerFile("explicit-w.md", dir);
    expect(def.origin).toBe("explicit");
    expect(def.name).toBe("explicit-w");
  });

  it("explicit file: missing → WORKER_NOT_FOUND, mismatch → WORKER_NAME_MISMATCH", async () => {
    await expect(resolveWorkerFile("nope.md", dir)).rejects.toMatchObject({
      code: "WORKER_NOT_FOUND",
    });
    const p = path.join(dir, "x.md");
    await writeFile(p, workerMd("y"));
    await expect(resolveWorkerFile(p, dir)).rejects.toMatchObject({
      code: "WORKER_NAME_MISMATCH",
    });
  });
});
