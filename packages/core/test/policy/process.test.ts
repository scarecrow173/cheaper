import { mkdir, mkdtemp, readFile, realpath, rm, writeFile } from "node:fs/promises";
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
      timeout: "2m",
      maxOutputBytes: 65536,
      killGrace: "2s",
      heartbeat: { interval: "30s" },
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
    [
      undefined,
      { timeout: "5m", maxOutputBytes: 1048576, killGrace: "5s", heartbeat: { interval: "1m" } },
      { timeout: "2m", maxOutputBytes: 65536, killGrace: "2s", heartbeat: { interval: "30s" } },
    ],
    [
      { timeout: "3m", maxOutputBytes: 1048576, killGrace: "5s", heartbeat: { interval: "45s" } },
      { timeout: "5m", maxOutputBytes: 2097152, killGrace: "10s", heartbeat: { interval: "1m" } },
      { timeout: "3m", maxOutputBytes: 1048576, killGrace: "5s", heartbeat: { interval: "45s" } },
    ],
    [
      { timeout: "3m", maxOutputBytes: 1048576, killGrace: "5s", heartbeat: { interval: "45s" } },
      { timeout: "60s", maxOutputBytes: 1024, killGrace: "100ms", heartbeat: { interval: "10s" } },
      { timeout: "60s", maxOutputBytes: 1024, killGrace: "100ms", heartbeat: { interval: "10s" } },
    ],
    [
      { timeout: "3m", maxOutputBytes: 1048576, killGrace: "5s", heartbeat: { interval: "45s" } },
      {},
      { timeout: "3m", maxOutputBytes: 1048576, killGrace: "5s", heartbeat: { interval: "45s" } },
    ],
  ])(
    "replaces trusted lifecycle values then only restricts project values (%j, %j)",
    async (trusted, project, expected) => {
      const root = await temporary();
      const userConfigDir = path.join(root, "user"),
        projectDir = path.join(root, "project");
      await mkdir(userConfigDir);
      await mkdir(projectDir);
      const builtinPolicyPath = path.join(root, "builtin.yaml");
      await writeFile(builtinPolicyPath, JSON.stringify(raw()));
      if (trusted)
        await writeFile(
          path.join(userConfigDir, "policy.yaml"),
          JSON.stringify({ version: 1, process: trusted }),
        );
      await writeFile(
        path.join(projectDir, "policy.yaml"),
        JSON.stringify({ version: 1, process: project }),
      );
      const policy = await core.loadPolicy({ builtinPolicyPath, userConfigDir, projectDir });
      expect(policy.process).toMatchObject(expected);
    },
  );

  it("declares identical lifecycle defaults in both distribution policy assets", async () => {
    const { parse } = await import("yaml");
    for (const asset of ["packages/cli/policy.yaml", "packages/apm/.apm/hooks/cli/policy.yaml"]) {
      const policy = parse(await readFile(path.join(process.cwd(), asset), "utf8"));
      expect(policy.process).toMatchObject({
        timeout: "2m",
        maxOutputBytes: 65536,
        killGrace: "2s",
        heartbeat: { interval: "30s" },
      });
      expect(
        core.normalizePolicy(core.validatePolicySchema(policy, "complete")).process,
      ).toMatchObject(policy.process);
    }
  });

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

describe("process lifecycle policy boundary", () => {
  it("snapshots heartbeat and supports positive bounded duration/cap overrides", () => {
    const configured = {
      maxConcurrent: 2,
      tasks: {},
      timeout: "2147483647ms",
      killGrace: "1ms",
      heartbeat: { interval: "5s" },
      maxOutputBytes: 16777216,
    };
    const resolved = core.resolveProcessPolicy(configured);
    configured.heartbeat.interval = "1h";
    expect(resolved).toMatchObject({
      timeout: "2147483647ms",
      killGrace: "1ms",
      heartbeat: { interval: "5s" },
      maxOutputBytes: 16777216,
    });
    expect(core.resolveProcessPolicy({ heartbeat: {} })).toMatchObject({
      heartbeat: { interval: "30s" },
    });
    expect(
      core.validatePolicySchema({ version: 1, process: { heartbeat: {} } }, "partial"),
    ).toEqual({ version: 1, process: { heartbeat: {} } });
  });

  it.each(["timeout", "killGrace", "heartbeat"])(
    "rejects invalid or overflowing %s at both raw boundaries",
    (field) => {
      for (const value of [
        "0s",
        "-1s",
        "1.5s",
        "1d",
        "2147483648ms",
        "999999999999999999999h",
        10,
        false,
        null,
      ]) {
        const processPolicy =
          field === "heartbeat" ? { heartbeat: { interval: value } } : { [field]: value };
        expect(() => core.resolveProcessPolicy(processPolicy as never)).toThrow(
          expect.objectContaining({ code: "INVALID_RUNTIME_POLICY" }),
        );
        expect(() =>
          core.validatePolicySchema({ version: 1, process: processPolicy }, "partial"),
        ).toThrow(expect.objectContaining({ code: "INVALID_RUNTIME_POLICY" }));
      }
    },
  );

  it.each([0, -1, 1.5, "1024", Infinity, 16777217])(
    "rejects invalid retained byte cap %s",
    (maxOutputBytes) => {
      expect(() => core.resolveProcessPolicy({ maxOutputBytes } as never)).toThrow(
        expect.objectContaining({ code: "INVALID_RUNTIME_POLICY" }),
      );
      expect(() =>
        core.validatePolicySchema({ version: 1, process: { maxOutputBytes } }, "partial"),
      ).toThrow(expect.objectContaining({ code: "INVALID_RUNTIME_POLICY" }));
    },
  );

  it.each([
    { timeout: "1m" },
    { maxOutputBytes: 1024 },
    { killGrace: "1s" },
    { heartbeat: { interval: "1s" } },
  ])("rejects per-task lifecycle overrides %j", (override) => {
    expect(() =>
      core.resolveProcessPolicy({ tasks: { check: { ...task(), ...override } } }),
    ).toThrow(expect.objectContaining({ code: "INVALID_RUNTIME_POLICY" }));
  });
});
