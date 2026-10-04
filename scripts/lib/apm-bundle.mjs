// Shared assembly of packages/apm/.apm/hooks/cli/ — the self-contained
// cheaper CLI tree shipped inside the APM hook bundle (issue #71).
// Used by scripts/bundle-cli.mjs (canonical `pnpm bundle` entrypoint) and
// scripts/build-apm-bundle.mjs (adds platform executables).
//
// Layout produced:
//   cli/bin/cheaper.mjs       esbuild ESM bundle (minified)
//   cli/bin/<target>/cheaper[.exe]  SEA executables (build-apm-bundle only)
//   cli/version.json          {version} derived from packages/cli
//   cli/workers/              worker definitions + schemas
//   cli/policy.yaml           built-in policy
//
// Small generated assets are committed. Git/local-path hook consumers acquire
// a pinned executable into user cache; release archives/local builds carry
// ignored platform executables instead of adding them to Git.

import { cpSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import path from "node:path";

import { buildEsmBundle, repoRoot } from "./esbuild-bundle.mjs";

export { repoRoot };

export function apmHooksDir(root = repoRoot()) {
  return path.join(root, "packages", "apm", ".apm", "hooks");
}

// Rebuilds cli/ generated assets. bin/cheaper.mjs・version.json・
// workers/・policy.yaml のみを削除対象にし、bin/<target>/ 配下の
// platform executable（local build/release 用、Git ignore 対象）は保持する。
export function buildApmCliTree(root = repoRoot()) {
  const cliPkg = path.join(root, "packages", "cli");
  const outDir = path.join(apmHooksDir(root), "cli");
  const outBin = path.join(outDir, "bin", "cheaper.mjs");

  for (const rel of [
    path.join("bin", "cheaper.mjs"),
    "version.json",
    "workers",
    "policy.yaml",
  ]) {
    rmSync(path.join(outDir, rel), { recursive: true, force: true });
  }

  buildEsmBundle({
    entry: path.join(cliPkg, "bin", "cheaper.js"),
    outfile: outBin,
  });

  // `package.json` は APM が hook bundle descriptor として扱い deploy
  // から外されるため、CLI は sibling の version.json を fallback で
  // 読む（packages/cli/src/version.ts）。
  const pkg = JSON.parse(readFileSync(path.join(cliPkg, "package.json"), "utf8"));
  writeFileSync(
    path.join(outDir, "version.json"),
    `${JSON.stringify({ version: pkg.version }, null, 2)}\n`,
  );
  cpSync(path.join(cliPkg, "workers"), path.join(outDir, "workers"), {
    recursive: true,
  });
  cpSync(path.join(cliPkg, "policy.yaml"), path.join(outDir, "policy.yaml"));

  return { outDir, sizeMiB: (statSync(outBin).size / 1024 / 1024).toFixed(1) };
}
