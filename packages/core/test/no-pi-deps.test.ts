import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const pkgPath = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../package.json");

describe("@cheaper/core has no @earendil-works dependencies", () => {
  it("package.json dependencies/devDependencies contain no @earendil-works/* entries", async () => {
    const pkg = JSON.parse(await readFile(pkgPath, "utf8")) as {
      dependencies?: Record<string, string>;
      devDependencies?: Record<string, string>;
    };
    for (const section of ["dependencies", "devDependencies"] as const) {
      const deps = pkg[section] ?? {};
      for (const name of Object.keys(deps)) {
        expect(name.startsWith("@earendil-works/"), `${section}.${name}`).toBe(false);
      }
    }
  });
});
