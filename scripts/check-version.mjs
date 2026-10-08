// Verifies that every product version declaration matches the canonical
// source of truth: packages/cli/package.json (ADR-0022 / DESIGN-0024).
// Wired into `mise run check`, so CI fails automatically on divergence.
//
// Checked:
//   packages/cli/package.json                canonical version
//   packages/core/package.json               must equal canonical
//   packages/tool-runtime/package.json       must equal canonical
//   packages/workflow/package.json           must equal canonical
//   packages/runtime-pi/package.json         must equal canonical
//   packages/apm/apm.yml                     must equal canonical (version: line)
//   packages/apm/.apm/hooks/cli/version.json must equal canonical (generated)
//   Cargo.toml                               must equal canonical
//                                            ([workspace.package] version)
//
// NOT checked: root apm.yml — it describes the cheaper-dev development
// project, whose version is independent of the product.

import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { cargoWorkspaceVersion } from "./lib/release-artifacts.mjs";

const repoRoot = fileURLToPath(new URL("..", import.meta.url));

const readJsonVersion = (rel) =>
  JSON.parse(readFileSync(path.join(repoRoot, rel), "utf8")).version;

const readApmVersion = (rel) => {
  const text = readFileSync(path.join(repoRoot, rel), "utf8");
  const m = text.match(/^version:\s*"?([^"\s]+)"?\s*$/m);
  if (!m) throw new Error(`no version: line found in ${rel}`);
  return m[1];
};

const canonical = readJsonVersion("packages/cli/package.json");

const declarations = [
  ["packages/core/package.json", () => readJsonVersion("packages/core/package.json")],
  [
    "packages/tool-runtime/package.json",
    () => readJsonVersion("packages/tool-runtime/package.json"),
  ],
  [
    "packages/workflow/package.json",
    () => readJsonVersion("packages/workflow/package.json"),
  ],
  [
    "packages/runtime-pi/package.json",
    () => readJsonVersion("packages/runtime-pi/package.json"),
  ],
  ["packages/apm/apm.yml", () => readApmVersion("packages/apm/apm.yml")],
  [
    "packages/apm/.apm/hooks/cli/version.json",
    () => readJsonVersion("packages/apm/.apm/hooks/cli/version.json"),
  ],
  ["Cargo.toml", () => cargoWorkspaceVersion(repoRoot)],
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
    `\nversion source of truth is packages/cli/package.json (${canonical}). ` +
      `Run node scripts/set-version.mjs <version> to sync declarations.`,
  );
  process.exit(1);
}
console.log(`version check OK: ${canonical}`);
