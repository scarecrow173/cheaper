import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";

const read = (name) => readFileSync(new URL(`../../.github/workflows/${name}`, import.meta.url), "utf8");
const workflow = () => read("release-acquisition.yml");
const job = (text, name) => {
  const block = text.match(new RegExp(`^  ${name}:\\n([\\s\\S]*?)(?=^  [a-z][a-z-]*:|$(?![\\s\\S]))`, "m"));
  assert.ok(block, `missing job ${name}`);
  return block[0];
};

test("workflow waits for native bundle smokes including the darwin sibling leg", () => {
  const text = workflow();
  assert.match(job(text, "smoke-bundle"), /needs: \[resolve-release, assemble-release\]/);
  for (const runner of ["windows-latest", "ubuntu-latest", "ubuntu-24.04-arm", "macos-latest"]) assert.ok(job(text, "smoke-bundle").includes(runner));
  assert.match(job(text, "publish-release"), /needs: \[resolve-release, build-executables, assemble-release, smoke-bundle\]/);
  assert.match(job(text, "smoke-bundle"), /node scripts\/smoke-release.mjs/);
  assert.match(read("build-executables.yml"), /Smoke test executable[\s\S]*--version[\s\S]*models/);
});

test("acquisition jobs consume bin-* artifacts from dist/bin", () => {
  const text = workflow();
  for (const name of ["assemble-release", "publish-release"]) {
    const block = job(text, name);
    assert.match(block, /pattern: bin-\*/);
    assert.match(block, /path: dist\/bin/);
    assert.match(block, /--asset-dir dist\/bin/);
  }
  assert.doesNotMatch(text, /sea-\*|dist\/sea/);
});

test("macos smoke stages the darwin sibling without acquiring it", () => {
  const smoke = job(workflow(), "smoke-bundle");
  assert.match(smoke, /os: \[windows-latest, ubuntu-latest, ubuntu-24\.04-arm, macos-latest\]/);
  assert.match(smoke, /name: bin-darwin-arm64/);
  assert.match(smoke, /if: \$\{\{ matrix\.os == 'macos-latest' \}\}/);
  assert.match(smoke, /\.apm\/hooks\/cli\/bin\/darwin-arm64/);
  assert.match(smoke, /chmod 0755 "\$dir\/cheaper"/);
  // sibling 配置は extract 後・smoke 実行前であること。
  assert.ok(smoke.indexOf("darwin-arm64") < smoke.indexOf("smoke-release.mjs"));
});

