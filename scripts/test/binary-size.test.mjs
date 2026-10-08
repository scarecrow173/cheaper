import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("../../", import.meta.url));
const SCRIPT = path.join(ROOT, "scripts", "check-binary-size.mjs");
const MIB = 1024 * 1024;

const run = (args) => spawnSync(process.execPath, [SCRIPT, ...args], { encoding: "utf8" });

function fixture(t) {
  mkdirSync(path.join(ROOT, "tmp"), { recursive: true });
  const dir = mkdtempSync(path.join(ROOT, "tmp", "binary-size-test-"));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  return dir;
}

test("file under the limit passes", async (t) => {
  const dir = fixture(t);
  const file = path.join(dir, "cheaper");
  writeFileSync(file, Buffer.alloc(MIB - 1, 0x61));
  const result = run(["--file", file, "--max-mb", "1"]);
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /binary size OK/);
});

test("file exactly at the limit passes", async (t) => {
  const dir = fixture(t);
  const file = path.join(dir, "cheaper");
  writeFileSync(file, Buffer.alloc(MIB, 0x61));
  const result = run(["--file", file, "--max-mb", "1"]);
  assert.equal(result.status, 0, result.stderr);
});

test("file over the limit fails", async (t) => {
  const dir = fixture(t);
  const file = path.join(dir, "cheaper");
  writeFileSync(file, Buffer.alloc(MIB + 1, 0x61));
  const result = run(["--file", file, "--max-mb", "1"]);
  assert.equal(result.status, 1);
  assert.match(result.stderr, /exceeds size limit/);
});

test("default limit is 20 MiB", async (t) => {
  const dir = fixture(t);
  const file = path.join(dir, "cheaper");
  writeFileSync(file, Buffer.alloc(1024, 0x61));
  const result = run(["--file", file]);
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /20 MiB/);
});

test("missing file fails", () => {
  const result = run(["--file", path.join(ROOT, "tmp", "does-not-exist-cheaper")]);
  assert.equal(result.status, 1);
  assert.match(result.stderr, /missing|unreadable/);
});

test("non-regular file (directory) fails", async (t) => {
  const dir = fixture(t);
  const result = run(["--file", dir, "--max-mb", "1"]);
  assert.equal(result.status, 1);
  assert.match(result.stderr, /regular file/);
});

test("bad arguments fail", async (t) => {
  const dir = fixture(t);
  const file = path.join(dir, "cheaper");
  writeFileSync(file, "abc");
  for (const args of [
    [],
    ["--max-mb", "1"],
    ["--file"],
    ["--file", file, "--bogus", "1"],
    ["--file", file, "--file", file],
    ["--file", file, "--max-mb", "abc"],
    ["--file", file, "--max-mb", "0"],
    ["--file", file, "--max-mb", "-1"],
    ["--file", file, "--max-mb"],
  ]) {
    const result = run(args);
    assert.equal(result.status, 1, `expected failure for ${JSON.stringify(args)}`);
  }
});
