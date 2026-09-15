import { describe, expect, it } from "vitest";
import * as core from "../src/index.js";

describe("@cheaper/core public API", () => {
  it("exports errors/duration/tools/config primitives", () => {
    expect(typeof core.CheaperError).toBe("function");
    expect(typeof core.isCheaperError).toBe("function");
    expect(typeof core.ERROR_TABLE).toBe("object");
    expect(typeof core.parseDuration).toBe("function");
    expect(typeof core.normalizeToolNames).toBe("function");
    expect(Array.isArray(core.TOOL_NAMES)).toBe(true);
    expect(typeof core.resolveConfigDirs).toBe("function");
  });

  it("exports worker modules", () => {
    expect(typeof core.parseWorkerFile).toBe("function");
    expect(typeof core.parseSchemaRef).toBe("function");
    expect(typeof core.loadSchema).toBe("function");
    expect(typeof core.compileSchema).toBe("function");
    expect(typeof core.validateWorkerDefinition).toBe("function");
    expect(typeof core.validateWorkerInput).toBe("function");
    expect(typeof core.resolveWorker).toBe("function");
    expect(typeof core.resolveWorkerFile).toBe("function");
  });

  it("exports policy modules", () => {
    expect(typeof core.POLICY_COMPLETE_SCHEMA).toBe("object");
    expect(typeof core.POLICY_PARTIAL_SCHEMA).toBe("object");
    expect(typeof core.validatePolicySchema).toBe("function");
    expect(typeof core.normalizePolicy).toBe("function");
    expect(typeof core.mergePolicyLayers).toBe("function");
    expect(typeof core.loadPolicy).toBe("function");
    expect(typeof core.restrictPolicy).toBe("function");
    expect(typeof core.checkCapabilities).toBe("function");
  });

  it("exports workspace and model modules", () => {
    expect(typeof core.canonicalizeWorkspaceRoot).toBe("function");
    expect(typeof core.resolveWorkspacePath).toBe("function");
    expect(typeof core.WorkspaceEscapeError).toBe("function");
    expect(typeof core.compareTier).toBe("function");
    expect(typeof core.isTier).toBe("function");
    expect(typeof core.nextTier).toBe("function");
    expect(Array.isArray(core.TIERS)).toBe(true);
    expect(typeof core.loadModelConfig).toBe("function");
    expect(typeof core.classifyModel).toBe("function");
    expect(typeof core.resolveModel).toBe("function");
  });
});
