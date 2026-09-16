import { readFileSync } from "node:fs";

/**
 * package.json の version（`--version` 出力用）。
 * APM hook bundle 経由で deploy される single-file bundle では
 * `package.json` が bundle descriptor と衝突して除去されるため、
 * sibling の `version.json`（`{"version": ...}`）を fallback とする
 * （DESIGN-0014）。
 */
export function packageVersion(): string {
  for (const rel of ["../package.json", "../version.json"]) {
    try {
      const pkg = JSON.parse(readFileSync(new URL(rel, import.meta.url), "utf8")) as {
        version: string;
      };
      return pkg.version;
    } catch {
      // try the next candidate
    }
  }
  throw new Error("failed to resolve package version");
}
