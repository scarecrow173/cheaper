import { mkdir, mkdtemp, realpath, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import * as core from "../../src/index.js";

const task = () => ({ command: ["node", "check.mjs"], inputs: ["src/**/*.ts"], outputs: [] });
const raw = (process?: unknown) => ({
  version: 1,
  execution: { timeout: "120s", maxTurns: 8, maxToolCalls: 64 },
  tools: { allow: ["read"] },
  filesystem: { symlinks: "deny" },
  model: { maxTier: "medium", tierFallback: "none" },
  output: { maxTokens: 2000, repairAttempts: 1 },
  retry: { transientAttempts: 1 },
  ...(process === undefined ? {} : { process }),
});
const dirs: string[] = [];
async function temporary() {
  const root = await realpath(await mkdtemp(path.join(process.cwd(), ".tmp-process-policy-")));
  dirs.push(root);
  return root;
}
afterEach(async () => {
  for (const root of dirs.splice(0)) {
    expect(path.dirname(root)).toBe(await realpath(process.cwd()));
    await rm(root, { recursive: true, force: true });
  }
});

describe("named process policy", () => {
  it("normalizes legacy omission and snapshots the configured task", () => {
    expect(core.normalizePolicy(core.validatePolicySchema(raw(), "complete")).process).toEqual({
      maxConcurrent: 1,
      tasks: {},
    });
    const configured = { maxConcurrent: 3, tasks: { check: task() } };
    const resolved = core.resolveProcessPolicy(configured);
    configured.tasks.check.command[1] = "changed.mjs";
    expect(resolved.tasks.check?.command).toEqual(["node", "check.mjs"]);
    expect(resolved.tasks.check).toMatchObject({ cwd: ".", env: {} });
  });

  it("requires complete process fields but preserves partial omission", () => {
    expect(() =>
      core.validatePolicySchema(raw({ maxConcurrent: 2, tasks: { check: task() } }), "complete"),
    ).not.toThrow();
    for (const partial of [{}, { maxConcurrent: 2 }, { tasks: { check: task() } }]) {
      expect(() => core.validatePolicySchema(raw(partial), "complete")).toThrow();
      expect(core.validatePolicySchema({ version: 1, process: partial }, "partial")).toBeDefined();
    }
    expect(() =>
      core.validatePolicySchema(
        { version: 1, process: { tasks: { check: { command: ["node"] } } } },
        "partial",
      ),
    ).toThrow();
  });

  it.each([0, -1, 1.5, "2", Infinity, Number.MAX_SAFE_INTEGER + 1])(
    "rejects invalid process cap %s",
    (maxConcurrent) => {
      expect(() => core.resolveProcessPolicy({ maxConcurrent: maxConcurrent as number })).toThrow(
        expect.objectContaining({ code: "INVALID_RUNTIME_POLICY" }),
      );
      expect(() =>
        core.validatePolicySchema(raw({ maxConcurrent, tasks: {} }), "complete"),
      ).toThrow();
    },
  );

  it.each([
    { command: "node check.mjs", inputs: [], outputs: [] },
    { command: [], inputs: [], outputs: [] },
    { ...task(), command: ["bash", "-c", "echo ok"] },
    { ...task(), command: ["C:\\Windows\\System32\\cmd.exe", "/c", "echo ok"] },
    { ...task(), command: ["pwsh", "-Command", "echo ok"] },
    { ...task(), command: ["pnpm.cmd", "test"] },
    { ...task(), command: ["node", "one.mjs", "x;touch file"] },
    { ...task(), command: ["node", "one.mjs", "$(echo x)"] },
    { ...task(), command: ["node", "one.mjs", "x|y"] },
    { ...task(), command: ["node", "one.mjs", "x\0y"] },
    { ...task(), cwd: "../escape" },
    { ...task(), cwd: "C:\\outside" },
    { ...task(), cwd: "/outside" },
    { ...task(), env: { "INVALID=KEY": "x" } },
    { ...task(), env: { GOOD: 1 } },
    { ...task(), input: [] },
  ])("rejects invalid task config %j", (invalidTask) => {
    expect(() => core.resolveProcessPolicy({ tasks: { check: invalidTask as never } })).toThrow(
      expect.objectContaining({ code: "INVALID_RUNTIME_POLICY" }),
    );
  });

  it("accepts literal argv, workspace cwd and explicit env, and rejects unknown names", () => {
    const policy = core.resolveProcessPolicy({
      tasks: {
        check: {
          ...task(),
          command: ["node", "check.mjs", "a b", "*.ts"],
          cwd: "src",
          env: { ONLY_THIS: "yes" },
        },
      },
    });
    expect(core.resolveProcessTask(policy, "check")).toMatchObject({
      cwd: "src",
      env: { ONLY_THIS: "yes" },
    });
    expect(() => core.resolveProcessTask(policy, "missing")).toThrow(
      expect.objectContaining({ code: "POLICY_VIOLATION" }),
    );
    expect(() => core.resolveProcessTask(policy, "constructor")).toThrow();
    expect(() => core.resolveProcessPolicy({ tasks: { "Bad Task": task() } })).toThrow();
  });

  it("projects I/O using canonical core normalization without inventing glob ownership", async () => {
    const root = await temporary();
    await mkdir(path.join(root, "src"));
    const policy = core.resolveProcessPolicy({
      tasks: { generate: { ...task(), outputs: ["out/a.json", "reports/**"] } },
    });
    const declaration = core.processTaskResources(policy, "generate");
    const manifest = await core.normalizeResourceManifest(root, declaration, "deny");
    expect(manifest.reads).toEqual([
      {
        resource: { kind: "workspace", path: path.join(root, "src"), scope: "subtree" },
        mode: "shared",
      },
    ]);
    expect(manifest.produces.map((claim) => [claim.resource.path, claim.resource.scope])).toEqual([
      [path.join(root, "out/a.json"), "exact"],
      [path.join(root, "reports"), "subtree"],
    ]);
    expect(manifest.writes).toEqual(manifest.produces);
    const invalid = core.resolveProcessPolicy({
      tasks: { generate: { ...task(), outputs: ["out/*.json"] } },
    });
    await expect(
      core.normalizeResourceManifest(root, core.processTaskResources(invalid, "generate"), "deny"),
    ).rejects.toMatchObject({ code: "INVALID_REQUEST" });
    expect(
      core.processTaskResources(core.resolveProcessPolicy({ tasks: { check: task() } }), "check")
        .produces,
    ).toEqual([]);
  });
});

describe("process policy production loading", () => {
  it.each([
    [undefined, 8, 1],
    [4, 8, 4],
    [4, 2, 2],
    [4, undefined, 4],
  ])(
    "fills trusted omission before project restriction (%s, %s)",
    async (trusted, requested, expected) => {
      const root = await temporary();
      const userConfigDir = path.join(root, "user"),
        projectDir = path.join(root, "project");
      await mkdir(userConfigDir);
      await mkdir(projectDir);
      const builtinPolicyPath = path.join(root, "builtin.yaml");
      await writeFile(builtinPolicyPath, JSON.stringify(raw()));
      if (trusted !== undefined)
        await writeFile(
          path.join(userConfigDir, "policy.yaml"),
          JSON.stringify({ version: 1, process: { maxConcurrent: trusted } }),
        );
      await writeFile(
        path.join(projectDir, "policy.yaml"),
        JSON.stringify({
          version: 1,
          process: {
            ...(requested === undefined ? {} : { maxConcurrent: requested }),
            tasks: { check: task() },
          },
        }),
      );
      const policy = await core.loadPolicy({ builtinPolicyPath, userConfigDir, projectDir });
      expect(policy.process?.maxConcurrent).toBe(expected);
      expect(policy.process?.tasks.check?.command).toEqual(["node", "check.mjs"]);
    },
  );
});
