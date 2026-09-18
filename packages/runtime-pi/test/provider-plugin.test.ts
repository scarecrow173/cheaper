import { mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import type { ModelRuntime } from "@earendil-works/pi-coding-agent";
import { describe, expect, it } from "vitest";
import { listProviderAuth } from "../src/auth.js";
import { createModelRuntime } from "../src/model-runtime.js";
import {
  bundledProviderPlugins,
  type CheaperProviderPlugin,
  type ProviderPluginContext,
} from "../src/provider-plugin.js";
import { createFakePi } from "../src/testsupport.js";

function tmpEnv() {
  return mkdtemp(path.join(tmpdir(), "cheaper-pp-")).then((dir) => ({
    dir,
    env: { CHEAPER_CONFIG_DIR: dir },
  }));
}

describe("bundledProviderPlugins", () => {
  it("bundles the Devin Local provider plugin", () => {
    expect(bundledProviderPlugins().map((p) => p.id)).toEqual(["devin"]);
  });
});

describe("createModelRuntime provider plugins", () => {
  it("passes the runtime and an env/userConfigDir context to register", async () => {
    const { dir, env } = await tmpEnv();
    let seen: { runtime: ModelRuntime; context: ProviderPluginContext } | undefined;
    const plugin: CheaperProviderPlugin = {
      id: "ctx-recorder",
      register: (runtime, context) => {
        seen = { runtime, context };
      },
    };
    const runtime = await createModelRuntime({ env, providerPlugins: [plugin] });

    expect(seen?.runtime).toBe(runtime);
    expect(seen?.context.env).toBe(env);
    expect(seen?.context.userConfigDir).toBe(dir);
  });

  it("exposes a plugin-registered provider in providers, availability, and auth listing", async () => {
    const { env } = await tmpEnv();
    const fake = createFakePi();
    const plugin: CheaperProviderPlugin = {
      id: "fake",
      register: (runtime) => runtime.registerNativeProvider(fake.provider),
    };
    const runtime = await createModelRuntime({ env, providerPlugins: [plugin] });

    // plugin registration が最初の availability 解決より前に完了している
    //（create 時 refresh を遅延し、register 後に await refresh する）。
    expect(runtime.getProviders().map((p) => p.id)).toContain("cheaper-fake");
    const available = await runtime.getAvailable();
    expect(available.some((m) => m.provider === "cheaper-fake" && m.id === "fake-model")).toBe(
      true,
    );
    const auth = await listProviderAuth(runtime);
    expect(auth.map((p) => p.providerId)).toContain("cheaper-fake");
  });

  it("runs without plugins identically to the default path", async () => {
    const { env } = await tmpEnv();
    const runtime = await createModelRuntime({ env, providerPlugins: [] });
    expect(runtime.getProviders().length).toBeGreaterThan(0);
    expect(runtime.getProviders().map((p) => p.id)).not.toContain("cheaper-fake");
  });

  it("propagates a plugin register failure instead of returning a half-registered runtime", async () => {
    const { env } = await tmpEnv();
    const plugin: CheaperProviderPlugin = {
      id: "broken",
      register: () => {
        throw new Error("plugin boom");
      },
    };
    await expect(createModelRuntime({ env, providerPlugins: [plugin] })).rejects.toThrow(
      "plugin boom",
    );
  });

  it("registers plugins in array order", async () => {
    const { env } = await tmpEnv();
    const order: string[] = [];
    const mk = (id: string): CheaperProviderPlugin => ({
      id,
      register: () => {
        order.push(id);
      },
    });
    await createModelRuntime({ env, providerPlugins: [mk("a"), mk("b"), mk("c")] });
    expect(order).toEqual(["a", "b", "c"]);
  });
});
