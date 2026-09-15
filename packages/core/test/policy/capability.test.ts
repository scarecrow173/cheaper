import { describe, expect, it } from "vitest";
import type { CheaperError } from "../../src/errors.js";
import type { Tier } from "../../src/model/tier.js";
import { checkCapabilities } from "../../src/policy/capability.js";
import type { RuntimePolicy } from "../../src/policy/schema.js";
import type { ToolName } from "../../src/tools/vocabulary.js";
import type { WorkerDefinition } from "../../src/worker/validate.js";

function policy(overrides?: { allow?: ToolName[]; maxTier?: Tier }): RuntimePolicy {
  return {
    execution: { timeoutMs: 120_000, maxTurns: 8, maxToolCalls: 64 },
    tools: { allow: overrides?.allow ?? ["read", "grep", "find", "ls", "write", "edit"] },
    filesystem: { symlinks: "within-workspace" },
    model: { maxTier: overrides?.maxTier ?? "medium", tierFallback: "none" },
    output: { maxTokens: 2000, repairAttempts: 1 },
    retry: { transientAttempts: 1 },
  };
}

function worker(tools: ToolName[], tier: Tier): WorkerDefinition {
  return {
    name: "w",
    description: "d",
    tools,
    cheaper: { model: { tier }, output: { schema: { inline: { type: "object" } } } },
    instructions: "do it",
    sourcePath: "w.md",
    origin: "builtin",
    outputSchema: { schema: { type: "object" }, validate: () => ({ ok: true }) },
  };
}

describe("checkCapabilities", () => {
  it("returns effective tools when requested ⊆ allow", () => {
    expect(checkCapabilities(worker(["read", "ls"], "low"), policy())).toEqual(["read", "ls"]);
  });

  it("rejects when requested tools ⊄ allow, listing missing tools", () => {
    try {
      checkCapabilities(
        worker(["read", "write", "bash" as ToolName], "low"),
        policy({ allow: ["read"] }),
      );
      expect.unreachable();
    } catch (e) {
      const err = e as CheaperError;
      expect(err.code).toBe("POLICY_VIOLATION");
      expect(err.exitCode).toBe(4);
      expect(err.details?.missing).toEqual(["write", "bash"]);
    }
  });

  it("rejects when worker tier exceeds maxTier", () => {
    try {
      checkCapabilities(worker(["read"], "high"), policy({ maxTier: "medium" }));
      expect.unreachable();
    } catch (e) {
      const err = e as CheaperError;
      expect(err.code).toBe("POLICY_VIOLATION");
      expect(err.details).toMatchObject({ requestedTier: "high", maxTier: "medium" });
    }
  });

  it("accepts tier equal to maxTier and lower", () => {
    expect(checkCapabilities(worker(["read"], "medium"), policy({ maxTier: "medium" }))).toEqual([
      "read",
    ]);
    expect(checkCapabilities(worker(["read"], "low"), policy({ maxTier: "medium" }))).toEqual([
      "read",
    ]);
  });

  it("rejects empty allow when worker requests any tool", () => {
    expect(() => checkCapabilities(worker(["read"], "low"), policy({ allow: [] }))).toThrow();
  });
});
