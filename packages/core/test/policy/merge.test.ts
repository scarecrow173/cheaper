import { describe, expect, it } from "vitest";
import { mergePolicyLayers } from "../../src/policy/merge.js";

describe("mergePolicyLayers", () => {
  it("deep merges nested objects", () => {
    const base = { execution: { timeout: "120s", maxTurns: 8 } };
    const override = { execution: { maxTurns: 12 } };
    expect(mergePolicyLayers(base, override)).toEqual({
      execution: { timeout: "120s", maxTurns: 12 },
    });
  });

  it("replaces scalars", () => {
    const base = { execution: { timeout: "120s", maxTurns: 8 } };
    const override = { execution: { timeout: "180s" } };
    expect(mergePolicyLayers(base, override)).toEqual({
      execution: { timeout: "180s", maxTurns: 8 },
    });
  });

  it("replaces arrays wholesale (no union)", () => {
    const base = { tools: { allow: ["read", "grep", "write"] } };
    const override = { tools: { allow: ["read"] } };
    expect(mergePolicyLayers(base, override)).toEqual({ tools: { allow: ["read"] } });
  });

  it("adds keys present only in override and keeps base-only keys", () => {
    const base = { a: 1, b: { c: 2 } };
    const override = { b: { d: 3 }, e: 4 };
    expect(mergePolicyLayers(base, override)).toEqual({ a: 1, b: { c: 2, d: 3 }, e: 4 });
  });

  it("replaces object with scalar and vice versa", () => {
    expect(mergePolicyLayers({ a: { x: 1 } }, { a: 5 })).toEqual({ a: 5 });
    expect(mergePolicyLayers({ a: 5 }, { a: { x: 1 } })).toEqual({ a: { x: 1 } });
  });

  it("does not mutate inputs", () => {
    const base = { execution: { timeout: "120s" }, tools: { allow: ["read"] } };
    const override = { execution: { maxTurns: 4 }, tools: { allow: ["ls"] } };
    mergePolicyLayers(base, override);
    expect(base).toEqual({ execution: { timeout: "120s" }, tools: { allow: ["read"] } });
    expect(override).toEqual({ execution: { maxTurns: 4 }, tools: { allow: ["ls"] } });
  });

  it("handles empty override and empty base", () => {
    expect(mergePolicyLayers({ a: 1 }, {})).toEqual({ a: 1 });
    expect(mergePolicyLayers({}, { a: 1 })).toEqual({ a: 1 });
    expect(mergePolicyLayers({}, {})).toEqual({});
  });
});
