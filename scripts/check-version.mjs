// Verifies that every product version declaration matches the canonical
// source of truth: the root Cargo.toml `[workspace.package].version`
// (Phase 6 — crates/cheaper-cli inherits it via `version.workspace = true`
// and build.rs embeds it into the binary). Wired into `mise run check`,
// so CI fails automatically on divergence.
//
// Checked:
//   Cargo.toml                               canonical version
//                                            ([workspace.package] version)
//   packages/apm/apm.yml                     must equal canonical (version: line)
//   packages/apm/.apm/hooks/cli/version.json must equal canonical (generated)
//
// NOT checked: root apm.yml — it describes the cheaper-dev development
// project, whose version is independent of the product. `packages/cli` is
// no longer a version declaration site (removed in Phase 6 Commit C).

import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { apmPackageVersion, cargoWorkspaceVersion } from "./lib/release-artifacts.mjs";

const repoRoot = fileURLToPath(new URL("..", import.meta.url));

const readJsonVersion = (rel) =>
  JSON.parse(readFileSync(path.join(repoRoot, rel), "utf8")).version;

const canonical = cargoWorkspaceVersion(repoRoot);

const declarations = [
  [
    "packages/apm/apm.yml",
    () => apmPackageVersion(path.join(repoRoot, "packages/apm/apm.yml")),
  ],
  [
    "packages/apm/.apm/hooks/cli/version.json",
    () => readJsonVersion("packages/apm/.apm/hooks/cli/version.json"),
  ],
];

let failed = false;
for (const [file, read] of declarations) {
  let value;
  try {
    value = read();
  } catch (e) {
    console.error(`version mismatch: ${file} — unreadable (${e.message})`);
    failed = true;
    continue;
  }
  if (value !== canonical) {
    console.error(`version mismatch: ${file} declares ${value}, canonical is ${canonical}`);
    failed = true;
  }
}

if (failed) {
  console.error(
    `\nversion source of truth is Cargo.toml [workspace.package].version (${canonical}). ` +
      `Run node scripts/set-version.mjs <version> to sync declarations.`,
  );
  process.exit(1);
}
console.log(`version check OK: ${canonical}`);
