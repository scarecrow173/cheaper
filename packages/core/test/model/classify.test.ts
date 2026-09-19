import { describe, expect, it } from "vitest";
import {
  buildAutoTierContext,
  classifyModel,
  classifyModels,
  DEFAULT_OUTPUT_PER_INPUT,
  type ModelInfo,
} from "../../src/model/classify.js";
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

describe("auto tier classification (issue #46)", () => {
  const classifySet = (models: ModelInfo[], outputPerInput?: number) =>
    classifyModels(models, EMPTY, { ...(outputPerInput !== undefined ? { outputPerInput } : {}) });

  it("effectiveCost = input + output * outputPerInput", () => {
    const a = model("a", { input: 2, output: 10 });
    const b = model("b", { input: 1, output: 1 });
    const out = classifySet([a, b], 0.05);
    expect(out.get(a)?.auto?.effectiveCost).toBeCloseTo(2.5);
    expect(out.get(b)?.auto?.effectiveCost).toBeCloseTo(1.05);
  });

  it("3 件の異価格 model → low / medium / high に分かれる", () => {
    const cheap = model("cheap", { input: 0.1, output: 0.1 });
    const mid = model("mid", { input: 1, output: 1 });
    const pricey = model("pricey", { input: 10, output: 10 });
    const out = classifySet([pricey, cheap, mid]);
    expect(out.get(cheap)).toMatchObject({ tier: "low", source: "auto" });
    expect(out.get(mid)).toMatchObject({ tier: "medium", source: "auto" });
    expect(out.get(pricey)).toMatchObject({ tier: "high", source: "auto" });
    expect(out.get(cheap)?.auto?.effectiveCostPercentile).toBeCloseTo(1 / 6);
    expect(out.get(mid)?.auto?.effectiveCostPercentile).toBeCloseTo(0.5);
    expect(out.get(pricey)?.auto?.effectiveCostPercentile).toBeCloseTo(5 / 6);
  });

  it("6 件 → low 2 / medium 2 / high 2", () => {
    const models = [1, 2, 3, 4, 5, 6].map((i) => model(`m${i}`, { input: i, output: 0 }));
    const out = classifySet(models);
    expect(models.map((m) => out.get(m)?.tier)).toEqual([
      "low",
      "low",
      "medium",
      "medium",
      "high",
      "high",
    ]);
  });

  it("母集団 2 件 → low / high（medium 無し）", () => {
    const a = model("a", { input: 1, output: 0 });
    const b = model("b", { input: 2, output: 0 });
    const out = classifySet([a, b]);
    expect(out.get(a)?.tier).toBe("low");
    expect(out.get(b)?.tier).toBe("high");
  });

  it("母集団 1 件 → p=0.5 で medium（相対順位なし）", () => {
    const a = model("a", { input: 5, output: 5 });
    const out = classifySet([a]);
    expect(out.get(a)).toMatchObject({
      tier: "medium",
      source: "auto",
      auto: { effectiveCostPercentile: 0.5 },
    });
  });

  it("全員同じ effectiveCost → 全員 medium（恣意的に分割しない）", () => {
    const models = [model("a", { input: 1, output: 1 }), model("b", { input: 1, output: 1 })];
    const out = classifySet(models);
    for (const m of models) {
      expect(out.get(m)).toMatchObject({
        tier: "medium",
        source: "auto",
        auto: { effectiveCostPercentile: 0.5 },
      });
    }
  });

  it("部分的な tie は同一 tier を共有する", () => {
    const a = model("a", { input: 1, output: 0 });
    const b = model("b", { input: 1, output: 0 });
    const c = model("c", { input: 2, output: 0 });
    const d = model("d", { input: 3, output: 0 });
    const out = classifySet([a, b, c, d]);
    // p(a)=p(b)=(0+1)/4=0.25 → low; p(c)=(2+0.5)/4=0.625 → medium; p(d)=0.875 → high
    expect(out.get(a)?.tier).toBe("low");
    expect(out.get(b)?.tier).toBe("low");
    expect(out.get(c)?.tier).toBe("medium");
    expect(out.get(d)?.tier).toBe("high");
  });

  it("workload が異なると同一 model の tier が変わりうる", () => {
    // inputHeavy は input 高・output 安、outputHeavy は逆
    const inputHeavy = model("input-heavy", { input: 10, output: 1 });
    const outputHeavy = model("output-heavy", { input: 1, output: 10 });
    const reader = classifySet([inputHeavy, outputHeavy], 0);
    expect(reader.get(inputHeavy)?.tier).toBe("high");
    expect(reader.get(outputHeavy)?.tier).toBe("low");
    const writer = classifySet([inputHeavy, outputHeavy], 10);
    // effectiveCost: inputHeavy=20, outputHeavy=101 → inputHeavy が low
    expect(writer.get(inputHeavy)?.tier).toBe("low");
    expect(writer.get(outputHeavy)?.tier).toBe("high");
  });

  it("outputPerInput 未指定は DEFAULT_OUTPUT_PER_INPUT", () => {
    const a = model("a", { input: 1, output: 1 });
    const b = model("b", { input: 5, output: 0 });
    const out = classifySet([a, b]);
    expect(out.get(a)?.auto?.outputPerInput).toBe(DEFAULT_OUTPUT_PER_INPUT);
    // effectiveCost: a=2, b=5
    expect(out.get(a)?.auto?.effectiveCost).toBe(2);
  });

  it("override / curated exception を持つ model も ranking 母集団に含まれる", () => {
    const config: ModelConfig = {
      overrides: { "test/pinned": "high" },
      preferences: {},
    };
    const pinned = model("pinned", { input: 0.1, output: 0.1 });
    const a = model("a", { input: 1, output: 1 });
    const b = model("b", { input: 10, output: 10 });
    const out = classifyModels([pinned, a, b], config);
    // pinned は母集団最安値を占有するが tier は override
    expect(out.get(pinned)).toMatchObject({ tier: "high", source: "explicit-override" });
    expect(out.get(a)?.tier).toBe("medium");
    expect(out.get(b)?.tier).toBe("high");
  });

  it("autoContext 未指定の classifyModel は auto 分類せず unclassified", () => {
    expect(classifyModel(model("x", { input: 1, output: 1 }), EMPTY)).toEqual({
      tier: "medium",
      source: "unclassified",
    });
  });

  it("価格ゼロ（input=0, output=0）は母集団にも auto 分類にも入らない", () => {
    const free = model("free", { input: 0, output: 0 });
    const a = model("a", { input: 1, output: 1 });
    const out = classifySet([free, a]);
    expect(out.get(free)).toEqual({ tier: "medium", source: "unclassified" });
    // 母集団は a のみ → a は p=0.5 → medium
    expect(out.get(a)).toMatchObject({ tier: "medium", source: "auto" });
  });
});

describe("buildAutoTierContext", () => {
  it("pricing 欠落・価格ゼロの model は母集団から除外される", () => {
    const ctx = buildAutoTierContext([
      model("unpriced"),
      model("zero", { input: 0, output: 0 }),
      model("a", { input: 2, output: 4 }),
    ]);
    expect(ctx.costs).toEqual([6]); // 2 + 4 * 1.0
    expect(ctx.outputPerInput).toBe(DEFAULT_OUTPUT_PER_INPUT);
  });
});

describe("classifyModel unclassified", () => {
  it("missing cost → medium / unclassified", () => {
    const ctx = buildAutoTierContext([model("a", { input: 1, output: 1 })]);
    expect(classifyModel(model("unmatched-model-xyz"), EMPTY, { autoContext: ctx })).toEqual({
      tier: "medium",
      source: "unclassified",
    });
  });

  it("zero cost は free の観測だが自動で low にしない → medium / unclassified", () => {
    const ctx = buildAutoTierContext([model("a", { input: 1, output: 1 })]);
    expect(
      classifyModel(model("unmatched-model-xyz", { input: 0, output: 0 }), EMPTY, {
        autoContext: ctx,
      }),
    ).toEqual({ tier: "medium", source: "unclassified" });
  });
});
