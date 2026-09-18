// Generates packages/apm/.apm/hooks/cli/ — the self-contained cheaper CLI
// shipped inside the APM hook bundle (DESIGN-0014). Run via `pnpm bundle`.
//
// Layout produced:
//   packages/apm/.apm/hooks/cli/bin/cheaper.mjs   esbuild ESM bundle (minified)
//   packages/apm/.apm/hooks/cli/package.json      {name, version} from packages/cli
//   packages/apm/.apm/hooks/cli/workers/          copied worker definitions + schemas
//   packages/apm/.apm/hooks/cli/policy.yaml       copied built-in policy
//
// The generated tree is committed so git/local-path consumers get a working
// CLI without a build step.

import { spawnSync } from "node:child_process";
import { cpSync, mkdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = fileURLToPath(new URL("..", import.meta.url));
const cliPkg = path.join(repoRoot, "packages", "cli");
const outDir = path.join(repoRoot, "packages", "apm", ".apm", "hooks", "cli");
const outBin = path.join(outDir, "bin", "cheaper.mjs");

rmSync(outDir, { recursive: true, force: true });
mkdirSync(path.dirname(outBin), { recursive: true });

// esbuild は root devDependency（pinned）。`pnpm exec` や shell 連結に
// 頼らず、JS launcher を node で直接起動する（cross-platform、shell 解釈なし）。
const esbuildJs = fileURLToPath(import.meta.resolve("esbuild/bin/esbuild"));
const build = spawnSync(
  process.execPath,
  [
    esbuildJs,
    "packages/cli/bin/cheaper.js",
    "--bundle",
    "--platform=node",
    "--format=esm",
    "--target=node20",
    "--minify",
    `--outfile=${path.relative(repoRoot, outBin)}`,
  ],
  { cwd: repoRoot, stdio: "inherit" },
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
const bundleSrc = readFileSync(outBin, "utf8");
const shebang = bundleSrc.startsWith("#!")
  ? bundleSrc.slice(0, bundleSrc.indexOf("\n") + 1)
  : "";
writeFileSync(outBin, shebang + banner + bundleSrc.slice(shebang.length));

// `package.json` は APM が hook bundle descriptor として扱い deploy から
// 外されるため、CLI は sibling の version.json を fallback で読む
// （packages/cli/src/version.ts）。
const pkg = JSON.parse(readFileSync(path.join(cliPkg, "package.json"), "utf8"));
writeFileSync(
  path.join(outDir, "version.json"),
  `${JSON.stringify({ version: pkg.version }, null, 2)}\n`,
);
cpSync(path.join(cliPkg, "workers"), path.join(outDir, "workers"), { recursive: true });
cpSync(path.join(cliPkg, "policy.yaml"), path.join(outDir, "policy.yaml"));

const size = (statSync(outBin).size / 1024 / 1024).toFixed(1);
console.log(`bundled cheaper CLI -> ${path.relative(repoRoot, outDir)} (${size} MiB)`);
