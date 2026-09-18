import { mkdtemp, stat } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { CheaperError, type ModelConfig } from "@cheaper/core";
import type { Api, Model } from "@earendil-works/pi-ai";
import { ModelRuntime } from "@earendil-works/pi-coding-agent";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  createModelRuntime,
  type ModelCatalog,
  resolvePiModel,
  resolvePiStatePaths,
  toModelInfo,
} from "../src/model-runtime.js";

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

describe("resolvePiStatePaths", () => {
  it("places auth/models/models-store under <userConfigDir>/pi", () => {
    const p = resolvePiStatePaths("/cfg");
    expect(p.piDir).toBe(path.join("/cfg", "pi"));
    expect(p.authPath).toBe(path.join("/cfg", "pi", "auth.json"));
    expect(p.modelsPath).toBe(path.join("/cfg", "pi", "models.json"));
    expect(p.modelsStorePath).toBe(path.join("/cfg", "pi", "models-store.json"));
  });
});

describe("createModelRuntime", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  function mockCreate() {
    const refresh = vi.fn().mockResolvedValue({ aborted: false, errors: new Map() });
    const spy = vi
      .spyOn(ModelRuntime, "create")
      .mockResolvedValue({ refresh, registerProvider: () => {} } as unknown as ModelRuntime);
    return { spy, refresh };
  }

  it("creates <CHEAPER_CONFIG_DIR>/pi and passes its paths to ModelRuntime.create", async () => {
    const dir = await mkdtemp(path.join(tmpdir(), "cheaper-mr-"));
    const { spy } = mockCreate();
    await createModelRuntime({ env: { CHEAPER_CONFIG_DIR: dir } });

    expect(spy).toHaveBeenCalledWith(
      expect.objectContaining({
        authPath: path.join(dir, "pi", "auth.json"),
        modelsPath: path.join(dir, "pi", "models.json"),
        modelsStorePath: path.join(dir, "pi", "models-store.json"),
      }),
    );
    const st = await stat(path.join(dir, "pi"));
    expect(st.isDirectory()).toBe(true);
    if (process.platform !== "win32") {
      expect(st.mode & 0o777).toBe(0o700);
    }
  });

  it("lets caller-provided paths override the cheaper defaults", async () => {
    const dir = await mkdtemp(path.join(tmpdir(), "cheaper-mr-"));
    const { spy } = mockCreate();
    await createModelRuntime({
      env: { CHEAPER_CONFIG_DIR: dir },
      authPath: path.join(dir, "custom-auth.json"),
      modelsPath: null,
    });
    expect(spy).toHaveBeenCalledWith(
      expect.objectContaining({
        authPath: path.join(dir, "custom-auth.json"),
        modelsPath: null,
        modelsStorePath: path.join(dir, "pi", "models-store.json"),
      }),
    );
  });

  it("resolves the config dir via HOME when CHEAPER_CONFIG_DIR is unset", async () => {
    const dir = await mkdtemp(path.join(tmpdir(), "cheaper-mr-"));
    const { spy } = mockCreate();
    await createModelRuntime({ env: { HOME: dir, USERPROFILE: dir } });
    const expected = path.join(dir, ".config", "cheaper", "pi");
    expect(spy).toHaveBeenCalledWith(
      expect.objectContaining({ authPath: path.join(expected, "auth.json") }),
    );
    expect((await stat(expected)).isDirectory()).toBe(true);
  });

  it("defers the create refresh, then registers plugins before awaiting refresh", async () => {
    const dir = await mkdtemp(path.join(tmpdir(), "cheaper-mr-"));
    const { spy, refresh } = mockCreate();
    const order: string[] = [];
    const plugin = {
      id: "recorder",
      register: vi.fn(() => {
        order.push("register");
      }),
    };
    refresh.mockImplementation(() => {
      order.push("refresh");
      return Promise.resolve({ aborted: false, errors: new Map() });
    });
    await createModelRuntime({
      env: { CHEAPER_CONFIG_DIR: dir },
      providerPlugins: [plugin],
    });

    expect(spy).toHaveBeenCalledWith(expect.objectContaining({ refreshOnCreate: false }));
    expect(order).toEqual(["register", "refresh"]);
  });

  it("skips the manual refresh when caller sets refreshOnCreate: false", async () => {
    const dir = await mkdtemp(path.join(tmpdir(), "cheaper-mr-"));
    const { refresh } = mockCreate();
    await createModelRuntime({
      env: { CHEAPER_CONFIG_DIR: dir },
      refreshOnCreate: false,
      providerPlugins: [],
    });
    expect(refresh).not.toHaveBeenCalled();
  });

  it("mirrors create-time allowNetwork from allowModelNetwork and PI_OFFLINE", async () => {
    const dir = await mkdtemp(path.join(tmpdir(), "cheaper-mr-"));
    const { refresh } = mockCreate();
    await createModelRuntime({
      env: { CHEAPER_CONFIG_DIR: dir, PI_OFFLINE: "1" },
      allowModelNetwork: true,
      providerPlugins: [],
    });
    expect(refresh).toHaveBeenCalledWith(expect.objectContaining({ allowNetwork: false }));
  });
});
