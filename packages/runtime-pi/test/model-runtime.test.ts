import { CheaperError, type ModelConfig } from "@cheaper/core";
import type { Api, Model } from "@earendil-works/pi-ai";
import { describe, expect, it } from "vitest";
import { type ModelCatalog, resolvePiModel, toModelInfo } from "../src/model-runtime.js";

function piModel(provider: string, id: string, maxTokens = 8192): Model<Api> {
  return {
    id,
    name: id,
    api: "fake-api",
    provider,
    baseUrl: "http://localhost",
    reasoning: false,
    input: ["text"],
    cost: { input: 1, output: 2, cacheRead: 0, cacheWrite: 0 },
    contextWindow: 128000,
    maxTokens,
  };
}

const config: ModelConfig = { overrides: {}, preferences: {} };

function catalog(models: Model<Api>[]): ModelCatalog {
  return {
    getAvailable: async () => models,
    getModel: (p, id) => models.find((m) => m.provider === p && m.id === id),
  };
}

describe("toModelInfo", () => {
  it("converts Pi Model to core ModelInfo minimal shape", () => {
    expect(toModelInfo(piModel("p", "m", 4096))).toEqual({
      provider: "p",
      id: "m",
      cost: { input: 1, output: 2 },
      maxTokens: 4096,
    });
  });
});

describe("resolvePiModel", () => {
  it("resolves a tier-matched Pi model", async () => {
    const cat = catalog([piModel("anthropic", "claude-haiku-4")]);
    const r = await resolvePiModel({
      catalog: cat,
      requestedTier: "low",
      policy: { maxTier: "high", tierFallback: "none" },
      config,
    });
    expect(r.tier).toBe("low");
    expect(r.model.id).toBe("claude-haiku-4");
    expect(r.model.provider).toBe("anthropic");
    expect(r.info).toEqual({
      provider: "anthropic",
      id: "claude-haiku-4",
      cost: { input: 1, output: 2 },
      maxTokens: 8192,
    });
  });

  it("throws MODEL_TIER_UNAVAILABLE with noAuthenticatedModels on empty availability", async () => {
    const err = await resolvePiModel({
      catalog: catalog([]),
      requestedTier: "low",
      policy: { maxTier: "high", tierFallback: "higher" },
      config,
    }).then(
      () => undefined,
      (e: unknown) => e,
    );
    expect(err).toBeInstanceOf(CheaperError);
    const ce = err as CheaperError;
    expect(ce.code).toBe("MODEL_TIER_UNAVAILABLE");
    expect((ce.details as { noAuthenticatedModels?: boolean }).noAuthenticatedModels).toBe(true);
  });

  it("applies tierFallback=higher when no candidate in requested tier", async () => {
    const cat = catalog([piModel("anthropic", "claude-sonnet-4")]);
    const r = await resolvePiModel({
      catalog: cat,
      requestedTier: "low",
      policy: { maxTier: "high", tierFallback: "higher" },
      config,
    });
    expect(r.tier).toBe("medium");
    expect(r.model.id).toBe("claude-sonnet-4");
  });

  it("throws POLICY_VIOLATION when requested tier exceeds maxTier", async () => {
    const err = await resolvePiModel({
      catalog: catalog([piModel("a", "m")]),
      requestedTier: "high",
      policy: { maxTier: "low", tierFallback: "none" },
      config,
    }).then(
      () => undefined,
      (e: unknown) => e,
    );
    expect((err as CheaperError).code).toBe("POLICY_VIOLATION");
  });
});
