import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { loadPolicy } from "../../src/policy/load.js";
import { mergePolicyLayers } from "../../src/policy/merge.js";
import { restrictPolicy } from "../../src/policy/restrict.js";
import * as schema from "../../src/policy/schema.js";
import { cleanupTmpDir, makeTmpDir } from "../fixtures/tmp.js";

const defaults = { maxOperations: 8, mutations: "serialized" };
function raw(concurrency?: unknown) {
  return {
    version: 1,
    execution: {
      timeout: "120s",
      maxTurns: 8,
      maxToolCalls: 64,
      ...(concurrency === undefined ? {} : { concurrency }),
    },
    tools: { allow: ["read"] },
    filesystem: { symlinks: "deny" },
    model: { maxTier: "medium", tierFallback: "none" },
    output: { maxTokens: 2000, repairAttempts: 1 },
    retry: { transientAttempts: 1 },
  };
}
function policy(concurrency?: unknown) {
  return schema.normalizePolicy(schema.validatePolicySchema(raw(concurrency), "complete"));
}

describe("concurrency policy boundaries", () => {
  it("normalizes legacy omission with the canonical resolver without mutating input", () => {
    const input = raw();
    expect(policy().execution.concurrency).toEqual(defaults);
    expect(schema.resolveConcurrencyPolicy()).toEqual(defaults);
    expect(schema.resolveConcurrencyPolicy({ mutations: "per-resource" })).toEqual({
      ...defaults,
      mutations: "per-resource",
    });
    expect(input.execution).not.toHaveProperty("concurrency");
  });
  it("requires both fields only when complete concurrency is present", () => {
    expect(policy({ maxOperations: 16, mutations: "per-resource" }).execution.concurrency).toEqual({
      maxOperations: 16,
      mutations: "per-resource",
    });
    for (const concurrency of [{}, { maxOperations: 4 }, { mutations: "serialized" }]) {
      expect(() => schema.validatePolicySchema(raw(concurrency), "complete")).toThrow();
      const partial = { version: 1, execution: { concurrency } };
      expect(schema.validatePolicySchema(partial, "partial")).toEqual(partial);
    }
  });
  it.each([0, -1, 1.5, "2", Number.POSITIVE_INFINITY, Number.MAX_SAFE_INTEGER + 1])(
    "rejects invalid cap %s at the schema and resolver boundaries",
    (maxOperations) => {
      expect(() =>
        schema.validatePolicySchema(raw({ maxOperations, mutations: "serialized" }), "complete"),
      ).toThrow();
      expect(() =>
        schema.resolveConcurrencyPolicy({ maxOperations: maxOperations as number }),
      ).toThrow();
    },
  );
  it("rejects unknown concurrency keys and mutation modes", () => {
    expect(() => schema.validatePolicySchema(raw({ ...defaults, pool: 2 }), "complete")).toThrow();
    expect(() =>
      schema.validatePolicySchema(raw({ ...defaults, mutations: "parallel" }), "complete"),
    ).toThrow();
    expect(() => schema.resolveConcurrencyPolicy({ mutations: "parallel" as never })).toThrow();
  });
  it("allows trusted replacement and restricts only declared concurrency paths", () => {
    const base = raw(defaults);
    const trusted = mergePolicyLayers(base, {
      execution: { concurrency: { maxOperations: 16, mutations: "per-resource" } },
    });
    const widened = mergePolicyLayers(
      trusted,
      {
        execution: { maxTurns: 12, concurrency: { maxOperations: 32, mutations: "per-resource" } },
        tools: { allow: ["ls"] },
      },
      { mode: "restrict-concurrency" },
    );
    expect(policy().execution.concurrency).toEqual(defaults);
    expect((widened.execution as Record<string, unknown>).concurrency).toEqual({
      maxOperations: 16,
      mutations: "per-resource",
    });
    expect((widened.execution as Record<string, unknown>).maxTurns).toBe(12);
    expect(widened.tools).toEqual({ allow: ["ls"] });
    const narrowed = mergePolicyLayers(
      trusted,
      { execution: { concurrency: { maxOperations: 4, mutations: "serialized" } } },
      { mode: "restrict-concurrency" },
    );
    expect((narrowed.execution as Record<string, unknown>).concurrency).toEqual({
      maxOperations: 4,
      mutations: "serialized",
    });
    expect((base.execution as Record<string, unknown>).concurrency).toEqual(defaults);
    expect(
      mergePolicyLayers(
        base,
        { execution: { concurrency: { mutations: "per-resource" } } },
        { mode: "restrict-concurrency" },
      ),
    ).toEqual(base);
    expect(
      mergePolicyLayers(
        trusted,
        { execution: { concurrency: {} } },
        { mode: "restrict-concurrency" },
      ),
    ).toEqual(trusted);
  });
  it("narrows CLI-style concurrency without allowing either relaxation", () => {
    const p = policy({ maxOperations: 16, mutations: "per-resource" });
    const narrowed = restrictPolicy(p, {
      timeoutMs: 60_000,
      concurrency: { maxOperations: 4, mutations: "serialized" },
    });
    expect(narrowed.execution).toMatchObject({
      timeoutMs: 60_000,
      concurrency: { maxOperations: 4, mutations: "serialized" },
    });
    expect(p.execution.concurrency).toEqual({ maxOperations: 16, mutations: "per-resource" });
    expect(() => restrictPolicy(p, { concurrency: { maxOperations: 17 } })).toThrow(
      expect.objectContaining({ code: "POLICY_VIOLATION" }),
    );
    expect(() => restrictPolicy(policy(), { concurrency: { mutations: "per-resource" } })).toThrow(
      expect.objectContaining({ code: "POLICY_VIOLATION" }),
    );
  });
});

const dirs: string[] = [];
afterEach(async () => {
  for (const dir of dirs.splice(0)) await cleanupTmpDir(dir);
});
describe("concurrency through production policy loading", () => {
  it.each([
    [undefined, { maxOperations: 16, mutations: "per-resource" }, defaults],
    [
      { maxOperations: 16, mutations: "per-resource" },
      { maxOperations: 32, mutations: "per-resource" },
      { maxOperations: 16, mutations: "per-resource" },
    ],
    [
      { maxOperations: 16, mutations: "per-resource" },
      { maxOperations: 4, mutations: "serialized" },
      { maxOperations: 4, mutations: "serialized" },
    ],
    [
      { mutations: "per-resource" },
      { maxOperations: 32 },
      { maxOperations: 8, mutations: "per-resource" },
    ],
    [
      { maxOperations: 16, mutations: "per-resource" },
      {},
      { maxOperations: 16, mutations: "per-resource" },
    ],
  ])("keeps project within the trusted/default bound (%j, %j)", async (user, project, expected) => {
    const dir = await makeTmpDir();
    dirs.push(dir);
    const builtinPolicyPath = path.join(dir, "builtin.yaml");
    const userConfigDir = path.join(dir, "user");
    const projectDir = path.join(dir, "project");
    await mkdir(userConfigDir);
    await mkdir(projectDir);
    await writeFile(builtinPolicyPath, JSON.stringify(raw()));
    if (user)
      await writeFile(
        path.join(userConfigDir, "policy.yaml"),
        JSON.stringify({ version: 1, execution: { concurrency: user } }),
      );
    await writeFile(
      path.join(projectDir, "policy.yaml"),
      JSON.stringify({ version: 1, execution: { concurrency: project } }),
    );
    expect(
      (await loadPolicy({ builtinPolicyPath, userConfigDir, projectDir })).execution.concurrency,
    ).toEqual(expected);
  });
});
