// Bumps the product version across every declaration in one command
// (Phase 6 — canonical source is the root Cargo.toml
// `[workspace.package].version`). Usage:
//
//   node scripts/set-version.mjs <X.Y.Z>
//
// Updates:
//   Cargo.toml                    canonical source of truth
//                                 ([workspace.package] version only)
//   packages/apm/apm.yml          version: line only
//
// Does NOT touch root apm.yml (cheaper-dev dev project — independent) or
// packages/*/package.json (no longer version declarations). The binary
// embeds the version at compile time via build.rs — no generated
// metadata file needs regeneration (issue #85).

import { spawnSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { setCargoWorkspaceVersion } from "./lib/release-artifacts.mjs";

const repoRoot = fileURLToPath(new URL("..", import.meta.url));
const version = process.argv[2];
if (!version || !/^\d+\.\d+\.\d+$/.test(version)) {
  console.error("usage: node scripts/set-version.mjs <X.Y.Z>");
  process.exit(1);
}

try {
  setCargoWorkspaceVersion(repoRoot, version);
} catch (error) {
  console.error(error.message);
  process.exit(1);
}
console.log(`updated Cargo.toml -> ${version}`);

const apmYml = path.join(repoRoot, "packages", "apm", "apm.yml");
const apmText = readFileSync(apmYml, "utf8");
if (!/^version:[^\n]*$/m.test(apmText)) {
  console.error("no version: line found in packages/apm/apm.yml");
  process.exit(1);
}
const apmUpdated = apmText.replace(/^version:[^\n]*$/m, `version: "${version}"`);
writeFileSync(apmYml, apmUpdated);
console.log(`updated packages/apm/apm.yml -> ${version}`);

const check = spawnSync(process.execPath, ["scripts/check-version.mjs"], {
  cwd: repoRoot,
  stdio: "inherit",
});
if (check.status !== 0) process.exit(check.status ?? 1);
console.log("\nDone. Commit the updated version declarations together.");
