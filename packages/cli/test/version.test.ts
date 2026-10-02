import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { packageVersion } from "../src/version.js";

const dirs: string[] = [];
function stagedDir(files: Record<string, string>): string {
  const dir = mkdtempSync(path.join(tmpdir(), "cheaper-version-"));
  dirs.push(dir);
  for (const [name, content] of Object.entries(files)) {
    writeFileSync(path.join(dir, name), content);
  }
  return dir;
}

afterEach(() => {
  for (const dir of dirs.splice(0)) {
    rmSync(dir, { recursive: true, force: true });
  }
});

describe("packageVersion", () => {
  it("resolves version.json under builtinDir (SEA / APM bundle form)", () => {
    const dir = stagedDir({ "version.json": '{"version":"9.9.9"}\n' });
    expect(packageVersion(dir)).toBe("9.9.9");
  });

  it("falls back to package.json under builtinDir (dev form)", () => {
    const dir = stagedDir({ "package.json": '{"name":"x","version":"1.2.3"}\n' });
    expect(packageVersion(dir)).toBe("1.2.3");
  });

  it("prefers version.json over package.json under builtinDir", () => {
    const dir = stagedDir({
      "version.json": '{"version":"9.9.9"}\n',
      "package.json": '{"name":"x","version":"1.2.3"}\n',
    });
    expect(packageVersion(dir)).toBe("9.9.9");
  });

  it("throws when builtinDir has neither version.json nor package.json", () => {
    const dir = stagedDir({});
    expect(() => packageVersion(dir)).toThrow("failed to resolve package version");
  });
});
