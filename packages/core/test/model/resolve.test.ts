import { describe, expect, it } from "vitest";
import type { CheaperError } from "../../src/errors.js";
import type { ModelInfo } from "../../src/model/classify.js";
import type { ModelConfig } from "../../src/model/config.js";
import { resolveExplicitModel, resolveModel } from "../../src/model/resolve.js";
import type { Tier } from "../../src/model/tier.js";

const NO_PREF: ModelConfig = { overrides: {}, preferences: {} };

function m(provider: string, id: string, cost?: { input: number; output: number }): ModelInfo {
  const info: ModelInfo = { provider, id };
  if (cost !== undefined) info.cost = cost;
  return info;
}

/** overrides で分類を固定する config を作る。 */
function cfg(
  overrides: Record<string, Tier>,
  preferences: Partial<Record<Tier, string[]>> = {},
): ModelConfig {
  return { overrides, preferences };
}

describe("resolveModel", () => {
  it("empty available → MODEL_TIER_UNAVAILABLE with noAuthenticatedModels", () => {
    try {
      resolveModel({
        available: [],
        requestedTier: "low",
        policy: { maxTier: "high", tierFallback: "higher" },
        config: NO_PREF,
      });
      expect.unreachable();
    } catch (e) {
      const err = e as CheaperError;
      expect(err.code).toBe("MODEL_TIER_UNAVAILABLE");
      expect(err.exitCode).toBe(5);
      expect(err.details?.noAuthenticatedModels).toBe(true);
    }
  });

  it("requested > maxTier → POLICY_VIOLATION", () => {
    try {
      resolveModel({
        available: [m("p", "x")],
        requestedTier: "high",
        policy: { maxTier: "medium", tierFallback: "none" },
        config: NO_PREF,
      });
      expect.unreachable();
    } catch (e) {
      const err = e as CheaperError;
      expect(err.code).toBe("POLICY_VIOLATION");
      expect(err.details).toMatchObject({ requestedTier: "high", maxTier: "medium" });
    }
  });

  it("selects the cheapest candidate of the requested tier (cost asc)", () => {
    const config = cfg({
      "p/expensive": "low",
      "p/cheap": "low",
      "p/mid": "low",
    });
    const available = [
      m("p", "expensive", { input: 1, output: 9 }),
      m("p", "mid", { input: 1, output: 4 }),
      m("p", "cheap", { input: 0.5, output: 1 }),
    ];
    const out = resolveModel({
      available,
      requestedTier: "low",
      policy: { maxTier: "high", tierFallback: "none" },
      config,
    });
    expect(out.tier).toBe("low");
    expect(out.model.id).toBe("cheap");
  });

  it("models without cost sort after costed ones; ties break by provider then id", () => {
    const config = cfg({
      "b/same": "low",
      "a/same": "low",
      "z/nocost": "low",
    });
    const out = resolveModel({
      available: [
        m("z", "nocost"),
        m("b", "same", { input: 1, output: 1 }),
        m("a", "same", { input: 1, output: 1 }),
      ],
      requestedTier: "low",
      policy: { maxTier: "high", tierFallback: "none" },
      config,
    });
    // a/same and b/same both cost 2; provider asc → a/same wins. nocost is Infinity → last.
    expect(out.model.provider).toBe("a");
    expect(out.model.id).toBe("same");
  });

  it("preferences patterns order candidates before cost", () => {
    const config = cfg(
      { "p/cheap": "low", "q/wild": "low", "q/exact": "low" },
      { low: ["q/exact", "p/*"] },
    );
    const out = resolveModel({
      available: [
        m("p", "cheap", { input: 0, output: 0.1 }),
        m("q", "wild", { input: 0, output: 0.2 }),
        m("q", "exact", { input: 100, output: 100 }),
      ],
      requestedTier: "low",
      policy: { maxTier: "high", tierFallback: "none" },
      config,
    });
    // "q/exact" is the first preference pattern even though it is the most expensive.
    expect(out.model.id).toBe("exact");
  });

  it("tierFallback none → MODEL_TIER_UNAVAILABLE with candidatesPerTier", () => {
    const config = cfg({ "p/highm": "high", "p/medm": "medium" });
    try {
      resolveModel({
        available: [m("p", "highm"), m("p", "medm")],
        requestedTier: "low",
        policy: { maxTier: "high", tierFallback: "none" },
        config,
      });
      expect.unreachable();
    } catch (e) {
      const err = e as CheaperError;
      expect(err.code).toBe("MODEL_TIER_UNAVAILABLE");
      expect(err.details?.candidatesPerTier).toEqual({ low: 0, medium: 1, high: 1 });
    }
  });

  it("tierFallback higher climbs one tier at a time up to maxTier", () => {
    const config = cfg({ "p/medm": "medium", "p/highm": "high" });
    const out = resolveModel({
      available: [m("p", "medm"), m("p", "highm")],
      requestedTier: "low",
      policy: { maxTier: "high", tierFallback: "higher" },
      config,
    });
    expect(out.tier).toBe("medium");
    expect(out.model.id).toBe("medm");
  });

  it("tierFallback higher stops at maxTier", () => {
    const config = cfg({ "p/highm": "high" });
    try {
      resolveModel({
        available: [m("p", "highm")],
        requestedTier: "low",
        policy: { maxTier: "medium", tierFallback: "higher" },
        config,
      });
      expect.unreachable();
    } catch (e) {
      const err = e as CheaperError;
      expect(err.code).toBe("MODEL_TIER_UNAVAILABLE");
      expect(err.details?.candidatesPerTier).toEqual({ low: 0, medium: 0, high: 1 });
    }
  });

  it("resolved model に selection/tierSource/pricing を返す", () => {
    const out = resolveModel({
      available: [m("p", "x", { input: 0.1, output: 0.5 })],
      requestedTier: "medium",
      policy: { maxTier: "high", tierFallback: "none" },
      config: NO_PREF,
    });
    expect(out.selection).toBe("tier");
    // 母集団 1 件 → p=0.5 → medium / auto（issue #46）
    expect(out.tier).toBe("medium");
    expect(out.tierSource).toBe("auto");
    expect(out.auto).toMatchObject({
      effectiveCost: 0.6,
      effectiveCostPercentile: 0.5,
      outputPerInput: 1,
    });
    expect(out.pricing).toMatchObject({ input: 0.1, output: 0.5, source: "runtime-catalog" });
  });

  it("worker の outputPerInput が auto 分類に反映される", () => {
    const available = [
      m("p", "input-heavy", { input: 10, output: 1 }),
      m("p", "output-heavy", { input: 1, output: 10 }),
    ];
    const reader = resolveModel({
      available,
      requestedTier: "low",
      policy: { maxTier: "high", tierFallback: "none" },
      config: NO_PREF,
      outputPerInput: 0,
    });
    expect(reader.model.id).toBe("output-heavy");
    const writer = resolveModel({
      available,
      requestedTier: "low",
      policy: { maxTier: "high", tierFallback: "none" },
      config: NO_PREF,
      outputPerInput: 10,
    });
    expect(writer.model.id).toBe("input-heavy");
  });

  it("exclude に一致する model は candidate から外れて選ばれない", () => {
    const config: ModelConfig = {
      overrides: { "p/excluded": "low", "p/kept": "low" },
      preferences: {},
      exclude: ["p/excluded"],
    };
    const out = resolveModel({
      available: [
        m("p", "excluded", { input: 0.01, output: 0.01 }),
        m("p", "kept", { input: 5, output: 1 }),
      ],
      requestedTier: "low",
      policy: { maxTier: "high", tierFallback: "none" },
      config,
    });
    expect(out.model.id).toBe("kept");
  });

  it("provider/* wildcard の exclude も candidate から外す", () => {
    const config: ModelConfig = {
      overrides: { "p/x": "low", "q/y": "low" },
      preferences: {},
      exclude: ["p/*"],
    };
    const out = resolveModel({
      available: [m("p", "x"), m("q", "y")],
      requestedTier: "low",
      policy: { maxTier: "high", tierFallback: "none" },
      config,
    });
    expect(out.model.provider).toBe("q");
  });

  it("全 model が exclude → MODEL_TIER_UNAVAILABLE", () => {
    const config: ModelConfig = { overrides: {}, preferences: {}, exclude: ["p/*"] };
    expect(() =>
      resolveModel({
        available: [m("p", "x")],
        requestedTier: "low",
        policy: { maxTier: "high", tierFallback: "none" },
        config,
      }),
    ).toThrow(/no authenticated models/);
  });

  it("never falls back to lower tiers", () => {
    const config = cfg({ "p/lowm": "low" });
    // request medium, only low exists, fallback higher → high has nothing → unavailable
    expect(() =>
      resolveModel({
        available: [m("p", "lowm")],
        requestedTier: "medium",
        policy: { maxTier: "high", tierFallback: "higher" },
        config,
      }),
    ).toThrow();
  });
});

