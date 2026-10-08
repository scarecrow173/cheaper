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
// packages/*/package.json (no longer version declarations). version.json
// is generated — regenerate the bundle so it follows, then commit.

import { readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
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
if (check.status !== 0) {
  console.error(
    "\nversion.json is generated — run node scripts/bundle-cli.mjs to regenerate it, " +
      "then commit all version changes together.",
  );
  process.exit(check.status ?? 1);
}
console.log(
  `\nDone. Now run node scripts/bundle-cli.mjs to regenerate version.json, ` +
    `then commit the declarations and generated bundle together.`,
);
