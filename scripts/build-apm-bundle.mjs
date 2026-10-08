// Assembles the full APM hook payload (issue #71 Phase 3 /
// DESIGN-034z1EgnQwax4VamK2xmdT): the canonical bundle tree plus
// platform executables under cli/bin/<target>/.
//
//   node scripts/build-apm-bundle.mjs [--exe-dir <dir>]
//
// --exe-dir points at a directory of native executables named
// cheaper-<platform>-<arch>[.exe] (release artifacts, or a local cargo
// build output renamed to match). Executables found are copied to
// cli/bin/<platform>-<arch>/cheaper[.exe]; POSIX targets get the
// executable bit. Without --exe-dir only the bundle tree is rebuilt —
// a host-only partial payload is allowed.

import { chmodSync, copyFileSync, mkdirSync, readdirSync, statSync } from "node:fs";
import path from "node:path";

import { buildApmCliTree, repoRoot } from "./lib/apm-bundle.mjs";

const EXE_NAME = /^cheaper-(windows|darwin|linux)-(x64|arm64)(\.exe)?$/;

function parseArgs(argv) {
  const opts = {};
  for (let i = 0; i < argv.length; i += 1) {
    if (argv[i] === "--exe-dir") opts.exeDir = argv[++i];
    else {
      console.error(`unknown argument: ${argv[i]}`);
      process.exit(1);
    }
  }
  return opts;
}

const opts = parseArgs(process.argv.slice(2));
const root = repoRoot();
const { outDir } = buildApmCliTree(root);
console.log(`bundled cheaper CLI -> ${path.relative(root, outDir)}`);

if (opts.exeDir) {
  const exeDir = path.resolve(root, opts.exeDir);
  const copied = [];
  for (const entry of readdirSync(exeDir, { withFileTypes: true })) {
    const match = EXE_NAME.exec(entry.name);
    if (!entry.isFile() || !match) continue;
    const target = `${match[1] === "windows" ? "win32" : match[1]}-${match[2]}`;
    const destDir = path.join(outDir, "bin", target);
    const dest = path.join(destDir, match[3] ? "cheaper.exe" : "cheaper");
    mkdirSync(destDir, { recursive: true });
    copyFileSync(path.join(exeDir, entry.name), dest);
    if (!match[3]) chmodSync(dest, 0o755);
    copied.push(
      `${path.relative(root, dest)} (${(statSync(dest).size / 1024 / 1024).toFixed(1)} MiB)`,
    );
  }
  if (copied.length === 0) {
    console.warn(
      `warning: no executables matching cheaper-<platform>-<arch>[.exe] in ${opts.exeDir}`,
    );
  } else {
    for (const line of copied) console.log(`payload executable -> ${line}`);
  }
}
