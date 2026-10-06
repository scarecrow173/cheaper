import { readdir, readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const pkgDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const pkgPath = path.join(pkgDir, "package.json");
const srcDir = path.join(pkgDir, "src");

const forbiddenPrefixes = ["@earendil-works/", "@cheaper/runtime-pi", "@cheaper/workflow"];

// `import ... from "X"`, `export ... from "X"`, side-effect `import "X"`,
// and literal dynamic `import("X")` specifiers.
const specifierPattern = /(?:from|import)\s*(?:\(\s*)?["']([^"']+)["']/g;

describe("@cheaper/tool-runtime dependency guard", () => {
  it("package.json dependencies/devDependencies contain no forbidden entries", async () => {
    const pkg = JSON.parse(await readFile(pkgPath, "utf8")) as {
      dependencies?: Record<string, string>;
      devDependencies?: Record<string, string>;
    };
    for (const section of ["dependencies", "devDependencies"] as const) {
      const deps = pkg[section] ?? {};
      for (const name of Object.keys(deps)) {
        for (const forbidden of forbiddenPrefixes) {
          expect(name.startsWith(forbidden), `${section}.${name}`).toBe(false);
        }
      }
    }
  });

  it("src/**/*.ts import/export/dynamic-import specifiers use no forbidden prefixes", async () => {
    const entries = await readdir(srcDir, { recursive: true });
    const tsFiles = entries.filter((entry) => entry.endsWith(".ts"));
    for (const rel of tsFiles) {
      const filePath = path.join(srcDir, rel);
      const text = await readFile(filePath, "utf8");
      for (const match of text.matchAll(specifierPattern)) {
        const specifier = match[1] as string;
        for (const forbidden of forbiddenPrefixes) {
          expect(specifier.startsWith(forbidden), `${filePath}: ${specifier}`).toBe(false);
        }
      }
    }
  });
});
