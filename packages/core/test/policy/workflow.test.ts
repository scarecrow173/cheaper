import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import * as core from "../../src/index.js";
import { cleanupTmpDir, makeTmpDir } from "../fixtures/tmp.js";

const defaults = {
  maxJobs: 1,
  artifacts: {
    maxArtifactBytes: 8388608,
    maxTotalBytes: 33554432,
    maxPreviewBytes: 2048,
    maxReadBytes: 65536,
  },
};
function raw(workflow?: unknown) {
  return {
    version: 1,
    execution: { timeout: "120s", maxTurns: 8, maxToolCalls: 64 },
    tools: { allow: ["read"] },
    filesystem: { symlinks: "deny" },
    model: { maxTier: "medium", tierFallback: "none" },
    output: { maxTokens: 2000, repairAttempts: 1 },
    retry: { transientAttempts: 1 },
    ...(workflow === undefined ? {} : { workflow }),
  };
}
const dirs: string[] = [];
afterEach(async () => {
  await Promise.all(dirs.splice(0).map(cleanupTmpDir));
});

describe("workflow policy の正本", () => {
  it("legacy omission を補完し返却 defaults を共有しない", () => {
    const input = raw();
    const policy = core.normalizePolicy(core.validatePolicySchema(input, "complete"));
    expect(policy.workflow).toEqual(defaults);
    expect(core.resolveWorkflowPolicy()).toEqual(defaults);
    if (!policy.workflow) throw new Error("normalized workflow missing");
    policy.workflow.artifacts.maxPreviewBytes = 0;
    expect(core.resolveWorkflowPolicy()).toEqual(defaults);
    expect(input).not.toHaveProperty("workflow");
  });
  it("partial workflow と artifacts の各 omission を補完する", () => {
    const cases = [
      { workflow: {}, expected: defaults },
      { workflow: { maxJobs: 3 }, expected: { ...defaults, maxJobs: 3 } },
      {
        workflow: { artifacts: { maxPreviewBytes: 0 } },
        expected: {
          ...defaults,
          artifacts: { ...defaults.artifacts, maxPreviewBytes: 0 },
        },
      },
    ];
    for (const { workflow, expected } of cases) {
      const policy = core.normalizePolicy(core.validatePolicySchema(raw(workflow), "complete"));
      expect(policy.workflow).toEqual(expected);
      expect(core.resolveWorkflowPolicy(workflow)).toEqual(expected);
      expect(core.validatePolicySchema({ version: 1, workflow }, "partial")).toBeTruthy();
    }
  });
  it.each([0, -1, 1.5, "2", Number.POSITIVE_INFINITY, Number.MAX_SAFE_INTEGER + 1])(
    "invalid maxJobs/read cap を schema と resolver で拒否: %s",
    (cap) => {
      for (const workflow of [{ maxJobs: cap }, { artifacts: { maxReadBytes: cap } }]) {
        expect(() => core.validatePolicySchema(raw(workflow), "complete")).toThrow(
          expect.objectContaining({ code: "INVALID_RUNTIME_POLICY" }),
        );
        expect(() => core.resolveWorkflowPolicy(workflow as never)).toThrow(
          expect.objectContaining({ code: "INVALID_RUNTIME_POLICY" }),
        );
      }
    },
  );
  it("zero storage/preview caps を許可し不正値と未知 key を拒否", () => {
    const zero = { maxArtifactBytes: 0, maxTotalBytes: 0, maxPreviewBytes: 0, maxReadBytes: 1 };
    expect(core.resolveWorkflowPolicy({ artifacts: zero }).artifacts).toEqual(zero);
    expect(core.validatePolicySchema(raw({ artifacts: zero }), "complete")).toBeTruthy();
    for (const artifacts of [{ maxTotalBytes: -1 }, { maxPreviewBytes: 0.5 }, { unknown: 2 }]) {
      expect(() => core.validatePolicySchema(raw({ artifacts }), "complete")).toThrow();
    }
    expect(() => core.validatePolicySchema(raw({ unknown: 1 }), "complete")).toThrow();
  });
  it.each([undefined, 8, 2])(
    "trusted default/user 後に project を min restriction: %s",
    async (user) => {
      const dir = await makeTmpDir();
      dirs.push(dir);
      const userConfigDir = path.join(dir, "user"),
        projectDir = path.join(dir, "project");
      await mkdir(userConfigDir);
      await mkdir(projectDir);
      const builtinPolicyPath = path.join(dir, "builtin.yaml");
      await writeFile(builtinPolicyPath, JSON.stringify(raw()));
      if (user !== undefined) {
        await writeFile(
          path.join(userConfigDir, "policy.yaml"),
          JSON.stringify({
            version: 1,
            workflow: {
              maxJobs: 4,
              artifacts: {
                maxPreviewBytes: 32,
                maxArtifactBytes: 4096,
                maxReadBytes: 64,
              },
            },
          }),
        );
      }
      await writeFile(
        path.join(projectDir, "policy.yaml"),
        JSON.stringify({
          version: 1,
          workflow: {
            maxJobs: user ?? 16,
            artifacts: {
              maxPreviewBytes: 100000,
              maxTotalBytes: 0,
              maxArtifactBytes: 512,
              maxReadBytes: 100000,
            },
          },
        }),
      );
      const policy = await core.loadPolicy({ builtinPolicyPath, userConfigDir, projectDir });
      expect(policy.workflow?.maxJobs).toBe(user === undefined ? 1 : Math.min(4, user));
      expect(policy.workflow?.artifacts.maxPreviewBytes).toBe(user === undefined ? 2048 : 32);
      expect(policy.workflow?.artifacts.maxTotalBytes).toBe(0);
      expect(policy.workflow?.artifacts.maxArtifactBytes).toBe(512);
      expect(policy.workflow?.artifacts.maxReadBytes).toBe(user === undefined ? 65536 : 64);
      expect(policy.execution.concurrency?.maxOperations).toBe(8);
      expect(() => core.restrictPolicy(policy, { timeoutMs: 120001 })).toThrow(
        expect.objectContaining({ code: "POLICY_VIOLATION" }),
      );
      expect(core.restrictPolicy(policy, { timeoutMs: 1000 }).workflow).toEqual(policy.workflow);
    },
  );
});
