import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import path from "node:path";
import { afterEach, describe, it } from "node:test";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
// PreToolUse matcher の source of truth — issue #118 で hook 宣言は
// per-target sibling package に分離され、claude/codex 向けの nested
// PreToolUse group は apm-hook-claude / apm-hook-codex が持つ(内容同一)。
const hooksPath = path.join(repoRoot, "packages/apm-hook-claude/.apm/hooks/hooks.json");
const hooksJson = JSON.parse(readFileSync(hooksPath, "utf8"));

const pretooluseGroups = (hooksJson.hooks?.PreToolUse ?? []).filter((group) =>
  (group.hooks ?? []).some((h) => String(h.command).includes("--target pretooluse")),
);
const matcher = new RegExp(pretooluseGroups[0]?.matcher ?? "$^");

describe("source hooks.json PreToolUse matcher", () => {
  it("matches read/grep/bash/powershell in every ASCII case variant", () => {
    for (const name of [
      "Read",
      "READ",
      "read",
      "rEaD",
      "Grep",
      "GREP",
      "grep",
      "gReP",
      "Bash",
      "BASH",
      "bash",
      "PowerShell",
      "POWERSHELL",
      "powershell",
      "pOwErShElL",
    ]) {
      assert.ok(matcher.test(name), `expected match: ${name}`);
    }
  });

  it("rejects prefix/suffix variants and unknown tools", () => {
    for (const name of [
      "reads",
      "xread",
      "readx",
      "grepx",
      "xgrep",
      "bashx",
      "xpowershell",
      "Write",
      "view",
      "shell",
      "",
      " read",
      "read ",
      "read,grep",
    ]) {
      assert.ok(!matcher.test(name), `expected non-match: ${name}`);
    }
  });
});

// `install-devin-hooks.ps1` は `cheaper hook-config` への thin wrapper —
// config 変更のロジックは Rust 側 integration test が担い、ここでは
// wrapper としての UX(プレビュー文・capability 事前確認・exit code
// 伝播)を検証する。
const windir = process.env.SystemRoot || "C:\\Windows";
const psPath = path.join(windir, "System32", "WindowsPowerShell", "v1.0", "powershell.exe");
const scriptPath = path.join(
  repoRoot,
  "packages/apm/.apm/skills/cheaper-usage/scripts/install-devin-hooks.ps1",
);
const CANON_COMMAND = "cheaper hook pre-tool-use --target devin";
const CANON_MATCHER = "^([Rr][Ee][Aa][Dd]|[Gg][Rr][Ee][Pp]|[Ee][Xx][Ee][Cc])$";
const CAP_OK =
  '{"schemaVersion":1,"targets":["windsurf","copilot","pretooluse","cursor","devin"],"hookConfig":true}';
const runPsAvailable = process.platform === "win32";
const cliExe = path.join(
  repoRoot,
  "target",
  "debug",
  process.platform === "win32" ? "cheaper.exe" : "cheaper",
);
const cliBuilt = existsSync(cliExe);

const createdRoots = [];
afterEach(() => {
  for (const root of createdRoots.splice(0)) {
    rmSync(root, { recursive: true, force: true });
  }
});

function makeConsumer({ cliStdout = CAP_OK, cliExit = 0, withCli = true } = {}) {
  const base = path.join(repoRoot, "tmp");
  mkdirSync(base, { recursive: true });
  const root = mkdtempSync(path.join(base, "devin-hooks-fixture-"));
  createdRoots.push(root);
  const bin = path.join(root, "fixturebin");
  mkdirSync(bin, { recursive: true });
  if (withCli) {
    const lines = ["@echo off"];
    if (cliStdout !== null) lines.push(`echo ${cliStdout}`);
    lines.push(`exit /b ${cliExit}`);
    writeFileSync(path.join(bin, "cheaper.cmd"), `${lines.join("\r\n")}\r\n`);
  }
  return { root, bin };
}

/// 実 binary 版 consumer — `target/debug` を PATH に載せて wrapper の
/// capability 確認・`hook-config` 呼出しを実 CLI で通す。
const REAL_BIN = () => path.dirname(cliExe);

function makeRealConsumer() {
  const base = path.join(repoRoot, "tmp");
  mkdirSync(base, { recursive: true });
  const root = mkdtempSync(path.join(base, "devin-hooks-fixture-"));
  createdRoots.push(root);
  return { root, bin: path.dirname(cliExe) };
}

function writeAt(root, rel, content) {
  const target = path.join(root, rel);
  mkdirSync(path.dirname(target), { recursive: true });
  writeFileSync(target, content);
  return target;
}

