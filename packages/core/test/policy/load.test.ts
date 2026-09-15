import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { CheaperError } from "../../src/errors.js";
import { loadPolicy } from "../../src/policy/load.js";
import { cleanupTmpDir, makeTmpDir } from "../fixtures/tmp.js";

const BUILTIN_POLICY = `version: 1
execution:
  timeout: 120s
  maxTurns: 8
  maxToolCalls: 64
tools:
  allow: [read, grep, find, ls, write, edit]
filesystem:
  symlinks: within-workspace
model:
  maxTier: medium
  tierFallback: none
output:
  maxTokens: 2000
  repairAttempts: 1
retry:
  transientAttempts: 1
`;

describe("loadPolicy", () => {
  let dir: string;
  let builtinPath: string;
  let userConfigDir: string;
  let projectDir: string;
  beforeEach(async () => {
    dir = await makeTmpDir();
    builtinPath = path.join(dir, "builtin-policy.yaml");
    userConfigDir = path.join(dir, "user");
    projectDir = path.join(dir, "proj", ".cheaper");
    await mkdir(userConfigDir, { recursive: true });
    await mkdir(projectDir, { recursive: true });
    await writeFile(builtinPath, BUILTIN_POLICY);
  });
  afterEach(async () => {
    await cleanupTmpDir(dir);
  });

  it("loads built-in policy alone when user/project absent", async () => {
    const policy = await loadPolicy({ builtinPolicyPath: builtinPath, userConfigDir, projectDir });
    expect(policy.execution.timeoutMs).toBe(120_000);
    expect(policy.execution.maxTurns).toBe(8);
    expect(policy.execution.maxToolCalls).toBe(64);
    expect(policy.tools.allow).toEqual(["read", "grep", "find", "ls", "write", "edit"]);
    expect(policy.filesystem.symlinks).toBe("within-workspace");
    expect(policy.model).toEqual({ maxTier: "medium", tierFallback: "none" });
    expect(policy.output).toEqual({ maxTokens: 2000, repairAttempts: 1 });
    expect(policy.retry.transientAttempts).toBe(1);
  });

  it("merges user partial override (deep merge + scalar replace)", async () => {
    await writeFile(
      path.join(userConfigDir, "policy.yaml"),
      "version: 1\nexecution:\n  timeout: 180s\n",
    );
    const policy = await loadPolicy({ builtinPolicyPath: builtinPath, userConfigDir, projectDir });
    expect(policy.execution.timeoutMs).toBe(180_000);
    expect(policy.execution.maxTurns).toBe(8);
  });

  it("project overrides user", async () => {
    await writeFile(
      path.join(userConfigDir, "policy.yaml"),
      "version: 1\nexecution:\n  timeout: 180s\n  maxTurns: 4\n",
    );
    await writeFile(
      path.join(projectDir, "policy.yaml"),
      "version: 1\nexecution:\n  maxTurns: 12\n",
    );
    const policy = await loadPolicy({ builtinPolicyPath: builtinPath, userConfigDir, projectDir });
    expect(policy.execution.timeoutMs).toBe(180_000);
    expect(policy.execution.maxTurns).toBe(12);
  });

  it("array replace via partial override", async () => {
    await writeFile(
      path.join(projectDir, "policy.yaml"),
      "version: 1\ntools:\n  allow: [read, grep]\n",
    );
    const policy = await loadPolicy({ builtinPolicyPath: builtinPath, userConfigDir, projectDir });
    expect(policy.tools.allow).toEqual(["read", "grep"]);
  });

  it("missing built-in policy → INVALID_RUNTIME_POLICY", async () => {
    await expect(
      loadPolicy({ builtinPolicyPath: path.join(dir, "nope.yaml"), userConfigDir, projectDir }),
    ).rejects.toMatchObject({ code: "INVALID_RUNTIME_POLICY" });
  });

  it("invalid user policy is not silently ignored", async () => {
    await writeFile(path.join(userConfigDir, "policy.yaml"), "version: 1\nbogus: true\n");
    await expect(
      loadPolicy({ builtinPolicyPath: builtinPath, userConfigDir, projectDir }),
    ).rejects.toMatchObject({ code: "INVALID_RUNTIME_POLICY" });
  });

  it("invalid project policy is not silently ignored", async () => {
    await writeFile(path.join(projectDir, "policy.yaml"), "execution:\n  timeout: 60s\n");
    await expect(
      loadPolicy({ builtinPolicyPath: builtinPath, userConfigDir, projectDir }),
    ).rejects.toMatchObject({ code: "INVALID_RUNTIME_POLICY" });
  });

  it("non-object yaml → INVALID_RUNTIME_POLICY", async () => {
    await writeFile(path.join(userConfigDir, "policy.yaml"), "- just\n- a\n- list\n");
    await expect(
      loadPolicy({ builtinPolicyPath: builtinPath, userConfigDir, projectDir }),
    ).rejects.toMatchObject({ code: "INVALID_RUNTIME_POLICY" });
  });

  it("unreadable optional policy (a directory) → INVALID_RUNTIME_POLICY read-failed", async () => {
    // policy.yaml が directory だと readFile は EISDIR で落ちる。ENOENT/ENOTDIR 以外の
    // 読み込み失敗は silent skip せず INVALID_RUNTIME_POLICY にする。
    await mkdir(path.join(userConfigDir, "policy.yaml"));
    try {
      await loadPolicy({ builtinPolicyPath: builtinPath, userConfigDir, projectDir });
      expect.unreachable();
    } catch (e) {
      const err = e as CheaperError;
      expect(err.code).toBe("INVALID_RUNTIME_POLICY");
      expect(err.details).toMatchObject({ reason: "read-failed" });
    }
  });

  it("ENOTDIR (config dir is a file) is still treated as absent", async () => {
    const fileAsDir = path.join(dir, "not-a-dir");
    await writeFile(fileAsDir, "x");
    const policy = await loadPolicy({
      builtinPolicyPath: builtinPath,
      userConfigDir: fileAsDir,
      projectDir,
    });
    expect(policy.execution.maxTurns).toBe(8);
  });

  it("effective policy is re-validated as complete after merge", async () => {
    await writeFile(
      path.join(projectDir, "policy.yaml"),
      "version: 1\nmodel:\n  maxTier: extreme\n",
    );
    try {
      await loadPolicy({ builtinPolicyPath: builtinPath, userConfigDir, projectDir });
      expect.unreachable();
    } catch (e) {
      expect((e as CheaperError).code).toBe("INVALID_RUNTIME_POLICY");
    }
  });
});

describe("packages/cli/policy.yaml (built-in canonical)", () => {
  const repoPolicyPath = path.resolve(
    path.dirname(fileURLToPath(import.meta.url)),
    "../../../cli/policy.yaml",
  );

  it("exists, satisfies complete schema, and normalizes to canonical values", async () => {
    const dir = await makeTmpDir();
    try {
      const policy = await loadPolicy({
        builtinPolicyPath: repoPolicyPath,
        userConfigDir: path.join(dir, "no-user"),
        projectDir: path.join(dir, "no-project"),
      });
      expect(policy).toEqual({
        execution: { timeoutMs: 120_000, maxTurns: 8, maxToolCalls: 64 },
        tools: { allow: ["read", "grep", "find", "ls", "write", "edit"] },
        filesystem: { symlinks: "within-workspace" },
        model: { maxTier: "medium", tierFallback: "none" },
        output: { maxTokens: 2000, repairAttempts: 1 },
        retry: { transientAttempts: 1 },
      });
    } finally {
      await cleanupTmpDir(dir);
    }
  });
});
