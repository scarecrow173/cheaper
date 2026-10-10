// Slim APM payload contract — issue #85 / TSPEC-034DmUtUjWCThCnJulok0c。
// check-apm-payload.mjs の検証範囲を fixture コピー上で assert する:
// 必須 launcher asset の存在・cli/ 不在・binary payload 排除・
// launcher byte 契約・hooks.json ↔ launcher 整合。
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import {
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import os from "node:os";
import path from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../..", import.meta.url));
const hookPath = "packages/apm/.apm/hooks";
const hookDir = path.join(root, hookPath);

function fixture(t) {
  const dir = mkdtempSync(path.join(os.tmpdir(), "cheaper-source-"));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  cpSync(hookDir, path.join(dir, hookPath), { recursive: true });
  return dir;
}

async function check(dir) {
  const { checkApmPayload } = await import("../check-apm-payload.mjs");
  return checkApmPayload(dir);
}

test("slim source assets satisfy the payload contract", async (t) => {
  assert.deepEqual(await check(fixture(t)), []);
});

for (const asset of ["cheaper.sh", "cheaper.ps1", "hooks.json"]) {
  test(`missing required asset is rejected: ${asset}`, async (t) => {
    const dir = fixture(t);
    rmSync(path.join(dir, hookPath, asset));
    assert.match((await check(dir)).join("\n"), /missing|regular file/);
  });
}

test("cli/ payload directory is rejected", async (t) => {
  const dir = fixture(t);
  mkdirSync(path.join(dir, hookPath, "cli"));
  assert.match((await check(dir)).join("\n"), /cli\/ payload must not exist/);
});

test("executable payloads are rejected by magic-byte sniffing", async (t) => {
  const dir = fixture(t);
  const pe = path.join(dir, hookPath, "sneaky.exe.bin");
  writeFileSync(pe, Buffer.from([0x4d, 0x5a, 0x90, 0x00]));
  assert.match((await check(dir)).join("\n"), /executable payload is forbidden.*PE/);
  rmSync(pe);
  const elf = path.join(dir, hookPath, "cheaper.sh");
  writeFileSync(elf, Buffer.from([0x7f, 0x45, 0x4c, 0x46, 0x02]));
  assert.match((await check(dir)).join("\n"), /executable payload is forbidden.*ELF/);
});

test("non-allowlisted extensions are rejected", async (t) => {
  const dir = fixture(t);
  writeFileSync(path.join(dir, hookPath, "notes.ini"), "x=1\n");
  assert.match((await check(dir)).join("\n"), /extension not allowed.*\.ini/);
});

test("cheaper.ps1 must be UTF-8 BOM + CRLF", async (t) => {
  const dir = fixture(t);
  const file = path.join(dir, hookPath, "cheaper.ps1");
  writeFileSync(file, "echo hi\r\n", "utf8");
  assert.match((await check(dir)).join("\n"), /UTF-8 BOM/);
  writeFileSync(file, "﻿echo hi\n", "utf8");
  assert.match((await check(dir)).join("\n"), /CRLF/);
});

test("cheaper.sh must be #!/bin/sh shebang + LF", async (t) => {
  const dir = fixture(t);
  const file = path.join(dir, hookPath, "cheaper.sh");
  writeFileSync(file, "#!/bin/bash\necho hi\n", "utf8");
  assert.match((await check(dir)).join("\n"), /#!\/bin\/sh shebang/);
  writeFileSync(file, "#!/bin/sh\r\necho hi\r\n", "utf8");
  assert.match((await check(dir)).join("\n"), /LF line endings/);
});

test("cheaper.sh git index mode is 100755 in the repo", () => {
  const mode = execFileSync("git", ["ls-files", "-s", `${hookPath}/cheaper.sh`], {
    cwd: root,
    encoding: "utf8",
  }).match(/^(\d+)/)?.[1];
  assert.strictEqual(mode, "100755");
});

test("hooks.json consistency: launcher refs and no bare ./cheaper", async (t) => {
  const dir = fixture(t);
  const file = path.join(dir, hookPath, "hooks.json");
  const hooks = JSON.parse(readFileSync(file, "utf8"));
  hooks.hooks.pre_read_code[0].command = "./cheaper hook pre-read-code --target windsurf";
  writeFileSync(file, JSON.stringify(hooks));
  const output = (await check(dir)).join("\n");
  assert.match(output, /bare "\.\/cheaper"/);
  assert.match(output, /sh \.\/cheaper\.sh/);
});

test("hooks.json powershell entries must reference ./cheaper.ps1", async (t) => {
  const dir = fixture(t);
  const file = path.join(dir, hookPath, "hooks.json");
  const hooks = JSON.parse(readFileSync(file, "utf8"));
  hooks.hooks.pre_read_code[0].powershell = "powershell -File ./cheaper.bat hook x";
  writeFileSync(file, JSON.stringify(hooks));
  assert.match((await check(dir)).join("\n"), /\.\/cheaper\.ps1/);
});

test("hooks.json must parse", async (t) => {
  const dir = fixture(t);
  writeFileSync(path.join(dir, hookPath, "hooks.json"), "{bad");
  assert.match((await check(dir)).join("\n"), /JSON/);
});

test("payload contains no cli/ directory or executable files", () => {
  assert.strictEqual(existsSync(path.join(hookDir, "cli")), false, "cli/ must not exist");
  const files = execFileSync("git", ["ls-files", hookPath], { cwd: root, encoding: "utf8" })
    .trim()
    .split("\n");
  for (const file of files) {
    const bytes = readFileSync(path.join(root, file));
    const pe = bytes.subarray(0, 2).equals(Buffer.from([0x4d, 0x5a]));
    const elf = bytes.subarray(0, 4).equals(Buffer.from([0x7f, 0x45, 0x4c, 0x46]));
    assert.ok(!pe && !elf, `executable payload in Git: ${file}`);
  }
});
