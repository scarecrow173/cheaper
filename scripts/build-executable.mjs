// Builds a Node.js SEA standalone executable for the cheaper CLI
// (issue #71 / DESIGN-034z1EfqYaEBfyNvqZyFSD).
//
//   node scripts/build-executable.mjs [--target <platform>-<arch>] [--out-dir <dir>]
//
// Pipeline: dist/sea-main.js → esbuild single ESM bundle (shared helper)
// → sea-config.json (mainFormat: module, useVfs, assets = workers/ +
// policy.yaml + version.json) → `node --build-sea` →
// dist/sea/cheaper-<platform>-<arch>[.exe].
//
// `--build-sea` (Node ≥25.5) injects the blob and writes the final
// executable directly; postject is not required. Cross-target builds need
// the target platform's node binary via the `executable` field — Phase 2
// concern; for now --target must equal the host.

import { spawnSync } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import path from "node:path";

import { buildEsmBundle, repoRoot } from "./lib/esbuild-bundle.mjs";

const PLATFORM_LABEL = { win32: "windows", darwin: "darwin", linux: "linux" };

function parseArgs(argv) {
  const opts = {};
  for (let i = 0; i < argv.length; i += 1) {
    if (argv[i] === "--target") opts.target = argv[++i];
    else if (argv[i] === "--out-dir") opts.outDir = argv[++i];
    else {
      console.error(`unknown argument: ${argv[i]}`);
      process.exit(1);
    }
  }
  return opts;
}

function fail(message) {
  console.error(message);
  process.exit(1);
}

function collectAssets(dir, prefix, out) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const abs = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      collectAssets(abs, `${prefix}${entry.name}/`, out);
    } else {
      out[`${prefix}${entry.name}`] = abs;
    }
  }
}

const opts = parseArgs(process.argv.slice(2));
const host = `${process.platform}-${process.arch}`;
const target = opts.target ?? host;
if (!PLATFORM_LABEL[target.split("-")[0]]) {
  fail(`unknown platform in target: ${target}`);
}
if (target !== host) {
  fail(
    `cross-target SEA builds need the target node binary (sea-config "executable" ` +
      `field); --target ${target} on host ${host} is not supported yet`,
  );
}

const root = repoRoot();
const cliPkg = path.join(root, "packages", "cli");
const seaEntry = path.join(cliPkg, "dist", "sea-main.js");
if (!existsSync(seaEntry)) {
  fail("packages/cli/dist/sea-main.js not found — run `pnpm build` first");
}

const outDir = path.resolve(root, opts.outDir ?? path.join("dist", "sea"));
const staging = path.join(outDir, "_staging");
rmSync(outDir, { recursive: true, force: true });
mkdirSync(staging, { recursive: true });

// Canonical esbuild bundle of the SEA entrypoint.
const bundlePath = path.join(staging, "cheaper-sea.mjs");
buildEsmBundle({ entry: seaEntry, outfile: bundlePath });

// version.json を staging に生成して VFS asset として埋め込む
// （packages/cli/package.json が version の canonical source）。
const pkg = JSON.parse(readFileSync(path.join(cliPkg, "package.json"), "utf8"));
const stagedVersion = path.join(staging, "version.json");
writeFileSync(stagedVersion, `${JSON.stringify({ version: pkg.version }, null, 2)}\n`);

const assets = {};
collectAssets(path.join(cliPkg, "workers"), "workers/", assets);
assets["policy.yaml"] = path.join(cliPkg, "policy.yaml");
assets["version.json"] = stagedVersion;

const [platform, arch] = target.split("-");
const exeName = `cheaper-${PLATFORM_LABEL[platform]}-${arch}${platform === "win32" ? ".exe" : ""}`;
const output = path.join(outDir, exeName);

const seaConfig = {
  main: bundlePath,
  output,
  mainFormat: "module",
  disableExperimentalSEAWarning: true,
  useVfs: true,
  useSnapshot: false,
  useCodeCache: false,
  assets,
};
const configPath = path.join(staging, "sea-config.json");
writeFileSync(configPath, `${JSON.stringify(seaConfig, null, 2)}\n`);

const build = spawnSync(process.execPath, ["--build-sea", configPath], { stdio: "inherit" });
if (build.error) {
  console.error(build.error.message);
  process.exit(1);
}
if (build.status !== 0) {
  process.exit(build.status ?? 1);
}

const size = (statSync(output).size / 1024 / 1024).toFixed(1);
console.log(`SEA executable -> ${path.relative(root, output)} (${size} MiB)`);
