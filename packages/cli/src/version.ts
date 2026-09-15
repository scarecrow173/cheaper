import { readFileSync } from "node:fs";

/** package.json の version（`--version` 出力用）。 */
export function packageVersion(): string {
  const pkg = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8")) as {
    version: string;
  };
  return pkg.version;
}
