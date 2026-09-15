import { describe, expect, it } from "vitest";
import type { CheaperError } from "../../src/errors.js";
import type { ModelInfo } from "../../src/model/classify.js";
import type { ModelConfig } from "../../src/model/config.js";
import { resolveModel } from "../../src/model/resolve.js";
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
