import { describe, expect, it } from "vitest";
import { compareTier, isTier, nextTier, TIERS } from "../../src/model/tier.js";

describe("tier", () => {
  it("orders low < medium < high", () => {
    expect(TIERS).toEqual(["low", "medium", "high"]);
    expect(compareTier("low", "medium")).toBeLessThan(0);
    expect(compareTier("high", "low")).toBeGreaterThan(0);
    expect(compareTier("medium", "medium")).toBe(0);
  });
  it("isTier guards runtime values", () => {
    expect(isTier("low")).toBe(true);
    expect(isTier("high")).toBe(true);
    expect(isTier("ultra")).toBe(false);
    expect(isTier(3)).toBe(false);
    expect(isTier(undefined)).toBe(false);
  });
  it("nextTier steps up and ends at high", () => {
    expect(nextTier("low")).toBe("medium");
    expect(nextTier("medium")).toBe("high");
    expect(nextTier("high")).toBeUndefined();
  });
});
