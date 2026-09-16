import { describe, expect, it } from "vitest";
import {
  createPathDenylist,
  DEFAULT_DENY_PATHS,
  denyPatternToRegex,
} from "../../src/workspace/deny-paths.js";

describe("denyPatternToRegex", () => {
  it.each([
    // basename pattern matches at any depth
    [".env", ".env", true],
    [".env", "config/.env", true],
    [".env", ".env.local", false],
    [".env", "envfile", false],
    // wildcard in segment
    [".env.*", ".env.local", true],
    [".env.*", "a/b/.env.production", true],
    [".env.*", ".env", false],
    ["*.pem", "certs/server.pem", true],
    ["*.pem", "certs/pem.txt", false],
    // dir pattern matches the dir itself and its contents, at any depth
    [".ssh/**", ".ssh", true],
    [".ssh/**", ".ssh/id_rsa", true],
    [".ssh/**", "home/user/.ssh/config", true],
    [".ssh/**", ".sshd_config", false],
    [".ssh/**", "ssh/file", false],
    // root-anchored pattern
    ["/config/secrets.json", "config/secrets.json", true],
    ["/config/secrets.json", "sub/config/secrets.json", false],
    // ** in the middle
    ["a/**/b.txt", "a/x/y/b.txt", true],
    ["a/**/b.txt", "a/b.txt", true],
    // ? wildcard
    ["secret?.txt", "secret1.txt", true],
    ["secret?.txt", "secret12.txt", false],
  ])("pattern %s vs %s → %s", (pattern, display, expected) => {
    expect(denyPatternToRegex(pattern).test(display)).toBe(expected);
  });
});

describe("createPathDenylist", () => {
  it("denies every DEFAULT_DENY_PATHS representative at any depth", () => {
    const list = createPathDenylist();
    for (const p of [
      ".env",
      "config/.env.local",
      "certs/server.pem",
      "keys/private.key",
      "id_rsa",
      "a/b/id_ed25519",
      ".netrc",
      "pkg/.npmrc",
      "secrets.yaml",
      ".ssh/config",
      "app/.aws/credentials",
    ]) {
      expect(list.isDenied(p), p).toBe(true);
    }
  });

  it("does not deny ordinary files", () => {
    const list = createPathDenylist();
    for (const p of [
      "src/index.ts",
      "docs/guide.md",
      "environment.ts",
      "README.md",
      "src/secrets-manager.ts",
      "keynote.md",
    ]) {
      expect(list.isDenied(p), p).toBe(false);
    }
  });

  it("merges extra patterns without removing built-ins", () => {
    const list = createPathDenylist(["internal/**"]);
    expect(list.isDenied("internal/notes.txt")).toBe(true);
    expect(list.isDenied(".env")).toBe(true);
    expect(list.patterns.length).toBe(DEFAULT_DENY_PATHS.length + 1);
  });

  it("normalizes a leading ./ in the display path", () => {
    const list = createPathDenylist();
    expect(list.isDenied("./.env")).toBe(true);
  });
});