test("build-executables builds the cargo release matrix behind a size gate", async () => {
  const build = read("build-executables.yml");
  for (const entry of [
    /os: windows-latest\n\s+target: win32-x64\n\s+cargo_target: x86_64-pc-windows-msvc\n\s+artifact: cheaper-windows-x64\.exe/,
    /os: ubuntu-latest\n\s+target: linux-x64\n\s+cargo_target: x86_64-unknown-linux-musl\n\s+artifact: cheaper-linux-x64/,
    /os: ubuntu-24\.04-arm\n\s+target: linux-arm64\n\s+cargo_target: aarch64-unknown-linux-musl\n\s+artifact: cheaper-linux-arm64/,
    /os: macos-latest\n\s+target: darwin-arm64\n\s+cargo_target: aarch64-apple-darwin\n\s+artifact: cheaper-darwin-arm64/,
  ]) assert.match(build, entry);
  assert.match(build, /cargo build --release --locked --target \$\{\{ matrix\.cargo_target \}\} -p cheaper-cli/);
  assert.match(build, /contains\(matrix\.cargo_target, 'musl'\)[\s\S]*musl-tools/);
  assert.match(build, /node scripts\/check-binary-size\.mjs --file "dist\/bin\/\$\{\{ matrix\.artifact \}\}"/);
  assert.ok(build.indexOf("cargo build") < build.indexOf("check-binary-size.mjs"));
  assert.ok(build.indexOf("check-binary-size.mjs") < build.indexOf("actions/upload-artifact"));
  assert.match(build, /name: bin-\$\{\{ matrix\.target \}\}/);
  assert.match(build, /path: dist\/bin\/\$\{\{ matrix\.artifact \}\}/);
  assert.match(build, /Smoke test executable[\s\S]*--version[\s\S]*models/);
  assert.doesNotMatch(build, /pnpm |build-executable\.mjs|dist\/sea|sea-\$\{\{/);
  const { releaseTargets } = await import("../lib/release-artifacts.mjs");
  assert.deepEqual(releaseTargets.map(({ asset }) => asset), ["cheaper-linux-x64", "cheaper-linux-arm64", "cheaper-windows-x64.exe"]);
  assert.ok(!releaseTargets.some(({ asset }) => asset === "cheaper-darwin-arm64"), "darwin artifact must stay out of releaseTargets");
});

test("workflow pins source SHA and rechecks peeled tag", () => {
  const text = workflow();
  assert.match(job(text, "resolve-release"), /verifyReleaseVersion/);
  assert.match(job(text, "resolve-release"), /git rev-parse HEAD/);
  assert.match(job(text, "build-executables"), /ref: \$\{\{ needs.resolve-release.outputs.sha \}\}/);
  const publish = job(text, "publish-release");
  assert.match(publish, /git fetch[\s\S]*refs\/tags\/\$TAG/);
  assert.match(publish, /FETCH_HEAD\^\{commit\}/);
  assert.match(publish, /RESOLVED_SHA/);
  assert.ok(publish.indexOf("FETCH_HEAD^{commit}") < publish.indexOf("gh release create"));
  assert.equal(text.includes("${{ inputs.tag }}\""), false, "tag must travel through env/action inputs, not shell interpolation");
});

test("workflow dispatch concurrency and token permissions are bounded", () => {
  const text = workflow();
  assert.match(text, /^permissions:\n  contents: read$/m);
  assert.equal((text.match(/contents: write/g) ?? []).length, 1);
  assert.match(job(text, "publish-release"), /permissions:\n      contents: write/);
  assert.deepEqual(text.match(/secrets\.[A-Za-z_]+/g), ["secrets.GITHUB_TOKEN"]);
  assert.match(text, /cancel-in-progress: false/);
  const publish = job(text, "publish-release");
  assert.match(publish, /set -euo pipefail/);
  assert.match(publish, /gh api --paginate --slurp/);
  assert.match(publish, /tag_name === process.env.TAG/);
  assert.ok(publish.indexOf("gh api") < publish.indexOf("gh release create"));
  assert.doesNotMatch(publish, /--clobber|\|\| true|gh release delete/);
  assert.match(text, /^on:\n  workflow_dispatch:/m);
  assert.doesNotMatch(text, /^  (push|pull_request|workflow_run|schedule):/m);
});

test("workflow publishes complete draft before public release", () => {
  const publish = job(workflow(), "publish-release");
  assert.match(publish, /gh release create[\s\S]*--draft[\s\S]*--verify-tag/);
  for (const name of ["cheaper-linux-x64", "cheaper-linux-arm64", "cheaper-windows-x64.exe", "SHA256SUMS", "cheaper-apm-v$VERSION.tar.gz"]) assert.ok(publish.includes(name));
  // Release asset は raw3 + SHA256SUMS + tar.gz の 5 件。darwin binary は含めない。
  assert.match(publish, /gh release create "\$TAG" \\\n\s+dist\/bin\/cheaper-linux-x64 dist\/bin\/cheaper-linux-arm64 dist\/bin\/cheaper-windows-x64\.exe \\\n\s+dist\/release\/SHA256SUMS "dist\/release\/cheaper-apm-v\$VERSION\.tar\.gz"/);
  assert.doesNotMatch(publish, /darwin/);
  assert.match(publish, /gh release edit[\s\S]*--draft=false[\s\S]*--latest=false/);
  assert.ok(publish.indexOf("gh release create") < publish.indexOf("gh release edit"));
});

test("source payload freshness is retired while native version smoke remains", () => {
  assert.doesNotMatch(workflow(), /strict_freshness/);
  const build = read("build-executables.yml");
  assert.match(build, /expected_version:[\s\S]*required: false[\s\S]*type: string/);
  assert.match(build, /if: \$\{\{ inputs.expected_version != '' \}\}/);
  assert.match(build, /EXPECTED_VERSION:[\s\S]*inputs.expected_version/);
  assert.doesNotMatch(build, /strict_freshness|cmp -s|matrix\.payload|lfs: true/);
});

test("legacy release refuses publication with read-only permissions", () => {
  const legacy = read("release.yml");
  assert.match(legacy, /^permissions:\n  contents: read$/m);
  assert.match(legacy, /release-acquisition/);
  assert.match(legacy, /exit 1/);
  assert.doesNotMatch(legacy, /gh release|contents: write|secrets\.|uses:.*build-executables/);
  assert.doesNotMatch(read("ci.yml"), /lfs: true/);
});

test("workflow runs focused suites before build", () => {
  const resolve = job(workflow(), "resolve-release");
  for (const file of ["binary-size.test.mjs", "release-artifacts.test.mjs", "release-workflow.test.mjs"]) assert.ok(resolve.includes(file));
});

test("workflow refuses existing releases and fails API errors", () => {
  const publish = job(workflow(), "publish-release");
  assert.match(publish, /gh api --paginate --slurp/);
  assert.match(publish, /throw new Error\("release already exists/);
  assert.match(publish, /throw new Error\("invalid release-list response/);
  assert.match(publish, /set -euo pipefail/);
  assert.doesNotMatch(publish, /gh api[^\n]*\|\|/);
});

test("tag passes through env and expected version skips when absent", () => {
  const text = workflow();
  assert.match(text, /TAG: \$\{\{ inputs.tag \}\}/);
  assert.doesNotMatch(text, /(?:git|node|gh)[^\n]*\$\{\{ inputs.tag \}\}/);
  assert.match(read("build-executables.yml"), /if: \$\{\{ inputs.expected_version != '' \}\}/);
});

test("cross-OS publisher hashes binaries without executing them", () => {
  const text = workflow();
  const publish = job(text, "publish-release");
  assert.match(publish, /cp dist\/release\/SHA256SUMS dist\/bin\/SHA256SUMS[\s\S]*generate-release-checksums.mjs[\s\S]*--verify/);
  assert.doesNotMatch(publish, /\.exe\s+--version|\.exe\s+models/);
  assert.match(job(text, "assemble-release"), /runs-on: ubuntu-latest/);
});

test("smoke rejects version mismatch and nonzero exit", async () => {
  const { assertSmokeResult } = await import("../smoke-release.mjs");
  assert.doesNotThrow(() => assertSmokeResult({ status: 0, stdout: "1.2.3\n" }, "--version", "1.2.3"));
  assert.throws(() => assertSmokeResult({ status: 0, stdout: "1.2.4\n" }, "--version", "1.2.3"), /version/);
  assert.throws(() => assertSmokeResult({ status: 2, stdout: "1.2.3" }, "--version", "1.2.3"), /exit|failed/);
  assert.throws(() => assertSmokeResult({ status: 0, stdout: '{"ok":false}' }, "models", "1.2.3"), /models/);
  assert.doesNotThrow(() => assertSmokeResult({ status: 0, stdout: '{"ok":true}' }, "models", "1.2.3"));
});

test("bundle smoke dry-run checks every hash and metadata before APM install", async (t) => {
  const { generateManifest, releaseTargets } = await import("../lib/release-artifacts.mjs");
  const { smokeBundle } = await import("../smoke-release.mjs");
  const tmp = fileURLToPath(new URL("../../tmp/", import.meta.url));
  mkdirSync(tmp, { recursive: true });
  const root = mkdtempSync(path.join(tmp, "release-smoke-test-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const pkg = path.join(root, "package");
  const cli = path.join(pkg, ".apm/hooks/cli");
  const assets = path.join(root, "assets");
  mkdirSync(assets);
  for (const { target, executable, asset } of releaseTargets) {
    const bin = path.join(cli, "bin", target);
    mkdirSync(bin, { recursive: true });
    writeFileSync(path.join(bin, executable), "abc");
    writeFileSync(path.join(assets, asset), "abc");
  }
  const manifestFile = path.join(root, "SHA256SUMS");
  const bytes = generateManifest(assets);
  writeFileSync(manifestFile, bytes);
  writeFileSync(path.join(cli, "SHA256SUMS"), bytes);
  writeFileSync(path.join(cli, "version.json"), '{"version":"1.2.3"}');
  const args = { packageDir: pkg, manifestFile, version: "1.2.3", dryRun: true };
  assert.equal(smokeBundle(args).verified, 3);
  assert.throws(() => smokeBundle({ ...args, version: "1.2.4" }), /version/);
  writeFileSync(path.join(cli, "SHA256SUMS"), Buffer.from(bytes.toString().replace(/a/, "b")));
  assert.throws(() => smokeBundle(args), /manifests differ/);
  writeFileSync(path.join(cli, "SHA256SUMS"), bytes);
  for (const { target, executable } of releaseTargets) {
    const exe = path.join(cli, "bin", target, executable);
    writeFileSync(exe, "tampered");
    assert.throws(() => smokeBundle(args), /checksum/);
    writeFileSync(exe, "abc");
  }
});
