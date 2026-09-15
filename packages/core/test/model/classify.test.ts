import { describe, expect, it } from "vitest";
import { classifyModel, type ModelInfo } from "../../src/model/classify.js";
import type { ModelConfig } from "../../src/model/config.js";

const EMPTY: ModelConfig = { overrides: {}, preferences: {} };

function model(id: string, cost?: { input: number; output: number }): ModelInfo {
  const m: ModelInfo = { provider: "test", id };
  if (cost !== undefined) m.cost = cost;
  return m;
}

describe("classifyModel overrides", () => {
  it("exact <provider>/<id> override wins over built-in table and cost", () => {
    const config: ModelConfig = {
      overrides: { "test/claude-opus-4": "low" },
      preferences: {},
    };
    expect(classifyModel({ provider: "test", id: "claude-opus-4" }, config)).toBe("low");
  });

  it("override does not match other providers' same id", () => {
    const config: ModelConfig = { overrides: { "a/x": "high" }, preferences: {} };
    expect(classifyModel({ provider: "b", id: "x" }, config)).toBe("medium");
  });
});

describe("classifyModel built-in table (high → low → medium)", () => {
  it.each([
    ["claude-opus-4-1", "high"],
    ["gpt-5-pro", "high"],
    ["gpt-5.2-pro", "high"],
    ["o1", "high"],
    ["o1-pro", "high"],
    ["o3-pro", "high"],
    ["gemini-heavy-1", "high"],
    ["ultra-x", "high"],
    ["claude-haiku-4-5", "low"],
    ["gpt-5-mini", "low"],
    ["gemini-2.0-flash", "low"],
    ["codex-mini-latest", "low"],
    ["llama-3-8b", "low"],
    ["mistral-small", "low"],
    ["claude-sonnet-4", "medium"],
    ["gpt-4o", "medium"],
    ["gpt-4.1", "medium"],
    ["gpt-5", "medium"],
    ["gpt-5.5", "medium"],
    ["o3", "medium"],
    ["codex", "medium"],
  ])("classifies %s as %s", (id, tier) => {
    expect(classifyModel(model(id), EMPTY)).toBe(tier);
  });

  it("evaluates high before low before medium (codex-mini is low, gpt-5-pro is high)", () => {
    // codex-mini also matches medium "codex"; low must win by order
    expect(classifyModel(model("codex-mini"), EMPTY)).toBe("low");
    // gpt-5-pro also ends with nothing that matches medium; high must win
    expect(classifyModel(model("gpt-5-pro"), EMPTY)).toBe("high");
  });

  it("'mini' is word-boundary anchored so gemini-*-pro reaches the medium rule", () => {
    // `\bmini\b` は "gemini" 内の "mini" に match しない（実装レビューで
    // 発見した dead medium rule を修正した design の挙動を固定する）。
    expect(classifyModel(model("gemini-2.5-pro"), EMPTY)).toBe("medium");
    // word boundary 側の low 判定は維持される
    expect(classifyModel(model("o3-mini"), EMPTY)).toBe("low");
  });
});

describe("classifyModel cost heuristic", () => {
  it.each([
    [1.9, "low"],
    [0.1, "low"],
    [2, "medium"],
    [20, "medium"],
    [20.1, "high"],
    [75, "high"],
  ])("cost.output %d → %s for unmatched id", (output, tier) => {
    expect(classifyModel(model("unmatched-model-xyz", { input: 0.5, output }), EMPTY)).toBe(tier);
  });

  it("missing or zero cost → medium", () => {
    expect(classifyModel(model("unmatched-model-xyz"), EMPTY)).toBe("medium");
    expect(classifyModel(model("unmatched-model-xyz", { input: 0, output: 0 }), EMPTY)).toBe(
      "medium",
    );
  });
});