describe("resolveExplicitModel", () => {
  const POLICY = { maxTier: "high" as const };

  it("available な model を selection=explicit で返す（tier 候補選択を bypass）", () => {
    const out = resolveExplicitModel({
      available: [
        m("p", "x", { input: 1, output: 30 }),
        m("q", "cheap", { input: 0.1, output: 0.1 }),
        m("q", "mid", { input: 1, output: 1 }),
      ],
      ref: { provider: "p", id: "x" },
      policy: POLICY,
      config: NO_PREF,
    });
    expect(out.model).toMatchObject({ provider: "p", id: "x" });
    expect(out.selection).toBe("explicit");
    // 母集団 3 件で最高 effectiveCost → high（issue #46）
    expect(out.tier).toBe("high");
    expect(out.tierSource).toBe("auto");
    expect(out.auto?.effectiveCostPercentile).toBeCloseTo(5 / 6);
    expect(out.pricing).toMatchObject({ source: "runtime-catalog" });
  });

  it("exclude 一致 → POLICY_VIOLATION model-excluded（--model でも突破不可）", () => {
    const config: ModelConfig = { overrides: {}, preferences: {}, exclude: ["openai/*"] };
    try {
      resolveExplicitModel({
        available: [m("openai", "gpt-5")],
        ref: { provider: "openai", id: "gpt-5" },
        policy: POLICY,
        config,
      });
      expect.unreachable();
    } catch (e) {
      const err = e as CheaperError;
      expect(err.code).toBe("POLICY_VIOLATION");
      expect(err.details?.reason).toBe("model-excluded");
    }
  });

  it("catalog 不在 → MODEL_TIER_UNAVAILABLE（availability を bypass しない）", () => {
    try {
      resolveExplicitModel({
        available: [m("p", "x")],
        ref: { provider: "foo", id: "bar" },
        policy: POLICY,
        config: NO_PREF,
      });
      expect.unreachable();
    } catch (e) {
      const err = e as CheaperError;
      expect(err.code).toBe("MODEL_TIER_UNAVAILABLE");
      expect(err.details?.reason).toBe("model-unavailable");
    }
  });

  it("effective tier > maxTier → POLICY_VIOLATION（policy を bypass しない）", () => {
    try {
      resolveExplicitModel({
        available: [
          m("p", "x", { input: 1, output: 30 }),
          m("q", "cheap", { input: 0.1, output: 0.1 }),
          m("q", "mid", { input: 1, output: 1 }),
        ],
        ref: { provider: "p", id: "x" },
        policy: { maxTier: "medium" },
        config: NO_PREF,
      });
      expect.unreachable();
    } catch (e) {
      const err = e as CheaperError;
      expect(err.code).toBe("POLICY_VIOLATION");
      expect(err.details?.reason).toBe("model-exceeds-max-tier");
    }
  });

  it("override tier でも maxTier を超えれば POLICY_VIOLATION", () => {
    const config: ModelConfig = { overrides: { "p/x": "high" }, preferences: {} };
    expect(() =>
      resolveExplicitModel({
        available: [m("p", "x", { input: 0.1, output: 0.5 })],
        ref: { provider: "p", id: "x" },
        policy: { maxTier: "medium" },
        config,
      }),
    ).toThrow(/exceeds policy maxTier/);
  });
});
