// Bumps the product version across every declaration in one command
// (ADR-0022 / DESIGN-0024). Usage:
//
//   node scripts/set-version.mjs <X.Y.Z>
//
// Updates:
//   packages/cli/package.json          canonical source of truth
//   packages/core/package.json
//   packages/tool-runtime/package.json
//   packages/workflow/package.json
//   packages/runtime-pi/package.json
//   packages/apm/apm.yml               version: line only
//
// Does NOT touch root apm.yml (cheaper-dev dev project — independent).
// Afterwards: regenerate the bundle so version.json follows, then commit.

import { readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const repoRoot = fileURLToPath(new URL("..", import.meta.url));
const version = process.argv[2];
if (!version || !/^\d+\.\d+\.\d+$/.test(version)) {
  console.error("usage: node scripts/set-version.mjs <X.Y.Z>");
  process.exit(1);
}

const jsonFiles = [
  "packages/cli/package.json",
  "packages/core/package.json",
  "packages/tool-runtime/package.json",
  "packages/workflow/package.json",
  "packages/runtime-pi/package.json",
];
for (const rel of jsonFiles) {
  const p = path.join(repoRoot, rel);
  const pkg = JSON.parse(readFileSync(p, "utf8"));
  pkg.version = version;
  writeFileSync(p, `${JSON.stringify(pkg, null, 2)}\n`);
  console.log(`updated ${rel} -> ${version}`);
}

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