function runInstaller(root, { apply = false, binRoot = root } = {}) {
  const env = {
    ...process.env,
    PATH: `${binRoot === root ? path.join(binRoot, "fixturebin") : binRoot};${windir}\\System32;${windir}\\System32\\WindowsPowerShell\\v1.0`,
  };
  const args = [
    "-NoProfile",
    "-ExecutionPolicy",
    "Bypass",
    "-File",
    scriptPath,
    "-ProjectRoot",
    root,
  ];
  if (apply) args.push("-Apply");
  const r = spawnSync(psPath, args, { encoding: "utf8", env });
  return {
    code: r.status,
    stdout: r.stdout ?? "",
    stderr: r.stderr ?? "",
    combined: `${r.stdout ?? ""}\n${r.stderr ?? ""}`,
  };
}

function configAt(root) {
  return path.join(root, ".devin", "config.json");
}

describe("install-devin-hooks.ps1 (thin wrapper)", { skip: !runPsAvailable }, () => {
  it("requires Windows PowerShell 5.1 on PATH-independent system location", () => {
    assert.ok(existsSync(psPath), `powershell.exe not found at ${psPath}`);
  });

  it("fails without a cheaper CLI on PATH (preview and apply)", () => {
    const { root } = makeConsumer({ withCli: false });
    for (const apply of [false, true]) {
      const r = runInstaller(root, { apply });
      assert.notEqual(r.code, 0, `apply=${apply}: ${r.combined}`);
      assert.match(r.combined, /not found on PATH/i, `apply=${apply}: ${r.combined}`);
      assert.match(r.combined, /cargo install/i, `apply=${apply}: ${r.combined}`);
    }
  });

  it("rejects a CLI that reports no hookConfig capability", () => {
    const oldCli =
      '{"schemaVersion":1,"targets":["windsurf","copilot","pretooluse","cursor","devin"]}';
    const { root } = makeConsumer({ cliStdout: oldCli });
    const r = runInstaller(root, { apply: true });
    assert.notEqual(r.code, 0, r.combined);
    assert.match(r.combined, /hook-config|hookConfig/i, r.combined);
    assert.equal(existsSync(configAt(root)), false);
  });

  it("rejects capability handshake failures without writes", () => {
    const cases = [
      { withCli: false, name: "cli missing" },
      { cliStdout: CAP_OK, cliExit: 1, name: "nonzero exit" },
      { cliStdout: null, cliExit: 0, name: "empty output" },
      { cliStdout: "not json", cliExit: 0, name: "malformed output" },
      {
        cliStdout: '{"schemaVersion":1,"targets":["windsurf"],"hookConfig":true}',
        cliExit: 0,
        name: "missing devin",
      },
      {
        cliStdout: '{"schemaVersion":1,"targets":["devin"],"hookConfig":false}',
        cliExit: 0,
        name: "hookConfig false",
      },
    ];
    for (const c of cases) {
      const { root } = makeConsumer(c);
      writeAt(root, ".devin/config.json", '{"keep":"yes","hooks":{}}');
      const r = runInstaller(root, { apply: true });
      assert.notEqual(r.code, 0, `${c.name}: ${r.combined}`);
      assert.equal(readFileSync(configAt(root), "utf8"), '{"keep":"yes","hooks":{}}', c.name);
    }
  });
});

