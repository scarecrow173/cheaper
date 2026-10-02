// Shared esbuild single-file ESM bundle step for the cheaper CLI.
// Used by scripts/bundle-cli.mjs (APM hook bundle) and
// scripts/build-executable.mjs (SEA standalone executable input).
//
// esbuild は root devDependency（pinned）。`pnpm exec` や shell 連結に
// 頼らず、JS launcher を node で直接起動する（cross-platform、shell 解釈なし）。

import { spawnSync } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

export function repoRoot() {
  return fileURLToPath(new URL("../..", import.meta.url));
}

/**
 * esbuild で entry を単一 ESM file に bundle し、CJS deps 用の
 * createRequire banner を前置する。失敗時は esbuild の exit status で
 * process.exit する。
 */
export function buildEsmBundle({ entry, outfile }) {
  const root = repoRoot();
  mkdirSync(path.dirname(outfile), { recursive: true });

  const esbuildJs = fileURLToPath(import.meta.resolve("esbuild/bin/esbuild"));
  const build = spawnSync(
    process.execPath,
    [
      esbuildJs,
      path.relative(root, entry),
      "--bundle",
      "--platform=node",
      "--format=esm",
      "--target=node20",
      "--minify",
      `--outfile=${path.relative(root, outfile)}`,
    ],
    { cwd: root, stdio: "inherit" },
  );
  if (build.error) {
    console.error(build.error.message);
    process.exit(1);
  }
  if (build.status !== 0) {
    process.exit(build.status ?? 1);
  }

  // CJS deps inside the ESM bundle call require(); provide one backed by
  // createRequire. Prepended after the build to keep the esbuild argv simple.
  const banner =
    "import { createRequire as __cr } from 'node:module';\n" +
    "const require = __cr(import.meta.url);\n";
  const bundleSrc = readFileSync(outfile, "utf8");
  const shebang = bundleSrc.startsWith("#!")
    ? bundleSrc.slice(0, bundleSrc.indexOf("\n") + 1)
    : "";
  writeFileSync(outfile, shebang + banner + bundleSrc.slice(shebang.length));
}
