import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("../../", import.meta.url));
const NAMES = ["cheaper-linux-x64", "cheaper-linux-arm64", "cheaper-windows-x64.exe"];
const ABC = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
const SUMS = NAMES.map((name) => `${ABC}  ${name}\n`).join("");

async function library() {
  try {
    return await import("../lib/release-artifacts.mjs");
  } catch (error) {
    assert.fail(`release artifact contract is unavailable: ${error.message}`);
  }
}

function fixture(t) {
  mkdirSync(path.join(ROOT, "tmp"), { recursive: true });
  const root = mkdtempSync(path.join(ROOT, "tmp", "release-test-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const pkg = path.join(root, "packages", "apm");
  const cli = path.join(pkg, ".apm", "hooks", "cli");
  writeFileSync(path.join(root, "Cargo.toml"), '[workspace.package]\nversion = "1.2.3"\n\n[profile.release]\n');
  mkdirSync(path.join(cli, "bin", "linux-x64"), { recursive: true });
  writeFileSync(path.join(pkg, "apm.yml"), 'name: cheaper\nversion: "1.2.3"\n');
  writeFileSync(path.join(cli, "version.json"), '{"version":"1.2.3"}\n');
  writeFileSync(path.join(cli, "bin", "linux-x64", "cheaper"), "old committed payload");
  const assets = path.join(root, "assets");
  mkdirSync(assets);
  for (const name of NAMES) writeFileSync(path.join(assets, name), "abc");
  writeFileSync(path.join(assets, "SHA256SUMS"), SUMS);
  return { root, pkg, cli, assets, output: path.join(root, "output") };
}

test("manifest uses ASCII and LF", async (t) => {
  const f = fixture(t);
  const { cargoWorkspaceVersion } = await library();
  const version = cargoWorkspaceVersion(ROOT);
  const result = spawnSync(process.execPath, [
    path.join(ROOT, "scripts/generate-release-checksums.mjs"),
    "--asset-dir", f.assets, "--tag", `v${version}`,
  ], { encoding: "utf8" });
  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(readFileSync(path.join(f.assets, "SHA256SUMS")), Buffer.from(SUMS, "ascii"));
});

test("manifest rejects CR and non-ASCII", async () => {
  const { parseManifest } = await library();
  for (const bad of [SUMS.replaceAll("\n", "\r\n"), `\uFEFF${SUMS}`, SUMS.replace("cheaper", "chëaper")]) {
    assert.throws(() => parseManifest(Buffer.from(bad)), /ASCII|CR|format/);
  }
});

test("manifest rejects blank lines", async () => {
  const { parseManifest } = await library();
  for (const bad of [`\n${SUMS}`, `${SUMS}\n`, SUMS.replace("\n", "\n\n")]) {
    assert.throws(() => parseManifest(Buffer.from(bad)), /blank|format/);
  }
});

test("manifest enforces hash delimiter and basename", async () => {
  const { parseManifest } = await library();
  for (const bad of [
    SUMS.replace(ABC, ABC.slice(1)), SUMS.replace(ABC, `z${ABC.slice(1)}`),
    SUMS.replace("  ", " "), SUMS.replace("  ", "   "), SUMS.replace("  ", "\t"),
    SUMS.replace("cheaper-linux-x64", "../cheaper-linux-x64"),
    SUMS.replace("cheaper-linux-x64", "dir\\cheaper-linux-x64"),
  ]) assert.throws(() => parseManifest(Buffer.from(bad)), /format/);
});

test("manifest enforces 64KiB limit", async () => {
  const { parseManifest } = await library();
  const prefix = `${SUMS}${ABC}  `;
  const exactly = `${prefix}${"x".repeat(65536 - prefix.length - 1)}\n`;
  assert.equal(Buffer.byteLength(exactly), 65536);
  assert.equal(parseManifest(Buffer.from(exactly)).size, 4);
  assert.throws(() => parseManifest(Buffer.from(exactly.replace(/\n$/, "x\n"))), /64KiB/);
});

test("manifest requires each target exactly once", async () => {
  const { parseManifest } = await library();
  for (const name of NAMES) {
    assert.throws(() => parseManifest(Buffer.from(SUMS.replace(`${ABC}  ${name}\n`, ""))), /missing/);
    assert.throws(() => parseManifest(Buffer.from(`${SUMS}${ABC}  ${name}\n`)), /duplicate/);
  }
});

test("manifest rejects duplicate basenames", async () => {
  const { parseManifest } = await library();
  assert.throws(() => parseManifest(Buffer.from(`${SUMS}${ABC}  extra\n${ABC}  extra\n`)), /duplicate/);
});

test("release targets stay the three published non-darwin assets", async () => {
  const { releaseTargets } = await library();
  assert.deepEqual(releaseTargets.map(({ asset }) => asset), NAMES);
  assert.ok(!releaseTargets.some(({ target }) => target.startsWith("darwin")), "darwin stays a sibling-only target outside releaseTargets");
});

test("asset names match launcher resolution", async (t) => {
  const f = fixture(t);
  const { stagePackage } = await library();
  const staged = stagePackage({ root: f.root, assetDir: f.assets, stageDir: path.join(f.root, "stage"), tag: "v1.2.3" });
  for (const rel of ["linux-x64/cheaper", "linux-arm64/cheaper", "win32-x64/cheaper.exe"]) {
    assert.equal(readFileSync(path.join(staged, ".apm/hooks/cli/bin", rel), "utf8"), "abc");
  }
});

test("release tag and declarations must agree", async (t) => {
  const f = fixture(t);
  const { verifyReleaseVersion } = await library();
  assert.equal(verifyReleaseVersion(f.root, "v1.2.3"), "1.2.3");
  for (const tag of ["v1.2.4", "1.2.3", "v01.2.3", "v1.2.3;echo bad"]) {
    assert.throws(() => verifyReleaseVersion(f.root, tag), /version|tag/);
  }
  writeFileSync(path.join(f.cli, "version.json"), '{"version":"1.2.4"}');
  assert.throws(() => verifyReleaseVersion(f.root, "v1.2.3"), /version/);
  writeFileSync(path.join(f.cli, "version.json"), '{"version":"1.2.3"}');
  writeFileSync(path.join(f.pkg, "apm.yml"), 'name: cheaper\nversion: "1.2.4"\n');
  assert.throws(() => verifyReleaseVersion(f.root, "v1.2.3"), /version/);
  writeFileSync(path.join(f.pkg, "apm.yml"), 'name: cheaper\nversion: "1.2.3"\n');
  // Cargo.toml is canonical — rewriting it moves the canonical version,
  // so the tag mismatch is reported against the new canonical.
  writeFileSync(path.join(f.root, "Cargo.toml"), '[workspace.package]\nversion = "1.2.4"\n');
  assert.throws(() => verifyReleaseVersion(f.root, "v1.2.3"), /tag .*canonical version/);
});

test("workspace version is read only from [workspace.package]", async (t) => {
  const f = fixture(t);
  const { cargoWorkspaceVersion, verifyReleaseVersion } = await library();
  assert.equal(cargoWorkspaceVersion(f.root), "1.2.3");
  writeFileSync(path.join(f.root, "Cargo.toml"), '[workspace.dependencies]\nserde = { version = "1" }\n\n[workspace.package]\n# comment\nversion = "1.2.3" # trailing comment\nedition = "2021"\n');
  assert.equal(verifyReleaseVersion(f.root, "v1.2.3"), "1.2.3");
  writeFileSync(path.join(f.root, "Cargo.toml"), '[workspace.package]\nedition = "2021"\n');
  assert.throws(() => cargoWorkspaceVersion(f.root), /Cargo\.toml/);
  rmSync(path.join(f.root, "Cargo.toml"));
  assert.throws(() => verifyReleaseVersion(f.root, "v1.2.3"), /ENOENT|Cargo\.toml/);
});

test("setCargoWorkspaceVersion rewrites only the section declaration", async (t) => {
  const f = fixture(t);
  const { setCargoWorkspaceVersion } = await library();
  const file = path.join(f.root, "Cargo.toml");
  writeFileSync(file, '[workspace]\nmembers = ["crates/*"]\n\n[workspace.package]\nedition = "2021"\nversion = "1.2.3"\n\n[workspace.dependencies]\nserde = { version = "1" }\n\n[profile.release]\nopt-level = "s"\n');
  setCargoWorkspaceVersion(f.root, "2.0.0");
  const text = readFileSync(file, "utf8");
  assert.match(text, /\[workspace\.package\]\nedition = "2021"\nversion = "2\.0\.0"\n/);
  assert.match(text, /serde = \{ version = "1" \}/);
  writeFileSync(file, '[workspace.package]\nedition = "2021"\n');
  assert.throws(() => setCargoWorkspaceVersion(f.root, "3.0.0"), /Cargo\.toml/);
});

test("setCargoWorkspaceVersion splices at the match offset without pattern expansion", async (t) => {
  const f = fixture(t);
  const { setCargoWorkspaceVersion, cargoWorkspaceVersion } = await library();
  const file = path.join(f.root, "Cargo.toml");
  // section body と同一の文字列が前方に出現しても [workspace.package] 側だけを
  // 書き換え、body 内の `$&` 等を replacement pattern として展開しないこと。
  const body = 'edition = "2021"\nversion = "1.2.3"\n# literal $& tail\n';
  writeFileSync(file, `[other.section]\n${body}\n[workspace.package]\n${body}\n[profile.release]\n`);
  setCargoWorkspaceVersion(f.root, "2.0.0");
  const text = readFileSync(file, "utf8");
  assert.equal(cargoWorkspaceVersion(f.root), "2.0.0");
  const [earlier, workspace] = text.split("[workspace.package]");
  assert.match(earlier, /version = "1\.2\.3"/);
  assert.match(workspace, /version = "2\.0\.0"/);
  assert.equal((text.match(/literal \$& tail/g) ?? []).length, 2);
});

test("snapshot preserves raw bytes and manifest", async (t) => {
  const f = fixture(t);
  const { assembleBundle } = await library();
  const result = assembleBundle({ root: f.root, assetDir: f.assets, outDir: f.output, tag: "v1.2.3" });
  assert.equal(path.basename(result.archive), "cheaper-apm-v1.2.3.tar.gz");
  const extracted = path.join(f.root, "extracted");
  mkdirSync(extracted);
  const tar = spawnSync("tar", ["-xzf", result.archive, "-C", extracted], { encoding: "utf8" });
  assert.equal(tar.status, 0, tar.stderr);
  const cli = path.join(extracted, "cheaper/.apm/hooks/cli");
  assert.equal(readFileSync(path.join(cli, "SHA256SUMS"), "utf8"), SUMS);
  assert.equal(readFileSync(path.join(f.output, "SHA256SUMS"), "utf8"), SUMS);
  for (const rel of ["linux-x64/cheaper", "linux-arm64/cheaper", "win32-x64/cheaper.exe"]) {
    const digest = createHash("sha256").update(readFileSync(path.join(cli, "bin", rel))).digest("hex");
    assert.equal(digest, ABC);
  }
});

test("snapshot leaves source payload unchanged", async (t) => {
  const f = fixture(t);
  const { stagePackage } = await library();
  stagePackage({ root: f.root, assetDir: f.assets, stageDir: path.join(f.root, "stage"), tag: "v1.2.3" });
  assert.equal(readFileSync(path.join(f.cli, "bin/linux-x64/cheaper"), "utf8"), "old committed payload");
  assert.throws(() => stagePackage({ root: f.root, assetDir: f.assets, stageDir: f.pkg, tag: "v1.2.3" }), /source|output|stage/);
  assert.equal(readFileSync(path.join(f.cli, "version.json"), "utf8"), '{"version":"1.2.3"}\n');
});

// Archive producer は Ubuntu。Windows の filesystem/tar は POSIX mode を保存しない。
test("snapshot gives POSIX binaries mode 0755", { skip: process.platform !== "linux" ? "Linux archive producer only; CI pre-gate verifies POSIX mode" : false }, async (t) => {
  const f = fixture(t);
  const { assembleBundle } = await library();
  const result = assembleBundle({ root: f.root, assetDir: f.assets, outDir: f.output, tag: "v1.2.3" });
  const list = spawnSync("tar", ["-tvzf", result.archive], { encoding: "utf8" });
  assert.equal(list.status, 0, list.stderr);
  for (const target of ["linux-x64", "linux-arm64"]) {
    const entry = list.stdout.split("\n").map((line) => line.trimEnd()).find((line) => line.endsWith(`cli/bin/${target}/cheaper`));
    assert.ok(entry, list.stdout);
    assert.match(entry, /^-rwxr-xr-x/);
  }
});

test("dry-run validates without creating output", async (t) => {
  const f = fixture(t);
  const { assembleBundle } = await library();
  const result = assembleBundle({ root: f.root, assetDir: f.assets, outDir: f.output, tag: "v1.2.3", dryRun: true });
  assert.equal(path.basename(result.archive), "cheaper-apm-v1.2.3.tar.gz");
  assert.equal(existsSync(f.output), false);
  writeFileSync(path.join(f.assets, NAMES[0]), "modified");
  assert.throws(() => assembleBundle({ root: f.root, assetDir: f.assets, outDir: f.output, tag: "v1.2.3", dryRun: true }), /hash|checksum/);
  assert.equal(existsSync(f.output), false);
});

test("snapshot refuses existing archives", async (t) => {
  const f = fixture(t);
  const { assembleBundle } = await library();
  mkdirSync(f.output);
  const archive = path.join(f.output, "cheaper-apm-v1.2.3.tar.gz");
  writeFileSync(archive, "existing release archive");
  assert.throws(() => assembleBundle({ root: f.root, assetDir: f.assets, outDir: f.output, tag: "v1.2.3" }), /existing|exists/);
  assert.equal(readFileSync(archive, "utf8"), "existing release archive");
});

test("artifact verification rejects pointer and wrong hash", async (t) => {
  const f = fixture(t);
  const { verifyAssets } = await library();
  writeFileSync(path.join(f.assets, NAMES[0]), "version https://git-lfs.github.com/spec/v1\n");
  assert.throws(() => verifyAssets(f.assets), /LFS|pointer/);
  writeFileSync(path.join(f.assets, NAMES[0]), "changed");
  assert.throws(() => verifyAssets(f.assets), /hash|checksum/);
  rmSync(path.join(f.assets, NAMES[0]));
  mkdirSync(path.join(f.assets, NAMES[0]));
  assert.throws(() => verifyAssets(f.assets), /regular|file/);
});

test("verify mode rejects corrupt artifacts without rewriting manifest", async (t) => {
  const f = fixture(t);
  const { cargoWorkspaceVersion } = await library();
  const version = cargoWorkspaceVersion(ROOT);
  writeFileSync(path.join(f.assets, NAMES[0]), "changed");
  const result = spawnSync(process.execPath, [path.join(ROOT, "scripts/generate-release-checksums.mjs"), "--asset-dir", f.assets, "--tag", `v${version}`, "--verify"], { encoding: "utf8" });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /checksum|hash/);
  assert.equal(readFileSync(path.join(f.assets, "SHA256SUMS"), "utf8"), SUMS);
  assert.equal(statSync(path.join(f.assets, NAMES[0])).isFile(), true);
});
