import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { cpSync, mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../..", import.meta.url));
const hookPath = "packages/apm/.apm/hooks";
function fixture(t) {
  const dir = mkdtempSync(path.join(os.tmpdir(), "cheaper-source-"));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  cpSync(path.join(root, "Cargo.toml"), path.join(dir, "Cargo.toml"));
  cpSync(path.join(root, hookPath), path.join(dir, hookPath), {
    recursive: true,
    filter: (p) => !/[\\/]bin[\\/][^\\/]+[\\/]/.test(p) &&
      !/[\\/]bin[\\/](win32-x64|linux-x64|linux-arm64)$/.test(p),
  });
  return dir;
}
async function check(dir) {
  const { checkApmPayload } = await import("../check-apm-payload.mjs");
  return checkApmPayload(dir);
}
test("binary-free source assets satisfy the source contract", async (t) => {
  assert.deepEqual(await check(fixture(t)), []);
});
for (const asset of ["cheaper", "cheaper.ps1", "hooks.json", "cli/version.json", "cli/policy.yaml", "cli/workers/bulk-reader.md", "cli/workers/schemas/bulk-reader.input.schema.json"]) {
  test(`missing required asset is rejected: ${asset}`, async (t) => {
    const dir = fixture(t);
    rmSync(path.join(dir, hookPath, asset));
    assert.match((await check(dir)).join("\n"), /missing|regular file/);
  });
}
test("small assets reject pointers and directories", async (t) => {
  const dir = fixture(t);
  const file = path.join(dir, hookPath, "cli/policy.yaml");
  writeFileSync(file, "version https://git-lfs.github.com/spec/v1\noid sha256:abc\nsize 100\n");
  assert.match((await check(dir)).join("\n"), /LFS pointer/);
  rmSync(file);
  mkdirSync(file);
  assert.match((await check(dir)).join("\n"), /regular file/);
});
for (const value of ['{"version":"0.0.0"}', '{"version":42}', '{"version":"1.0.0","extra":true}', '{bad']) {
  test(`version metadata rejects drift or malformed input: ${value}`, async (t) => {
    const dir = fixture(t);
    writeFileSync(path.join(dir, hookPath, "cli/version.json"), value);
    assert.match((await check(dir)).join("\n"), /version|JSON/);
  });
}
test("hooks and schema JSON must parse", async (t) => {
  const dir = fixture(t);
  writeFileSync(path.join(dir, hookPath, "hooks.json"), "{bad");
  assert.match((await check(dir)).join("\n"), /JSON/);
  writeFileSync(path.join(dir, hookPath, "hooks.json"), "{}");
  writeFileSync(path.join(dir, hookPath, "cli/workers/schemas/bulk-reader.input.schema.json"), "{bad");
  assert.match((await check(dir)).join("\n"), /invalid JSON.*schema/);
});
test("empty required assets are rejected", async (t) => {
  const dir = fixture(t);
  writeFileSync(path.join(dir, hookPath, "cli/policy.yaml"), "");
  assert.match((await check(dir)).join("\n"), /nonempty regular file/);
});
test("Git index and policies exclude all platform payload directories", () => {
  assert.equal(execFileSync("git", ["ls-files", "packages/apm/.apm/hooks/cli/bin/*/"], { cwd: root, encoding: "utf8" }).trim(), "");
  assert.doesNotMatch(readFileSync(path.join(root, ".gitattributes"), "utf8"), /cli\/bin\/.*filter=lfs/);
  for (const target of ["win32-x64", "linux-x64", "linux-arm64", "darwin-arm64"]) {
    const file = `packages/apm/.apm/hooks/cli/bin/${target}/cheaper`;
    assert.equal(execFileSync("git", ["check-ignore", "--no-index", file], { cwd: root, encoding: "utf8" }).trim(), file);
  }
});
