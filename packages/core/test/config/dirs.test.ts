import path from "node:path";
import { describe, expect, it } from "vitest";
import { resolveConfigDirs } from "../../src/config/dirs.js";

const opts = { cwd: "/work/proj", builtinDir: "/builtin", homedir: "/home/u" };

describe("resolveConfigDirs", () => {
  it("prefers CHEAPER_CONFIG_DIR", () => {
    const d = resolveConfigDirs({
      ...opts,
      env: { CHEAPER_CONFIG_DIR: "/custom", XDG_CONFIG_HOME: "/xdg" },
    });
    expect(d.userConfigDir).toBe("/custom");
  });
  it("falls back to XDG_CONFIG_HOME/cheaper", () => {
    const d = resolveConfigDirs({ ...opts, env: { XDG_CONFIG_HOME: "/xdg" } });
    expect(d.userConfigDir).toBe(path.join("/xdg", "cheaper"));
  });
  it("falls back to ~/.config/cheaper", () => {
    const d = resolveConfigDirs({ ...opts, env: {} });
    expect(d.userConfigDir).toBe(path.join("/home/u", ".config", "cheaper"));
  });
  it("ignores empty CHEAPER_CONFIG_DIR and XDG_CONFIG_HOME", () => {
    const d = resolveConfigDirs({
      ...opts,
      env: { CHEAPER_CONFIG_DIR: "", XDG_CONFIG_HOME: "" },
    });
    expect(d.userConfigDir).toBe(path.join("/home/u", ".config", "cheaper"));
  });
  it("uses process env and os.homedir defaults", () => {
    const d = resolveConfigDirs({ cwd: "/w", builtinDir: "/b", env: {}, homedir: undefined });
    expect(d.userConfigDir.endsWith(path.join(".config", "cheaper"))).toBe(true);
  });
  it("sets projectDir to <cwd>/.cheaper and passes builtinDir through", () => {
    const d = resolveConfigDirs({ ...opts, env: {} });
    expect(d.projectDir).toBe(path.join("/work/proj", ".cheaper"));
    expect(d.builtinDir).toBe("/builtin");
  });
});
