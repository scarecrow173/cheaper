// Verifies the committed APM hook payload carries the expected set of
// platform executables (issue #71): every supported target dir must
// contain a real binary — not a Git LFS pointer — and the executable
// matching the host platform must report the canonical package version.
// Wired into `mise run check` so a missing/pointer/stale payload fails
// locally as well as in CI.
//
// The byte-for-byte "same artifact as the release pipeline" guarantee
// lives in .github/workflows/release.yml (freshness compare step); this
// script is the host-side check that works without a release run.
//
// Keep EXPECTED_TARGETS in sync with the release.yml build matrix.

import { execFileSync } from "node:child_process";
import { closeSync, existsSync, openSync, readFileSync, readSync, statSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = fileURLToPath(new URL("..", import.meta.url));
const payloadBin = path.join(repoRoot, "packages/apm/.apm/hooks/cli/bin");

const EXPECTED_TARGETS = ["win32-x64", "linux-x64", "linux-arm64"];

const LFS_MIN_SIZE = 1 * 1024 * 1024;

function isLfsPointer(file) {
  const buf = Buffer.alloc(64);
  const fd = openSync(file, "r");
  try {
    readSync(fd, buf, 0, 64, 0);
  } finally {
    closeSync(fd);
  }
  return buf.toString("utf8", 0, 44).startsWith("version https://git-lfs");
}

const hostTarget = `${process.platform}-${process.arch}`;
const canonicalVersion = JSON.parse(
  readFileSync(path.join(repoRoot, "packages/cli/package.json"), "utf8"),
).version;

let failed = false;
for (const target of EXPECTED_TARGETS) {
  const name = target === "win32-x64" ? "cheaper.exe" : "cheaper";
  const exe = path.join(payloadBin, target, name);
  const rel = path.relative(repoRoot, exe);
  if (!existsSync(exe)) {
    console.error(`payload missing: ${rel} (expected target ${target})`);
    failed = true;
    continue;
  }
  const size = statSync(exe).size;
  if (size < LFS_MIN_SIZE || isLfsPointer(exe)) {
    console.error(
      `payload is not a real binary: ${rel} (${size} bytes — ` +
        `looks like a Git LFS pointer; run 'git lfs pull')`,
    );
    failed = true;
    continue;
  }
  if (target !== hostTarget) continue;
  let reported;
  try {
    reported = execFileSync(exe, ["--version"], { encoding: "utf8" }).trim();
  } catch (e) {
    console.error(`payload executable failed to run: ${rel} (${e.message})`);
    failed = true;
    continue;
  }
  if (reported !== canonicalVersion) {
    console.error(
      `payload version mismatch: ${rel} reports ${reported}, canonical is ${canonicalVersion}`,
    );
    failed = true;
  }
}

if (failed) {
  console.error(
    "\nAPM payload check failed. Regenerate platform executables via the " +
      "release workflow and commit them through Git LFS.",
  );
  process.exit(1);
}
console.log(`APM payload check OK: ${EXPECTED_TARGETS.join(", ")}`);
