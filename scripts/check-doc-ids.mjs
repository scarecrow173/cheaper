// Verifies that active docs do not reintroduce legacy sequential
// artifact IDs (TYPE-NNNN). Canonical IDs are opaque Base62
// (TYPE-<22 chars>) and always contain letters, so a digits-only
// suffix is always the retired scheme.
// Wired into `mise run check`, so CI fails automatically on regression.
//
// Checked: docs/**/*.md, AGENTS.md, README.md, packages/*/.apm/**/*.md
// (tracked files only — every distributable package's .apm/ markdown).
//
// NOT checked: EXP-NNNN — experiment artifacts are .jsonl files
// referenced by path and never received opaque IDs. Pre-existing
// EXP-NNNN body references are a separate, known gap.

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = fileURLToPath(new URL("..", import.meta.url));

const tracked = execFileSync("git", ["ls-files", "*.md"], {
  cwd: repoRoot,
  encoding: "utf8",
})
  .split("\n")
  .filter(
    (f) =>
      f.startsWith("docs/") ||
      f === "AGENTS.md" ||
      f === "README.md" ||
      /^packages\/[^/]+\/\.apm\//.test(f),
  );

const legacyId = /\b(?:SPEC|DESIGN|DISC|ADR|IMPL|PLAN|TASK|IDEA|REQ|BRF)-\d+\b/g;

let failed = false;
for (const rel of tracked) {
  const text = readFileSync(path.join(repoRoot, rel), "utf8");
  for (const m of text.matchAll(legacyId)) {
    const line = text.slice(0, m.index).split("\n").length;
    console.error(`legacy artifact id ${m[0]} in ${rel}:${line}`);
    failed = true;
  }
}

if (failed) {
  console.error(
    "\nArtifact IDs use the opaque Base62 scheme (TYPE-<22 chars>). " +
      "Replace legacy sequential IDs with the current id or a relative link.",
  );
  process.exit(1);
}
console.log(`doc id check OK: ${tracked.length} markdown files scanned`);
