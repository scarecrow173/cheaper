import { describe, expect, it } from "vitest";
import { findModelException } from "../../src/model/exceptions.js";
import { type PricingSnapshot, resolvePricing } from "../../src/model/pricing.js";
import { matchModelPattern } from "../../src/model/ref.js";

const SNAPSHOT: PricingSnapshot = {
  version: "test-v1",
  updatedAt: "2026-09-01",
  models: {
    "snap/exact": { input: 1, output: 2, cacheRead: 0.1 },
    "snap-wild/*": { input: 3, output: 4 },
  },
};

describe("resolvePricing", () => {
  it("runtime catalog の cost を優先し source=runtime-catalog を返す", () => {
    const p = resolvePricing(
      { provider: "p", id: "m", cost: { input: 5, output: 25, cacheRead: 0.5 } },
      SNAPSHOT,
    );
    expect(p).toMatchObject({
      input: 5,
      output: 25,
      cacheRead: 0.5,
      source: "runtime-catalog",
      unit: "usd-per-1m-tokens",
      currency: "USD",
    });
    expect(p?.snapshotVersion).toBeUndefined();
  });

  it("cost=0 も runtime の有効な観測として扱う", () => {
    const p = resolvePricing({ provider: "p", id: "m", cost: { input: 0, output: 0 } });
    expect(p).toMatchObject({ input: 0, output: 0, source: "runtime-catalog" });
  });

  it("runtime cost 欠落時は snapshot の exact entry で補完する", () => {
    const p = resolvePricing({ provider: "snap", id: "exact" }, SNAPSHOT);
    expect(p).toMatchObject({
      input: 1,
      output: 2,
      cacheRead: 0.1,
      source: "bundled-snapshot",
      snapshotVersion: "test-v1",
      updatedAt: "2026-09-01",
    });
  });

  it("snapshot の provider/* wildcard entry を補完に使う", () => {
    const p = resolvePricing({ provider: "snap-wild", id: "anything" }, SNAPSHOT);
    expect(p).toMatchObject({ input: 3, output: 4, source: "bundled-snapshot" });
  });

  it("runtime・snapshot 双方に無い model は unpriced（error にしない）", () => {
    expect(resolvePricing({ provider: "no", id: "pe" }, SNAPSHOT)).toBeUndefined();
  });
});

describe("findModelException", () => {
  const table = {
    "p/exact": { tier: "high" as const, reason: "r", source: "s", checkedAt: "2026-09-19" },
    "p/*": { tier: "low" as const, reason: "r", source: "s", checkedAt: "2026-09-19" },
  };

  it("exact key が wildcard より優先する", () => {
    expect(findModelException("p", "exact", table)?.tier).toBe("high");
    expect(findModelException("p", "other", table)?.tier).toBe("low");
  });

  it("一致しない model は undefined", () => {
    expect(findModelException("q", "exact", table)).toBeUndefined();
  });

  it("bundled MODEL_EXCEPTIONS は ollama/* を low に固定する", async () => {
    const { MODEL_EXCEPTIONS } = await import("../../src/model/exceptions.js");
    const e = findModelException("ollama", "qwen3", MODEL_EXCEPTIONS);
    expect(e?.tier).toBe("low");
    expect(e?.reason).toBeTruthy();
    expect(e?.source).toBeTruthy();
    expect(e?.checkedAt).toBeTruthy();
  });
});

describe("matchModelPattern", () => {
  it("exact と provider/* wildcard を解釈する", () => {
    expect(matchModelPattern("a", "m", "a/m")).toBe(true);
    expect(matchModelPattern("a", "m", "a/*")).toBe(true);
    expect(matchModelPattern("a", "m", "b/*")).toBe(false);
    expect(matchModelPattern("a", "m", "a/n")).toBe(false);
  });
});