describe("install-devin-hooks.ps1 + real cheaper CLI", {
  skip: !runPsAvailable || !cliBuilt,
}, () => {
  it("apply registers the canonical read|grep|exec entry and preview reports it", () => {
    const { root } = makeRealConsumer();
    const realBin = REAL_BIN();
    const preview = runInstaller(root, { binRoot: realBin });
    assert.equal(preview.code, 0, preview.combined);
    assert.match(preview.combined, /would add/i, preview.combined);

    const r = runInstaller(root, { apply: true, binRoot: realBin });
    assert.equal(r.code, 0, r.combined);
    const updated = JSON.parse(readFileSync(configAt(root), "utf8"));
    const groups = updated.hooks.PreToolUse;
    assert.equal(groups.length, 1);
    assert.equal(groups[0].matcher, CANON_MATCHER);
    assert.deepEqual(groups[0].hooks, [{ type: "command", command: CANON_COMMAND }]);

    const after = runInstaller(root, { binRoot: realBin });
    assert.equal(after.code, 0, after.combined);
    assert.match(after.combined, /already registered/i, after.combined);

    // sync は冪等 — 再 apply しても entry は 1 件のまま。
    const second = runInstaller(root, { apply: true, binRoot: realBin });
    assert.equal(second.code, 0, second.combined);
    const again = JSON.parse(readFileSync(configAt(root), "utf8"));
    assert.equal(again.hooks.PreToolUse.length, 1);
  });

  it("preserves unrelated config and converges the legacy matcher", () => {
    const { root } = makeRealConsumer();
    const realBin = REAL_BIN();
    const original = {
      meta: { nested: { jp: "日本" } },
      hooks: {
        PreToolUse: [
          {
            matcher: "^([Rr][Ee][Aa][Dd]|[Gg][Rr][Ee][Pp])$",
            hooks: [{ type: "command", command: CANON_COMMAND }],
          },
        ],
      },
    };
    writeAt(root, ".devin/config.json", JSON.stringify(original, null, 2));
    const r = runInstaller(root, { apply: true, binRoot: realBin });
    assert.equal(r.code, 0, r.combined);
    const updated = JSON.parse(readFileSync(configAt(root), "utf8"));
    assert.equal(updated.meta.nested.jp, "日本");
    assert.equal(updated.hooks.PreToolUse[0].matcher, CANON_MATCHER);
  });

  it("warns on legacy cheaper registrations in known sources", () => {
    const { root } = makeRealConsumer();
    const realBin = REAL_BIN();
    writeAt(
      root,
      ".windsurf/hooks.json",
      JSON.stringify({
        hooks: {
          pre_read_code: [{ command: "sh ./cheaper.sh hook pre-read-code --target windsurf" }],
        },
      }),
    );
    // 降格後は conflict 有りでも install は収束する — sync は warning として
    // 報告しつつ canonical entry を書き込む。
    const r = runInstaller(root, { apply: true, binRoot: realBin });
    assert.equal(r.code, 0, r.combined);
    assert.equal(existsSync(configAt(root)), true);
    const config = JSON.parse(readFileSync(configAt(root), "utf8"));
    const own = (config.hooks?.PreToolUse ?? []).filter(
      (g) =>
        g.matcher === CANON_MATCHER && (g.hooks ?? []).some((h) => h.command === CANON_COMMAND),
    );
    assert.equal(own.length, 1, JSON.stringify(config));

    // wrapper は成功時に sync envelope を echo しないため、warning surfacing は
    // 実 CLI の verify を direct spawn で確認する(verify は capability check を
    // 持たないので PATH 加工は不要)。
    const verify = spawnSync(cliExe, ["hook-config", "verify", "--project-root", root], {
      encoding: "utf8",
    });
    assert.equal(verify.status, 0, `${verify.stdout ?? ""}\n${verify.stderr ?? ""}`);
    const vresult = JSON.parse(verify.stdout).result;
    const windsurfFile = path.join(root, ".windsurf", "hooks.json");
    assert.ok(
      (vresult.legacyConflicts ?? []).some((f) => path.resolve(f) === windsurfFile),
      `legacyConflicts: ${JSON.stringify(vresult.legacyConflicts)}`,
    );
    assert.ok(
      (vresult.findings ?? []).some((f) => f.startsWith("legacy-conflict:")),
      `findings: ${JSON.stringify(vresult.findings)}`,
    );
    assert.equal(vresult.clean, true, JSON.stringify(vresult.findings));

    // `warning` field 自体の確認 — sync は冪等なので再実行しても `warning`
    // が返る。`check_capability` が PATH 上の cheaper を要求するため CLI の
    // dir を PATH へ含めて spawn する。
    const sync = spawnSync(cliExe, ["hook-config", "sync", "--project-root", root], {
      encoding: "utf8",
      env: {
        ...process.env,
        PATH: `${path.dirname(cliExe)}${path.delimiter}${process.env.PATH}`,
      },
    });
    assert.equal(sync.status, 0, `${sync.stdout ?? ""}\n${sync.stderr ?? ""}`);
    const sresult = JSON.parse(sync.stdout).result;
    assert.equal(typeof sresult.warning, "string", JSON.stringify(sresult));
    assert.ok(sresult.warning.length > 0);
    assert.ok(
      (sresult.legacyConflicts ?? []).some((f) => path.resolve(f) === windsurfFile),
      `legacyConflicts: ${JSON.stringify(sresult.legacyConflicts)}`,
    );
  });

  it("rejects malformed config without writes", () => {
    const { root } = makeRealConsumer();
    const realBin = REAL_BIN();
    const cfg = writeAt(root, ".devin/config.json", "{ not json");
    const r = runInstaller(root, { apply: true, binRoot: realBin });
    assert.notEqual(r.code, 0, r.combined);
    assert.equal(readFileSync(cfg, "utf8"), "{ not json");
  });

  it("rejects reparse point components on the config path", () => {
    const { root } = makeRealConsumer();
    const realBin = REAL_BIN();
    const real = path.join(root, "realdevin");
    mkdirSync(real, { recursive: true });
    writeFileSync(path.join(real, "config.json"), '{"hooks":{}}');
    symlinkSync(real, path.join(root, ".devin"), "junction");
    const r = runInstaller(root, { apply: true, binRoot: realBin });
    assert.notEqual(r.code, 0, r.combined);
  });
});
