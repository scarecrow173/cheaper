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
const hooksPath = path.join(repoRoot, "packages/apm/.apm/hooks/hooks.json");
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

const windir = process.env.SystemRoot || "C:\\Windows";
const psPath = path.join(windir, "System32", "WindowsPowerShell", "v1.0", "powershell.exe");
const scriptPath = path.join(
  repoRoot,
  "packages/apm/.apm/skills/cheaper-usage/scripts/install-devin-hooks.ps1",
);
const CANON_COMMAND = "cheaper hook pre-tool-use --target devin";
const CANON_MATCHER = "^([Rr][Ee][Aa][Dd]|[Gg][Rr][Ee][Pp])$";
const CAP_OK = '{"schemaVersion":1,"targets":["windsurf","copilot","pretooluse","cursor","devin"]}';
const CAP_SENTINEL = "CAP_SENTINEL_9xZ_secret";
const CFG_SENTINEL = "CFG_SENTINEL_7qW_secret";
const runPsAvailable = process.platform === "win32";

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

function writeAt(root, rel, content) {
  const target = path.join(root, rel);
  mkdirSync(path.dirname(target), { recursive: true });
  writeFileSync(target, content);
  return target;
}

function readBytes(p) {
  return readFileSync(p);
}

function runInstaller(root, { apply = false, binRoot = root } = {}) {
  const env = {
    ...process.env,
    PATH: `${path.join(binRoot, "fixturebin")};${windir}\\System32;${windir}\\System32\\WindowsPowerShell\\v1.0`,
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

describe("install-devin-hooks.ps1", { skip: !runPsAvailable }, () => {
  it("requires Windows PowerShell 5.1 on PATH-independent system location", () => {
    assert.ok(existsSync(psPath), `powershell.exe not found at ${psPath}`);
  });

  it("preview creates no files or directories and needs no CLI", () => {
    const { root } = makeConsumer({ withCli: false });
    const r = runInstaller(root);
    assert.equal(r.code, 0, r.combined);
    assert.equal(existsSync(path.join(root, ".devin")), false);
  });

  it("preview leaves existing config bytes unchanged and hides values", () => {
    const { root } = makeConsumer({ withCli: false });
    const original = `{\r\n  "secret": "${CFG_SENTINEL}",\r\n  "hooks": {}\r\n}\r\n`;
    const cfg = writeAt(root, ".devin/config.json", original);
    const r = runInstaller(root);
    assert.equal(r.code, 0, r.combined);
    assert.equal(readBytes(cfg).toString("utf8"), original);
    assert.ok(!r.combined.includes(CFG_SENTINEL), r.combined);
  });

  it("apply adds the canonical entry and preserves unknown data", () => {
    const { root } = makeConsumer();
    const original = {
      meta: { nested: { list: [1, { k: null }], jp: "日本" }, arr: [true, null, "x"] },
      hooks: {
        PostToolUse: [{ matcher: "Bash", hooks: [{ type: "command", command: "echo hi" }] }],
      },
    };
    const cfg = writeAt(root, ".devin/config.json", `${JSON.stringify(original, null, 2)}\r\n`);
    const r = runInstaller(root, { apply: true });
    assert.equal(r.code, 0, r.combined);
    const updated = JSON.parse(readFileSync(cfg, "utf8"));
    assert.deepEqual(updated.meta, original.meta);
    assert.deepEqual(updated.hooks.PostToolUse, original.hooks.PostToolUse);
    const groups = updated.hooks.PreToolUse;
    assert.equal(groups.length, 1);
    assert.equal(groups[0].matcher, CANON_MATCHER);
    assert.deepEqual(groups[0].hooks, [{ type: "command", command: CANON_COMMAND }]);
    assert.equal(updated.meta.nested.jp, "日本");
    assert.equal(existsSync(path.join(root, ".devin", "hooks.v1.json")), false);
    assert.ok(!r.combined.includes(CFG_SENTINEL));
  });

  it("re-apply is a bytewise no-op", () => {
    const { root } = makeConsumer();
    writeAt(root, ".devin/config.json", '{"hooks":{}}');
    const cfg = configAt(root);
    const first = runInstaller(root, { apply: true });
    assert.equal(first.code, 0, first.combined);
    const afterFirst = readBytes(cfg);
    const second = runInstaller(root, { apply: true });
    assert.equal(second.code, 0, second.combined);
    assert.deepEqual(readBytes(cfg), afterFirst);
  });

  it("existing canonical entry still requires a capable CLI on apply", () => {
    const canonical = JSON.stringify({
      hooks: {
        PreToolUse: [
          { matcher: CANON_MATCHER, hooks: [{ type: "command", command: CANON_COMMAND }] },
        ],
      },
    });
    for (const c of [
      { withCli: false, name: "cli missing" },
      { cliStdout: CAP_OK, cliExit: 1, name: "old cli nonzero" },
      { cliStdout: null, cliExit: 0, name: "empty output" },
    ]) {
      const { root } = makeConsumer(c);
      const cfg = writeAt(root, ".devin/config.json", canonical);
      const r = runInstaller(root, { apply: true });
      assert.notEqual(r.code, 0, `${c.name}: ${r.combined}`);
      assert.equal(readBytes(cfg).toString("utf8"), canonical, c.name);
    }
    const { root } = makeConsumer();
    const cfg = writeAt(root, ".devin/config.json", canonical);
    const r = runInstaller(root, { apply: true });
    assert.equal(r.code, 0, r.combined);
    assert.equal(readBytes(cfg).toString("utf8"), canonical);
  });

  it("capability handshake failures write nothing and print guidance", () => {
    const cases = [
      { withCli: false, name: "cli missing" },
      { cliStdout: CAP_OK, cliExit: 1, name: "nonzero exit" },
      { cliStdout: null, cliExit: 0, name: "empty output" },
      { cliStdout: "not json", cliExit: 0, name: "malformed output" },
      { cliStdout: '{"schemaVersion":2,"targets":["devin"]}', cliExit: 0, name: "unknown schema" },
      {
        cliStdout: '{"schemaVersion":1,"targets":["windsurf"]}',
        cliExit: 0,
        name: "missing devin",
      },
      { cliStdout: '{"schemaVersion":1,"targets":["Devin"]}', cliExit: 0, name: "case mismatch" },
      {
        cliStdout: '{"schemaVersion":1.0,"targets":["devin"]}',
        cliExit: 0,
        name: "float schemaVersion",
      },
      {
        cliStdout: '{"schemaVersion":"1","targets":["devin"]}',
        cliExit: 0,
        name: "string schemaVersion",
      },
      { cliStdout: '{"schemaVersion":1,"targets":"devin"}', cliExit: 0, name: "scalar targets" },
      {
        cliStdout: '{"schemaVersion":1,"targets":["devin",1]}',
        cliExit: 0,
        name: "nonstring target",
      },
      { cliStdout: '[{"schemaVersion":1,"targets":["devin"]}]', cliExit: 0, name: "root array" },
      {
        cliStdout: '{"SchemaVersion":1,"targets":["devin"]}',
        cliExit: 0,
        name: "miscased schemaVersion",
      },
      {
        cliStdout: `{"schemaVersion":1,"targets":["devin"],"note":"${CAP_SENTINEL}"}`,
        cliExit: 0,
        name: "sentinel payload",
      },
    ];
    for (const c of cases) {
      const { root } = makeConsumer(c);
      const original = '{"keep":"yes","hooks":{}}';
      const cfg = writeAt(root, ".devin/config.json", original);
      const r = runInstaller(root, { apply: true });
      const accepted = c.name === "sentinel payload" ? { code: 0 } : { code: "nonzero" };
      if (accepted.code === 0) {
        assert.equal(r.code, 0, `${c.name}: ${r.combined}`);
        assert.ok(!r.combined.includes(CAP_SENTINEL), c.name);
        const written = readFileSync(cfg, "utf8");
        assert.ok(!written.includes(CAP_SENTINEL), c.name);
        const updated = JSON.parse(written);
        assert.equal(updated.keep, "yes");
      } else {
        assert.notEqual(r.code, 0, `${c.name}: ${r.combined}`);
        assert.equal(readBytes(cfg).toString("utf8"), original, c.name);
        assert.match(r.combined, /cargo install/i, c.name);
        assert.ok(!r.combined.includes(CAP_SENTINEL), c.name);
      }
    }
  });

  it("rejects malformed, ambiguous, and non-schema configs without writes", () => {
    const configs = [
      "{ not json",
      "[]",
      '"text"',
      "null",
      '{"hooks":1}',
      '{"hooks":{"PreToolUse":"x"}}',
      '{"hooks":{"PreToolUse":[7]}}',
      '{"a":1,"a":2}',
      '{"a":1,"\\u0061":2}',
      '{"Key":1,"key":2}',
      '{"n":123456789012345678901234567890}',
      '{"n":0.12345678901234567890123456789}',
      '{"n":1e5}',
    ];
    for (const content of configs) {
      const { root } = makeConsumer();
      const cfg = writeAt(root, ".devin/config.json", content);
      const r = runInstaller(root, { apply: true });
      assert.notEqual(r.code, 0, `config ${content}: ${r.combined}`);
      assert.equal(readBytes(cfg).toString("utf8"), content, content);
    }
  });

  it("rejects modified, extra-attribute, or duplicated own entries", () => {
    const own = (extra) => ({
      matcher: CANON_MATCHER,
      hooks: [{ type: "command", command: CANON_COMMAND, ...extra }],
    });
    const fixtures = [
      { matcher: ".*", hooks: [{ type: "command", command: CANON_COMMAND }] },
      { ...own(), extra: "x" },
      { matcher: CANON_MATCHER, hooks: [{ type: "command", command: CANON_COMMAND, note: "x" }] },
    ];
    for (const group of fixtures) {
      const { root } = makeConsumer();
      const content = JSON.stringify({ hooks: { PreToolUse: [group] } });
      const cfg = writeAt(root, ".devin/config.json", content);
      const r = runInstaller(root, { apply: true });
      assert.notEqual(r.code, 0, `group ${content}: ${r.combined}`);
      assert.equal(readBytes(cfg).toString("utf8"), content);
    }
    const { root } = makeConsumer();
    const dup = JSON.stringify({
      hooks: { PreToolUse: [own(), own()] },
    });
    const cfg = writeAt(root, ".devin/config.json", dup);
    const r = runInstaller(root, { apply: true });
    assert.notEqual(r.code, 0, r.combined);
    assert.equal(readBytes(cfg).toString("utf8"), dup);
  });

  it("detects known legacy cheaper read hooks and stops apply", () => {
    const windsurfHook = JSON.stringify({
      hooks: {
        pre_read_code: [{ command: "sh ./cheaper.sh hook pre-read-code --target windsurf" }],
      },
    });
    const claudeHook = JSON.stringify({
      hooks: {
        PreToolUse: [
          {
            matcher: "Read",
            hooks: [{ type: "command", command: "cheaper hook pre-tool-use --target pretooluse" }],
          },
        ],
      },
    });
    const psField = JSON.stringify({
      hooks: {
        pre_read_code: [
          {
            powershell: 'powershell -File "./cheaper.ps1" hook pre-read-code --target windsurf',
          },
        ],
      },
    });
    const cases = [
      [".windsurf/hooks.json", windsurfHook],
      [".devin/hooks.json", windsurfHook],
      [".devin/hooks.v1.json", windsurfHook],
      [".devin/config.local.json", claudeHook],
      [".claude/settings.json", claudeHook],
      [".claude/settings.local.json", claudeHook],
      [".devin/hooks.json", psField],
    ];
    for (const [rel, content] of cases) {
      const { root } = makeConsumer();
      writeAt(root, rel, content);
      const cfgPath = configAt(root);
      const r = runInstaller(root, { apply: true });
      assert.notEqual(r.code, 0, `${rel}: ${r.combined}`);
      assert.ok(
        r.combined.includes(rel.replace(/\//g, "\\")) || r.combined.includes(rel),
        `${rel}: ${r.combined}`,
      );
      assert.equal(existsSync(cfgPath), false, rel);
    }
  });

  it("detects cheaper read hooks inside the primary config", () => {
    const { root } = makeConsumer();
    const content = JSON.stringify({
      hooks: {
        PreToolUse: [
          {
            matcher: "Read",
            hooks: [{ type: "command", command: "cheaper hook pre-tool-use --target pretooluse" }],
          },
        ],
      },
    });
    const cfg = writeAt(root, ".devin/config.json", content);
    const r = runInstaller(root, { apply: true });
    assert.notEqual(r.code, 0, r.combined);
    assert.equal(readBytes(cfg).toString("utf8"), content);
  });

  it("accepts shell-only legacy pre-run-command hooks", () => {
    const { root } = makeConsumer();
    writeAt(
      root,
      ".windsurf/hooks.json",
      JSON.stringify({
        hooks: {
          pre_run_command: [{ command: "sh ./cheaper.sh hook pre-run-command --target windsurf" }],
        },
      }),
    );
    const r = runInstaller(root, { apply: true });
    assert.equal(r.code, 0, r.combined);
    const updated = JSON.parse(readFileSync(configAt(root), "utf8"));
    assert.equal(updated.hooks.PreToolUse[0].hooks[0].command, CANON_COMMAND);
  });

  it("rejects unparseable known config files without writes", () => {
    const { root } = makeConsumer();
    writeAt(root, ".windsurf/hooks.json", "{ broken");
    const r = runInstaller(root, { apply: true });
    assert.notEqual(r.code, 0, r.combined);
    assert.equal(existsSync(configAt(root)), false);
  });

  it("rejects reparse point components on the config path", () => {
    const { root } = makeConsumer();
    const real = path.join(root, "realdevin");
    mkdirSync(real, { recursive: true });
    writeFileSync(path.join(real, "config.json"), '{"hooks":{}}');
    symlinkSync(real, path.join(root, ".devin"), "junction");
    const r = runInstaller(root, { apply: true });
    assert.notEqual(r.code, 0, r.combined);
  });

  it("rejects reparse point ancestors of the project root", () => {
    const { root } = makeConsumer();
    const real = path.join(root, "real");
    mkdirSync(path.join(real, "consumer"), { recursive: true });
    symlinkSync(real, path.join(root, "junc"), "junction");
    const r = runInstaller(path.join(root, "junc", "consumer"), { apply: true, binRoot: root });
    assert.notEqual(r.code, 0, r.combined);
    assert.equal(existsSync(path.join(real, "consumer", ".devin")), false);
  });

  it("rejects differently-cased hooks/PreToolUse keys without writes", () => {
    for (const content of [
      '{"Hooks":{"PreToolUse":[]}}',
      '{"HOOKS":{}}',
      '{"hooks":{"pretooluse":[]}}',
      '{"hooks":{"PREToolUse":[]}}',
    ]) {
      const { root } = makeConsumer();
      const cfg = writeAt(root, ".devin/config.json", content);
      const r = runInstaller(root, { apply: true });
      assert.notEqual(r.code, 0, `config ${content}: ${r.combined}`);
      assert.equal(readBytes(cfg).toString("utf8"), content, content);
    }
  });

  it("enforces the native PreToolUse group schema without writes", () => {
    const bad = [
      `{"hooks":{"PreToolUse":[{"matcher":${JSON.stringify(CANON_MATCHER)},"hooks":{"type":"command","command":${JSON.stringify(CANON_COMMAND)}}}]}}`,
      '{"hooks":{"PreToolUse":[{"matcher":".*","hooks":"x"}]}}',
      '{"hooks":{"PreToolUse":[{"matcher":".*"}]}}',
      '{"hooks":{"PreToolUse":[{"hooks":[7]}]}}',
      '{"hooks":{"PreToolUse":[{"hooks":[{"type":"weird","command":"x"}]}]}}',
      '{"hooks":{"PreToolUse":[{"hooks":[{"type":"command"}]}]}}',
      '{"hooks":{"PreToolUse":[{"hooks":[{"type":"command","command":7}]}]}}',
      '{"hooks":{"PreToolUse":[{"hooks":[{"type":"prompt"}]}]}}',
      '{"hooks":{"PreToolUse":[{"hooks":[{"type":"prompt","prompt":7}]}]}}',
      '{"hooks":{"PreToolUse":[{"matcher":7,"hooks":[{"type":"command","command":"x"}]}]}}',
    ];
    for (const content of bad) {
      const { root } = makeConsumer();
      const cfg = writeAt(root, ".devin/config.json", content);
      const r = runInstaller(root, { apply: true });
      assert.notEqual(r.code, 0, `config ${content}: ${r.combined}`);
      assert.equal(readBytes(cfg).toString("utf8"), content, content);
    }
  });

  it("preserves valid unrelated command/prompt groups with extra attributes", () => {
    const { root } = makeConsumer();
    const unrelated = [
      {
        matcher: ".*",
        hooks: [{ type: "prompt", prompt: "summarize", note: 1 }],
        extra: true,
      },
      { hooks: [{ type: "command", command: "echo other" }] },
    ];
    const cfg = writeAt(
      root,
      ".devin/config.json",
      JSON.stringify({ hooks: { PreToolUse: unrelated } }),
    );
    const r = runInstaller(root, { apply: true });
    assert.equal(r.code, 0, r.combined);
    const updated = JSON.parse(readFileSync(cfg, "utf8"));
    assert.deepEqual(updated.hooks.PreToolUse.slice(0, 2), unrelated);
    assert.equal(updated.hooks.PreToolUse[2].hooks[0].command, CANON_COMMAND);
  });

  it("detects concurrent config modification during the capability call", () => {
    const { root, bin } = makeConsumer({ withCli: false });
    const cfg = writeAt(root, ".devin/config.json", '{"keep":"yes","hooks":{}}');
    const external = '{"keep":"concurrent"}';
    writeFileSync(
      path.join(bin, "cheaper.cmd"),
      [
        "@echo off",
        `echo ${external}> "${configAt(root)}"`,
        `echo ${CAP_OK}`,
        "exit /b 0",
        "",
      ].join("\r\n"),
    );
    const r = runInstaller(root, { apply: true });
    assert.notEqual(r.code, 0, r.combined);
    const after = readBytes(cfg).toString("utf8");
    assert.ok(after.includes('"concurrent"'), after);
    assert.ok(!after.includes("PreToolUse"), after);
    assert.ok(!after.includes(CANON_COMMAND), after);
  });
});
