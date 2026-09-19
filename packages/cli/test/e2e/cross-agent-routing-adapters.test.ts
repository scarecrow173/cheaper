import { spawnSync } from "node:child_process";
import { mkdir, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { apmPackageDir } from "./apm-fixture.js";
import { cliRoot, makeIsolatedEnv } from "./helpers.js";

// cross-agent routing adapter の contract test（TSPEC-034rAaJtM27Ibpy78mq0kP）。
// 各 adapter が native stdin payload を canonical request へ normalize し、
// shared policy verdict を target native response へ変換することを
// stdin 注入で検証する。

const hooksDir = path.join(apmPackageDir, ".apm", "hooks");
const pretooluseScript = path.join(hooksDir, "pre-read-code-pretooluse.js");
const cursorScript = path.join(hooksDir, "pre-read-code-cursor.js");
const windsurfShellScript = path.join(hooksDir, "pre-run-command.js");
const cheaperBin = path.join(cliRoot, "bin", "cheaper.js");
const bundledCli = path.join(hooksDir, "cli", "bin", "cheaper.mjs");

interface HookResult {
  status: number | null;
  stdout: string;
  stderr: string;
}

function runHook(
  script: string,
  stdin: string,
  opts: { cwd: string; env?: Record<string, string>; cliArg?: string },
): HookResult {
  const r = spawnSync(process.execPath, [script, ...(opts.cliArg ? [opts.cliArg] : [])], {
    cwd: opts.cwd,
    env: opts.env,
    input: stdin,
    encoding: "utf8",
  });
  return { status: r.status, stdout: r.stdout, stderr: r.stderr };
}

const BIG_TEXT = "lorem ipsum dolor sit amet ".repeat(4000);

describe("PreToolUse adapter (pre-read-code-pretooluse.js, claude + codex)", () => {
  let dir: string;
  let ws: string;
  let env: Record<string, string>;

  const run = (stdin: string, e = env) => runHook(pretooluseScript, stdin, { cwd: ws, env: e });
  const readPayload = (filePath: string) =>
    JSON.stringify({ tool_name: "Read", tool_input: { file_path: filePath } });

  beforeEach(async () => {
    ({ dir, ws, env } = await makeIsolatedEnv());
    env.CHEAPER_CLI = cheaperBin;
  });
  afterEach(async () => {
    await rm(dir, { recursive: true, force: true });
  });

  it("denies a large Read via hookSpecificOutput deny JSON with exit 0", async () => {
    await writeFile(path.join(ws, "big.txt"), BIG_TEXT, "utf8");
    const r = run(readPayload(path.join(ws, "big.txt")));
    expect(r.status).toBe(0);
    const out = JSON.parse(r.stdout.trim());
    expect(out.hookSpecificOutput.hookEventName).toBe("PreToolUse");
    expect(out.hookSpecificOutput.permissionDecision).toBe("deny");
    expect(out.hookSpecificOutput.permissionDecisionReason).toContain("bulk-read");
    expect(out.hookSpecificOutput.permissionDecisionReason).toContain(
      "workspace-relative path: big.txt",
    );
  });

  it("denies via the bundled CLI passed as argv[2] (production command shape)", async () => {
    await writeFile(path.join(ws, "big.txt"), BIG_TEXT, "utf8");
    const { CHEAPER_CLI: _dropped, ...envNoCli } = env;
    const r = runHook(pretooluseScript, readPayload(path.join(ws, "big.txt")), {
      cwd: ws,
      env: envNoCli,
      cliArg: bundledCli,
    });
    expect(r.status).toBe(0);
    expect(r.stdout).toContain('"permissionDecision":"deny"');
  });

  it("denies on a FILE_TOO_LARGE envelope (bytes over the cap)", async () => {
    await writeFile(path.join(ws, "big.bin"), Buffer.alloc(64, 0x61));
    const r = runHook(pretooluseScript, readPayload(path.join(ws, "big.bin")), {
      cwd: ws,
      env: { ...env, CHEAPER_READ_TOKEN_THRESHOLD: "8", CHEAPER_COUNT_MAX_BYTES: "16" },
    });
    expect(r.status).toBe(0);
    const out = JSON.parse(r.stdout.trim());
    expect(out.hookSpecificOutput.permissionDecision).toBe("deny");
    expect(out.hookSpecificOutput.permissionDecisionReason).toContain("64 bytes");
  });

  it("allows a small Read silently and Bash/PowerShell shell requests pass the seam", async () => {
    await writeFile(path.join(ws, "small.txt"), "hello\n", "utf8");
    const r = run(readPayload(path.join(ws, "small.txt")));
    expect(r.status).toBe(0);
    expect(r.stdout).not.toContain("permissionDecision");
    // shell request は canonical seam を通り allow（#42 の接続点）
    for (const tool of ["Bash", "PowerShell"]) {
      const s = run(JSON.stringify({ tool_name: tool, tool_input: { command: "cat big.txt" } }));
      expect(s.status).toBe(0);
      expect(s.stdout).not.toContain("permissionDecision");
    }
  });

  it("fails open for unrelated tools, malformed payloads, and missing fields", async () => {
    await writeFile(path.join(ws, "big.txt"), BIG_TEXT, "utf8");
    const big = path.join(ws, "big.txt");
    const cases: Array<[string, string]> = [
      ["not json", "non-JSON stdin"],
      [JSON.stringify({ tool_name: "Write", tool_input: { file_path: big } }), "unrelated tool"],
      [JSON.stringify({ tool_name: "Read" }), "missing tool_input"],
      [JSON.stringify({ tool_name: "Read", tool_input: {} }), "missing file_path"],
      [JSON.stringify({ tool_name: "Bash", tool_input: {} }), "missing command"],
      [readPayload(path.join(dir, "outside.txt")), "path outside workspace"],
      [readPayload(path.join(ws, "missing.txt")), "missing file"],
      [JSON.stringify({ tool_name: 42 }), "non-string tool_name"],
    ];
    for (const [stdin, label] of cases) {
      const r = run(stdin);
      expect(r.status, `case: ${label}`).toBe(0);
      expect(r.stdout, `case: ${label}`).not.toContain("permissionDecision");
    }
  });

  it("fails open when the CLI is unavailable", async () => {
    await writeFile(path.join(ws, "big.txt"), BIG_TEXT, "utf8");
    const r = run(readPayload(path.join(ws, "big.txt")), {
      ...env,
      CHEAPER_CLI: path.join(dir, "no-such-cli.js"),
    });
    expect(r.status).toBe(0);
    expect(r.stdout).not.toContain("permissionDecision");
  });
});

describe("Cursor adapter (pre-read-code-cursor.js)", () => {
  let dir: string;
  let ws: string;
  let env: Record<string, string>;

  const run = (stdin: string, e = env) => runHook(cursorScript, stdin, { cwd: ws, env: e });
  const readPayload = (filePath: string) => JSON.stringify({ file_path: filePath, cwd: ws });

  beforeEach(async () => {
    ({ dir, ws, env } = await makeIsolatedEnv());
    env.CHEAPER_CLI = cheaperBin;
  });
  afterEach(async () => {
    await rm(dir, { recursive: true, force: true });
  });

  it("denies a large beforeReadFile via {permission:deny} JSON with exit 0", async () => {
    await writeFile(path.join(ws, "big.txt"), BIG_TEXT, "utf8");
    const r = run(readPayload(path.join(ws, "big.txt")));
    expect(r.status).toBe(0);
    const out = JSON.parse(r.stdout.trim());
    expect(out.permission).toBe("deny");
    expect(out.user_message).toContain("bulk-read");
    expect(out.user_message).toContain("workspace-relative path: big.txt");
    expect(out.agent_message).toContain("bulk-read");
  });

  it("denies via the bundled CLI passed as argv[2] (production command shape)", async () => {
    await writeFile(path.join(ws, "big.txt"), BIG_TEXT, "utf8");
    const { CHEAPER_CLI: _dropped, ...envNoCli } = env;
    const r = runHook(cursorScript, readPayload(path.join(ws, "big.txt")), {
      cwd: ws,
      env: envNoCli,
      cliArg: bundledCli,
    });
    expect(r.status).toBe(0);
    expect(JSON.parse(r.stdout.trim()).permission).toBe("deny");
  });

  it("answers {permission:allow} for shell payloads and small files", async () => {
    await writeFile(path.join(ws, "small.txt"), "hello\n", "utf8");
    const shell = run(JSON.stringify({ command: "cat big.txt", cwd: ws }));
    expect(shell.status).toBe(0);
    expect(JSON.parse(shell.stdout.trim()).permission).toBe("allow");
    const small = run(readPayload(path.join(ws, "small.txt")));
    expect(small.status).toBe(0);
    expect(JSON.parse(small.stdout.trim()).permission).toBe("allow");
  });

  it("always emits a schema-valid permission JSON, never a bare exit 0", async () => {
    await writeFile(path.join(ws, "big.txt"), BIG_TEXT, "utf8");
    const cases: Array<[string, string]> = [
      ["not json", "non-JSON stdin"],
      ["{}", "empty object"],
      [JSON.stringify({ other: 1 }), "unknown payload"],
      [readPayload(path.join(dir, "outside.txt")), "path outside workspace"],
      [readPayload(path.join(ws, "missing.txt")), "missing file"],
      [readPayload(ws), "directory path"],
    ];
    for (const [stdin, label] of cases) {
      const r = run(stdin);
      expect(r.status, `case: ${label}`).toBe(0);
      const out = JSON.parse(r.stdout.trim());
      expect(["allow", "deny", "ask"], `case: ${label}`).toContain(out.permission);
      expect(out.permission, `case: ${label}`).toBe("allow");
    }
    const noCli = { ...env, CHEAPER_CLI: path.join(dir, "no-such-cli.js") };
    const r = run(readPayload(path.join(ws, "big.txt")), noCli);
    expect(r.status).toBe(0);
    expect(JSON.parse(r.stdout.trim()).permission).toBe("allow");
  });
});

describe("Windsurf pre_run_command adapter (pre-run-command.js)", () => {
  let dir: string;
  let ws: string;
  let env: Record<string, string>;

  const run = (stdin: string) => runHook(windsurfShellScript, stdin, { cwd: ws, env });
  const cmdPayload = (commandLine: string) =>
    JSON.stringify({
      agent_action_name: "pre_run_command",
      tool_info: { command_line: commandLine },
    });

  beforeEach(async () => {
    ({ dir, ws, env } = await makeIsolatedEnv());
    env.CHEAPER_CLI = cheaperBin;
  });
  afterEach(async () => {
    await rm(dir, { recursive: true, force: true });
  });

  it("passes shell invocations through the canonical seam with exit 0", async () => {
    const r = run(cmdPayload("cat huge-file.txt"));
    expect(r.status).toBe(0);
  });

  it("fails open for other events, missing fields, and malformed stdin", async () => {
    const cases = [
      "not json",
      JSON.stringify({ agent_action_name: "pre_read_code", tool_info: {} }),
      JSON.stringify({ agent_action_name: "pre_run_command" }),
      JSON.stringify({ agent_action_name: "pre_run_command", tool_info: { command_line: 1 } }),
    ];
    for (const stdin of cases) {
      const r = run(stdin);
      expect(r.status).toBe(0);
      expect(r.stdout).toBe("");
    }
  });
});
