import { describe, expect, it } from "vitest";
import type { CheaperError } from "../../src/errors.js";
import { restrictPolicy } from "../../src/policy/restrict.js";
import type { RuntimePolicy } from "../../src/policy/schema.js";

function policy(): RuntimePolicy {
  return {
    execution: { timeoutMs: 120_000, maxTurns: 8, maxToolCalls: 64 },
    tools: { allow: ["read", "grep", "find", "ls", "write", "edit"] },
    filesystem: { symlinks: "within-workspace", denyPaths: [] },
    model: { maxTier: "medium", tierFallback: "none" },
    output: { maxTokens: 2000, repairAttempts: 1 },
    retry: { transientAttempts: 1 },
  };
}

describe("restrictPolicy", () => {
  it("returns policy unchanged when no restrictions", () => {
    const p = policy();
    expect(restrictPolicy(p, {})).toBe(p);
  });

  it("narrows timeout when --timeout < policy timeout", () => {
    const out = restrictPolicy(policy(), { timeoutMs: 60_000 });
    expect(out.execution.timeoutMs).toBe(60_000);
    expect(out.execution.maxTurns).toBe(8);
  });

  it("accepts equal timeout", () => {
    const out = restrictPolicy(policy(), { timeoutMs: 120_000 });
    expect(out.execution.timeoutMs).toBe(120_000);
  });

  it("rejects --timeout > policy timeout with POLICY_VIOLATION", () => {
    try {
      restrictPolicy(policy(), { timeoutMs: 300_000 });
      expect.unreachable();
    } catch (e) {
      const err = e as CheaperError;
      expect(err.code).toBe("POLICY_VIOLATION");
      expect(err.exitCode).toBe(4);
      expect(err.details).toMatchObject({ requested: 300_000, policy: 120_000 });
    }
  });

  it("does not mutate the original policy", () => {
    const p = policy();
    restrictPolicy(p, { timeoutMs: 60_000 });
    expect(p.execution.timeoutMs).toBe(120_000);
  });
});
