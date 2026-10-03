import { readFileSync } from "node:fs";
import path from "node:path";

/**
 * package version（`--version` 出力用）。
 * builtinDir を起点に `version.json`（APM hook bundle / SEA VFS 配置）を
 * 優先し、dev checkout 用に `package.json` を fallback とする
 * （DESIGN-0014 / issue #71 の resource-root 統一）。
 */
export function packageVersion(builtinDir: string): string {
  for (const name of ["version.json", "package.json"]) {
    try {
      const pkg = JSON.parse(readFileSync(path.join(builtinDir, name), "utf8")) as {
        version?: unknown;
      };
      if (typeof pkg.version === "string") return pkg.version;
    } catch {
      // try the next candidate
    }
  }
  throw new Error("failed to resolve package version");
}
