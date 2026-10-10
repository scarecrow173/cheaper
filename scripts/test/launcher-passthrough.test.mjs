// Thin PATH launcher contract — issue #85 / TSPEC-034DmUtUjWCThCnJulok0c。
// cheaper.sh / cheaper.ps1 は PATH 上の Cargo install 済み cheaper を
// 起動するだけの thin wrapper であることを検証する:
//   - CLI 存在: argv/stdin/stdout/exit code を verbatim で通す
//   - CLI 不在: install/PATH guidance を stderr へ出し stdout は空、exit 1
// fake PATH 上の stub `cheaper` で CLI を代替する(hook 実 CLI 不要)。

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { chmodSync, existsSync, mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { test } from "node:test";

import { nodeFreePath, repoRoot } from "./lib/apm-fixture.mjs";

const hooksDir = path.join(repoRoot, "packages", "apm", ".apm", "hooks");
const isWindows = process.platform === "win32";
const hasPosixShell = spawnSync("sh", ["-c", "exit 0"], { shell: false }).status === 0;
// Windows では spawn の PATH env だけで `sh` を解決するため、sh.exe の
// ある dir(msys2 usr\bin — msys runtime dll も同居)を PATH に含める。
// where.exe は System32 にあり常に解決できる。
const shDir = isWindows
  ? (() => {
      const w = spawnSync("where", ["sh"], { encoding: "utf8" });
      return w.status === 0 ? path.dirname(w.stdout.split(/\r?\n/)[0].trim()) : null;
    })()
  : null;
const powershell = isWindows
  ? path.join(
      process.env.WINDIR ?? "C:/Windows",
      "System32",
      "WindowsPowerShell",
      "v1.0",
      "powershell.exe",
    )
  : null;
const hasPowershell = powershell ? existsSync(powershell) : false;

// PATH 上に stub CLI を置く dir を作る。stub は argv を `ARGS:` 行で echo
// し、stdin を stdout へ通し、exit 7 で終わる。POSIX 用に extensionless の
// shell script、Windows 用に .cmd(`Get-Command -CommandType Application`
// が解決する)を両方置く — Git Bash の sh は POSIX 側を解決する。
function stubCliDir(root) {
  const dir = path.join(root, "stub-bin");
  mkdirSync(dir, { recursive: true });
  const file = path.join(dir, "cheaper");
  writeFileSync(file, '#!/bin/sh\necho ARGS:"$*"\ncat\nexit 7\n');
  chmodSync(file, 0o755);
  if (isWindows) {
    writeFileSync(
      path.join(dir, "cheaper.cmd"),
      "@echo off\r\necho ARGS:%*\r\nmore\r\nexit /b 7\r\n",
    );
  }
  return dir;
}

// 起動側 process の env を継承しつつ PATH だけを差し替える(PATHEXT 等の
// 実行必須 env を落とさない — PATHEXT 無しでは Get-Command が exe/cmd を
// 解決しない)。
function testEnv(...pathDirs) {
  return { ...process.env, PATH: pathDirs.join(path.delimiter) };
}

test("cheaper.sh passes argv/stdin/stdout/exit code through to PATH-resolved CLI", {
  skip: !hasPosixShell,
}, () => {
  const root = mkdtempSync(path.join(tmpdir(), "launcher-posix-"));
  const env = testEnv(stubCliDir(root), nodeFreePath(root), ...(shDir ? [shDir] : []));
  const r = spawnSync("sh", ["./cheaper.sh", "hook", "pre-read-code", "--target", "windsurf"], {
    cwd: hooksDir,
    env,
    input: "stdin-payload\n",
    encoding: "utf8",
  });
  assert.strictEqual(r.status, 7);
  assert.match(r.stdout, /ARGS:hook pre-read-code --target windsurf/);
  assert.match(r.stdout, /stdin-payload/);
});

test("cheaper.sh writes install guidance to stderr and exits 1 without stdout when CLI missing", {
  skip: !hasPosixShell,
}, () => {
  const root = mkdtempSync(path.join(tmpdir(), "launcher-posix-missing-"));
  const env = testEnv(nodeFreePath(root), ...(shDir ? [shDir] : []));
  const r = spawnSync("sh", ["./cheaper.sh", "hook", "pre-read-code"], {
    cwd: hooksDir,
    env,
    encoding: "utf8",
  });
  assert.strictEqual(r.status, 1);
  assert.strictEqual(r.stdout, "");
  assert.match(r.stderr, /not installed or not on PATH/);
  assert.match(r.stderr, /cargo install --git/);
});

test("cheaper.ps1 passes argv/stdin/stdout/exit code through to PATH-resolved CLI", {
  skip: !hasPowershell,
}, () => {
  const root = mkdtempSync(path.join(tmpdir(), "launcher-ps-"));
  const env = testEnv(stubCliDir(root), nodeFreePath(root));
  const r = spawnSync(
    powershell,
    [
      "-NoProfile",
      "-ExecutionPolicy",
      "Bypass",
      "-File",
      "cheaper.ps1",
      "hook",
      "pre-read-code",
      "--target",
      "windsurf",
    ],
    { cwd: hooksDir, env, input: "stdin-payload\n", encoding: "utf8" },
  );
  assert.strictEqual(r.status, 7);
  assert.match(r.stdout, /ARGS:.*pre-read-code.*--target.*windsurf/);
  assert.match(r.stdout, /stdin-payload/);
});

test("cheaper.ps1 writes install guidance to stderr and exits 1 without stdout when CLI missing", {
  skip: !hasPowershell,
}, () => {
  const root = mkdtempSync(path.join(tmpdir(), "launcher-ps-missing-"));
  const env = testEnv(nodeFreePath(root));
  const r = spawnSync(
    powershell,
    ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", "cheaper.ps1", "hook", "pre-read-code"],
    { cwd: hooksDir, env, encoding: "utf8" },
  );
  assert.strictEqual(r.status, 1);
  assert.strictEqual(r.stdout, "");
  assert.match(r.stderr, /not installed or not on PATH/);
  assert.match(r.stderr, /cargo install --git/);
});
