import type { ResourceLoader } from "@earendil-works/pi-coding-agent";
import { describe, expect, it } from "vitest";
import { CheaperResourceLoader } from "../src/resource-loader.js";

describe("CheaperResourceLoader", () => {
  it("implements the ResourceLoader interface", () => {
    const loader: ResourceLoader = new CheaperResourceLoader({ systemPrompt: "sp" });
    // 11 メソッドすべてが存在する
    for (const m of [
      "getExtensions",
      "getSkills",
      "getPrompts",
      "getThemes",
      "getAgentsFiles",
      "getSystemPrompt",
      "getSystemPromptSource",
      "getAppendSystemPrompt",
      "getAppendSystemPromptSources",
      "extendResources",
      "reload",
    ] as const) {
      expect(typeof loader[m]).toBe("function");
    }
  });

  it("returns the worker system prompt and append sections", () => {
    const loader = new CheaperResourceLoader({
      systemPrompt: "You are a worker.",
      appendSystemPrompt: ["Output contract: return JSON only."],
    });
    expect(loader.getSystemPrompt()).toBe("You are a worker.");
    expect(loader.getAppendSystemPrompt()).toEqual(["Output contract: return JSON only."]);
  });

  it("defaults appendSystemPrompt to empty", () => {
    const loader = new CheaperResourceLoader({ systemPrompt: "sp" });
    expect(loader.getAppendSystemPrompt()).toEqual([]);
  });

  it("returns empty resources for extensions/skills/prompts/themes/agentsFiles", () => {
    const loader = new CheaperResourceLoader({ systemPrompt: "sp" });
    const ext = loader.getExtensions();
    expect(ext.extensions).toEqual([]);
    expect(ext.errors).toEqual([]);
    expect(ext.runtime).toBeDefined();
    expect(loader.getSkills()).toEqual({ skills: [], diagnostics: [] });
    expect(loader.getPrompts()).toEqual({ prompts: [], diagnostics: [] });
    expect(loader.getThemes()).toEqual({ themes: [], diagnostics: [] });
    expect(loader.getAgentsFiles()).toEqual({ agentsFiles: [] });
    expect(loader.getSystemPromptSource()).toBeUndefined();
    expect(loader.getAppendSystemPromptSources()).toEqual([]);
  });

  it("extendResources and reload are safe no-ops", async () => {
    const loader = new CheaperResourceLoader({ systemPrompt: "sp" });
    expect(() => loader.extendResources({})).not.toThrow();
    await expect(loader.reload()).resolves.toBeUndefined();
  });
});
