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
  it("exact <provider>/<id> override wins and reports source=explicit-override", () => {
    const config: ModelConfig = {
      overrides: { "test/claude-opus-4": "low" },
      preferences: {},
    };
    expect(classifyModel({ provider: "test", id: "claude-opus-4" }, config)).toEqual({
      tier: "low",
      source: "explicit-override",
    });
  });

  it("override does not match other providers' same id", () => {
    const config: ModelConfig = { overrides: { "a/x": "high" }, preferences: {} };
    expect(classifyModel({ provider: "b", id: "x" }, config)).toEqual({
      tier: "medium",
      source: "unclassified",
    });
  });
});

describe("classifyModel curated exceptions", () => {
  it("bundled ollama/* exception → low / curated-exception", () => {
    expect(classifyModel({ provider: "ollama", id: "qwen3" }, EMPTY)).toEqual({
      tier: "low",
      source: "curated-exception",
    });
  });

  it("override wins over curated exception", () => {
    const config: ModelConfig = { overrides: { "ollama/qwen3": "high" }, preferences: {} };
    expect(classifyModel({ provider: "ollama", id: "qwen3" }, config)).toEqual({
      tier: "high",
      source: "explicit-override",
    });
  });
});

describe("classifyModel auto classification（#46 scope、#45 では未生成）", () => {
  it.each([0.1, 1.9, 2, 20, 20.1, 75])(
    "cost.output %d でも auto 分類せず medium / unclassified",
    (output) => {
      expect(classifyModel(model("unmatched-model-xyz", { input: 0.5, output }), EMPTY)).toEqual({
        tier: "medium",
        source: "unclassified",
      });
    },
  );
});

describe("classifyModel unclassified", () => {
  it("missing cost → medium / unclassified", () => {
    expect(classifyModel(model("unmatched-model-xyz"), EMPTY)).toEqual({
      tier: "medium",
      source: "unclassified",
    });
  });

  it("zero cost は free の観測だが自動で low にしない → medium / unclassified", () => {
    expect(classifyModel(model("unmatched-model-xyz", { input: 0, output: 0 }), EMPTY)).toEqual({
      tier: "medium",
      source: "unclassified",
    });
  });
});
