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

import { cpSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import path from "node:path";

import { buildEsmBundle, repoRoot } from "./lib/esbuild-bundle.mjs";

const root = repoRoot();
const cliPkg = path.join(root, "packages", "cli");
const outDir = path.join(root, "packages", "apm", ".apm", "hooks", "cli");
const outBin = path.join(outDir, "bin", "cheaper.mjs");

rmSync(outDir, { recursive: true, force: true });

buildEsmBundle({
  entry: path.join(cliPkg, "bin", "cheaper.js"),
  outfile: outBin,
});

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
console.log(`bundled cheaper CLI -> ${path.relative(root, outDir)} (${size} MiB)`);
