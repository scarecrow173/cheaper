import path from "node:path";
import { describe, expect, it } from "vitest";
import { resolveConfigDirs, resolveUserConfigDir } from "../../src/config/dirs.js";

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
  it("resolves homedir from env.USERPROFILE / env.HOME for test isolation", () => {
    const noHomedir = { cwd: "/w", builtinDir: "/b" };
    expect(
      resolveConfigDirs({ ...noHomedir, env: { USERPROFILE: "/up", HOME: "/h" } }).userConfigDir,
    ).toBe(path.join("/up", ".config", "cheaper"));
    expect(resolveConfigDirs({ ...noHomedir, env: { HOME: "/h" } }).userConfigDir).toBe(
      path.join("/h", ".config", "cheaper"),
    );
  });
  it("sets projectDir to <cwd>/.cheaper and passes builtinDir through", () => {
    const d = resolveConfigDirs({ ...opts, env: {} });
    expect(d.projectDir).toBe(path.join("/work/proj", ".cheaper"));
    expect(d.builtinDir).toBe("/builtin");
  });
});

describe("resolveUserConfigDir", () => {
  it("prefers CHEAPER_CONFIG_DIR", () => {
    expect(
      resolveUserConfigDir({
        env: { CHEAPER_CONFIG_DIR: "/custom", XDG_CONFIG_HOME: "/xdg" },
      }),
    ).toBe("/custom");
  });
  it("falls back to XDG_CONFIG_HOME/cheaper", () => {
    expect(resolveUserConfigDir({ env: { XDG_CONFIG_HOME: "/xdg" }, homedir: "/home/u" })).toBe(
      path.join("/xdg", "cheaper"),
    );
  });
  it("falls back to ~/.config/cheaper", () => {
    expect(resolveUserConfigDir({ env: {}, homedir: "/home/u" })).toBe(
      path.join("/home/u", ".config", "cheaper"),
    );
  });
  it("resolves homedir from env.USERPROFILE / env.HOME for test isolation", () => {
    expect(resolveUserConfigDir({ env: { USERPROFILE: "/up", HOME: "/h" } })).toBe(
      path.join("/up", ".config", "cheaper"),
    );
    expect(resolveUserConfigDir({ env: { HOME: "/h" } })).toBe(
      path.join("/h", ".config", "cheaper"),
    );
  });
});
