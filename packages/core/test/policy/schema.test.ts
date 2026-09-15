import { describe, expect, it } from "vitest";
import type { CheaperError } from "../../src/errors.js";
import {
  normalizePolicy,
  POLICY_COMPLETE_SCHEMA,
  POLICY_PARTIAL_SCHEMA,
  validatePolicySchema,
} from "../../src/policy/schema.js";

function completePolicy(): Record<string, unknown> {
  return {
    version: 1,
    execution: { timeout: "120s", maxTurns: 8, maxToolCalls: 64 },
    tools: { allow: ["read", "grep", "find", "ls", "write", "edit"] },
    filesystem: { symlinks: "within-workspace" },
    model: { maxTier: "medium", tierFallback: "none" },
    output: { maxTokens: 2000, repairAttempts: 1 },
    retry: { transientAttempts: 1 },
  };
}

describe("validatePolicySchema complete", () => {
  it("accepts a full canonical policy", () => {
    const out = validatePolicySchema(completePolicy(), "complete");
    expect(out.version).toBe(1);
  });

  it("rejects missing required section", () => {
    const raw = completePolicy();
    delete raw.retry;
    try {
      validatePolicySchema(raw, "complete");
      expect.unreachable();
    } catch (e) {
      const err = e as CheaperError;
      expect(err.code).toBe("INVALID_RUNTIME_POLICY");
      expect(err.exitCode).toBe(3);
      expect(Array.isArray(err.details?.errors)).toBe(true);
    }
  });

  it("rejects missing version", () => {
    const raw = completePolicy();
    delete raw.version;
    expect(() => validatePolicySchema(raw, "complete")).toThrow(/INVALID|policy/i);
  });

  it("rejects unknown top-level field", () => {
    const raw = { ...completePolicy(), bogus: true };
    expect(() => validatePolicySchema(raw, "complete")).toThrow();
  });

  it("rejects unknown nested field (additionalProperties false at all levels)", () => {
    const raw = completePolicy();
    (raw.execution as Record<string, unknown>).extra = 1;
    expect(() => validatePolicySchema(raw, "complete")).toThrow();
    const raw2 = completePolicy();
    (raw2.model as Record<string, unknown>).provider = "openai";
    expect(() => validatePolicySchema(raw2, "complete")).toThrow();
  });

  it("rejects wrong types and out-of-range values", () => {
    for (const mutate of [
      (r: Record<string, unknown>) => {
        (r.execution as Record<string, unknown>).timeout = 120;
      },
      (r: Record<string, unknown>) => {
        (r.execution as Record<string, unknown>).maxTurns = 0;
      },
      (r: Record<string, unknown>) => {
        (r.output as Record<string, unknown>).repairAttempts = -1;
      },
      (r: Record<string, unknown>) => {
        (r.model as Record<string, unknown>).maxTier = "ultra";
      },
      (r: Record<string, unknown>) => {
        (r.filesystem as Record<string, unknown>).symlinks = "allow";
      },
      (r: Record<string, unknown>) => {
        (r.execution as Record<string, unknown>).timeout = "10x";
      },
    ]) {
      const raw = completePolicy();
      mutate(raw);
      expect(() => validatePolicySchema(raw, "complete")).toThrow();
    }
  });
});

describe("validatePolicySchema partial", () => {
  it("accepts version only", () => {
    const out = validatePolicySchema({ version: 1 }, "partial");
    expect(out.version).toBe(1);
  });

  it("accepts a sparse override", () => {
    const out = validatePolicySchema({ version: 1, execution: { timeout: "60s" } }, "partial");
    expect((out.execution as Record<string, unknown>).timeout).toBe("60s");
  });

  it("still requires version", () => {
    expect(() => validatePolicySchema({ execution: { timeout: "60s" } }, "partial")).toThrow();
  });

  it("rejects unknown fields and bad types in present sections", () => {
    expect(() => validatePolicySchema({ version: 1, nope: {} }, "partial")).toThrow();
    expect(() =>
      validatePolicySchema({ version: 1, tools: { deny: ["read"] } }, "partial"),
    ).toThrow();
    expect(() =>
      validatePolicySchema({ version: 1, execution: { maxTurns: "8" } }, "partial"),
    ).toThrow();
  });
});

describe("normalizePolicy", () => {
  it("normalizes timeout duration to timeoutMs and drops version", () => {
    const raw = validatePolicySchema(completePolicy(), "complete");
    const policy = normalizePolicy(raw);
    expect(policy.execution.timeoutMs).toBe(120_000);
    expect(policy.execution.maxTurns).toBe(8);
    expect(policy.execution.maxToolCalls).toBe(64);
    expect(policy.tools.allow).toEqual(["read", "grep", "find", "ls", "write", "edit"]);
    expect(policy.filesystem.symlinks).toBe("within-workspace");
    expect(policy.model.maxTier).toBe("medium");
    expect(policy.model.tierFallback).toBe("none");
    expect(policy.output.maxTokens).toBe(2000);
    expect(policy.output.repairAttempts).toBe(1);
    expect(policy.retry.transientAttempts).toBe(1);
    expect("version" in policy).toBe(false);
    expect("timeout" in policy.execution).toBe(false);
  });

  it("normalizes tool aliases in tools.allow", () => {
    const raw = completePolicy();
    (raw.tools as Record<string, unknown>).allow = ["Read", "grep"];
    const policy = normalizePolicy(validatePolicySchema(raw, "complete"));
    expect(policy.tools.allow).toEqual(["read", "grep"]);
  });

  it("rejects unknown tool with INVALID_RUNTIME_POLICY (not UNKNOWN_TOOL)", () => {
    const raw = completePolicy();
    (raw.tools as Record<string, unknown>).allow = ["read", "Bash"];
    try {
      normalizePolicy(validatePolicySchema(raw, "complete"));
      expect.unreachable();
    } catch (e) {
      const err = e as CheaperError;
      expect(err.code).toBe("INVALID_RUNTIME_POLICY");
      expect(err.details?.tool).toBe("Bash");
    }
  });

  it("rejects duplicate tools after alias normalization", () => {
    const raw = completePolicy();
    (raw.tools as Record<string, unknown>).allow = ["read", "Read"];
    expect(() => normalizePolicy(validatePolicySchema(raw, "complete"))).toThrow();
  });
});

describe("schema objects", () => {
  it("both schemas are exported as JSON Schema objects", () => {
    expect(POLICY_COMPLETE_SCHEMA.type).toBe("object");
    expect(POLICY_PARTIAL_SCHEMA.type).toBe("object");
    expect(POLICY_COMPLETE_SCHEMA.additionalProperties).toBe(false);
    expect(POLICY_PARTIAL_SCHEMA.additionalProperties).toBe(false);
  });
});
