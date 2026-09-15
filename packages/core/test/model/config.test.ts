import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { loadModelConfig } from "../../src/model/config.js";
import { cleanupTmpDir, makeTmpDir } from "../fixtures/tmp.js";

describe("loadModelConfig", () => {
  let dir: string;
  let userConfigDir: string;
  let projectDir: string;
  beforeEach(async () => {
    dir = await makeTmpDir();
    userConfigDir = path.join(dir, "user");
    projectDir = path.join(dir, "proj", ".cheaper");
    await mkdir(userConfigDir, { recursive: true });
    await mkdir(projectDir, { recursive: true });
  });
  afterEach(async () => {
    await cleanupTmpDir(dir);
  });

  it("returns empty config when no files exist", async () => {
    const config = await loadModelConfig({ userConfigDir, projectDir });
    expect(config).toEqual({ overrides: {}, preferences: {} });
  });

  it("loads user models.yaml", async () => {
    await writeFile(
      path.join(userConfigDir, "models.yaml"),
      `version: 1
overrides:
  openai/gpt-5-mini: low
preferences:
  low: ["anthropic/claude-haiku-4-5", "openai/*"]
`,
    );
    const config = await loadModelConfig({ userConfigDir, projectDir });
    expect(config.overrides).toEqual({ "openai/gpt-5-mini": "low" });
    expect(config.preferences.low).toEqual(["anthropic/claude-haiku-4-5", "openai/*"]);
  });

  it("merges user then project (overrides deep merge, preferences array replace)", async () => {
    await writeFile(
      path.join(userConfigDir, "models.yaml"),
      `version: 1
overrides:
  openai/gpt-5-mini: low
  anthropic/claude-sonnet-4: medium
preferences:
  low: ["openai/*"]
  medium: ["a/x"]
`,
    );
    await writeFile(
      path.join(projectDir, "models.yaml"),
      `version: 1
overrides:
  anthropic/claude-sonnet-4: high
preferences:
  low: ["anthropic/claude-haiku-4-5"]
`,
    );
    const config = await loadModelConfig({ userConfigDir, projectDir });
    expect(config.overrides).toEqual({
      "openai/gpt-5-mini": "low",
      "anthropic/claude-sonnet-4": "high",
    });
    // project replaces the whole array for low; user-only medium stays
    expect(config.preferences.low).toEqual(["anthropic/claude-haiku-4-5"]);
    expect(config.preferences.medium).toEqual(["a/x"]);
  });

  it("rejects missing version", async () => {
    await writeFile(path.join(userConfigDir, "models.yaml"), "overrides: {}\n");
    await expect(loadModelConfig({ userConfigDir, projectDir })).rejects.toMatchObject({
      code: "INVALID_MODEL_CONFIG",
    });
  });

  it("rejects unknown top-level field", async () => {
    await writeFile(path.join(projectDir, "models.yaml"), "version: 1\nbogus: 1\n");
    await expect(loadModelConfig({ userConfigDir, projectDir })).rejects.toMatchObject({
      code: "INVALID_MODEL_CONFIG",
    });
  });

  it("rejects invalid tier value in overrides", async () => {
    await writeFile(
      path.join(userConfigDir, "models.yaml"),
      "version: 1\noverrides:\n  p/m: ultra\n",
    );
    await expect(loadModelConfig({ userConfigDir, projectDir })).rejects.toMatchObject({
      code: "INVALID_MODEL_CONFIG",
    });
  });

  it("rejects unknown tier key in preferences", async () => {
    await writeFile(
      path.join(userConfigDir, "models.yaml"),
      'version: 1\npreferences:\n  extreme: ["p/*"]\n',
    );
    await expect(loadModelConfig({ userConfigDir, projectDir })).rejects.toMatchObject({
      code: "INVALID_MODEL_CONFIG",
    });
  });

  it("unreadable models.yaml (a directory) → INVALID_MODEL_CONFIG read-failed", async () => {
    // ENOENT/ENOTDIR 以外の読み込み失敗は silent skip しない。
    await mkdir(path.join(userConfigDir, "models.yaml"));
    await expect(loadModelConfig({ userConfigDir, projectDir })).rejects.toMatchObject({
      code: "INVALID_MODEL_CONFIG",
      details: { reason: "read-failed" },
    });
  });

  it("rejects non-object yaml and yaml parse errors", async () => {
    await writeFile(path.join(userConfigDir, "models.yaml"), "- a\n- b\n");
    await expect(loadModelConfig({ userConfigDir, projectDir })).rejects.toMatchObject({
      code: "INVALID_MODEL_CONFIG",
    });
    await writeFile(path.join(userConfigDir, "models.yaml"), "version: [unclosed\n");
    await expect(loadModelConfig({ userConfigDir, projectDir })).rejects.toMatchObject({
      code: "INVALID_MODEL_CONFIG",
    });
  });
});
