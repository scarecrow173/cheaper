import { spawnSync } from "node:child_process";
import { mkdir, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { apmPackageDir } from "./apm-fixture.js";
import { cliRoot, makeIsolatedEnv } from "./helpers.js";

// cross-agent routing adapter の contract test（TSPEC-034rAaJtM27Ibpy78mq0kP、
// TSPEC-034z50PbuzTYDWvQXYdvaG）。各 target が native stdin payload を
// canonical request へ normalize し、shared policy verdict を target
// native response へ変換することを `cheaper hook` の stdin 注入で検証する。

const cheaperBin = path.join(cliRoot, "bin", "cheaper.js");
const bundledCli = path.join(apmPackageDir, ".apm", "hooks", "cli", "bin", "cheaper.mjs");

type HookArgs = [event: string, target: string];

interface HookResult {
  status: number | null;
  stdout: string;
  stderr: string;
}

function runHook(
  [event, target]: HookArgs,
  stdin: string,
  opts: { cwd: string; env?: Record<string, string>; bin?: string },
): HookResult {
  const r = spawnSync(
    process.execPath,
    [opts.bin ?? cheaperBin, "hook", event, "--target", target],
    {
      cwd: opts.cwd,
      env: opts.env,
      input: stdin,
      encoding: "utf8",
    },
  );
  return { status: r.status, stdout: r.stdout, stderr: r.stderr };
}

const BIG_TEXT = "lorem ipsum dolor sit amet ".repeat(4000);

/** 破損した user policy を返す env（infrastructure failure の fail-open 検証用）。 */
async function corruptPolicyEnv(
  dir: string,
  env: Record<string, string>,
): Promise<Record<string, string>> {
  const configDir = path.join(dir, "broken-config");
  await mkdir(configDir, { recursive: true });
  await writeFile(path.join(configDir, "policy.yaml"), "::: not yaml :::[", "utf8");
  return { ...env, CHEAPER_CONFIG_DIR: configDir };
}

describe("PreToolUse adapter (cheaper hook --target pretooluse, claude + codex)", () => {
  let dir: string;
  let ws: string;
  let env: Record<string, string>;

  const run = (stdin: string, e = env) =>
    runHook(["pre-tool-use", "pretooluse"], stdin, { cwd: ws, env: e });
  const readPayload = (filePath: string) =>
    JSON.stringify({ tool_name: "Read", tool_input: { file_path: filePath } });

  beforeEach(async () => {
    ({ dir, ws, env } = await makeIsolatedEnv());
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

  it("denies via the bundled CLI (production command shape)", async () => {
    await writeFile(path.join(ws, "big.txt"), BIG_TEXT, "utf8");
    const r = runHook(["pre-tool-use", "pretooluse"], readPayload(path.join(ws, "big.txt")), {
      cwd: ws,
      env,
      bin: bundledCli,
    });
    expect(r.status).toBe(0);
    expect(r.stdout).toContain('"permissionDecision":"deny"');
  });

  it("denies on FILE_TOO_LARGE (bytes over the cap)", async () => {
    await writeFile(path.join(ws, "big.bin"), Buffer.alloc(64, 0x61));
    const r = run(readPayload(path.join(ws, "big.bin")), {
      ...env,
      CHEAPER_READ_TOKEN_THRESHOLD: "8",
      CHEAPER_COUNT_MAX_BYTES: "16",
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
      const s = run(JSON.stringify({ tool_name: tool, tool_input: { command: "cat small.txt" } }));
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
  }, 120_000);

  it("fails open when policy loading fails", async () => {
    await writeFile(path.join(ws, "big.txt"), BIG_TEXT, "utf8");
    const r = run(readPayload(path.join(ws, "big.txt")), await corruptPolicyEnv(dir, env));
    expect(r.status).toBe(0);
    expect(r.stdout).not.toContain("permissionDecision");
  });
});

describe("Cursor adapter (cheaper hook --target cursor)", () => {
  let dir: string;
  let ws: string;
  let env: Record<string, string>;

  const run = (stdin: string, e = env) =>
    runHook(["before-read-file", "cursor"], stdin, { cwd: ws, env: e });
  const readPayload = (filePath: string) => JSON.stringify({ file_path: filePath, cwd: ws });

  beforeEach(async () => {
    ({ dir, ws, env } = await makeIsolatedEnv());
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

  it("denies via the bundled CLI (production command shape)", async () => {
    await writeFile(path.join(ws, "big.txt"), BIG_TEXT, "utf8");
    const r = runHook(["before-read-file", "cursor"], readPayload(path.join(ws, "big.txt")), {
      cwd: ws,
      env,
      bin: bundledCli,
    });
    expect(r.status).toBe(0);
    expect(JSON.parse(r.stdout.trim()).permission).toBe("deny");
  });

  it("answers {permission:allow} for shell payloads and small files", async () => {
    await writeFile(path.join(ws, "small.txt"), "hello\n", "utf8");
    const shell = runHook(
      ["before-shell-execution", "cursor"],
      JSON.stringify({ command: "cat small.txt", cwd: ws }),
      { cwd: ws, env },
    );
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
    const r = run(readPayload(path.join(ws, "big.txt")), await corruptPolicyEnv(dir, env));
    expect(r.status).toBe(0);
    expect(JSON.parse(r.stdout.trim()).permission).toBe("allow");
  }, 120_000);
});

describe("Windsurf pre_run_command adapter (cheaper hook --target windsurf)", () => {
  let dir: string;
  let ws: string;
  let env: Record<string, string>;

  const run = (stdin: string) => runHook(["pre-run-command", "windsurf"], stdin, { cwd: ws, env });
  const cmdPayload = (commandLine: string) =>
    JSON.stringify({
      agent_action_name: "pre_run_command",
      tool_info: { command_line: commandLine },
    });

  beforeEach(async () => {
    ({ dir, ws, env } = await makeIsolatedEnv());
  });
  afterEach(async () => {
    await rm(dir, { recursive: true, force: true });
  });

  it("denies a large `cat` via exit 2 + stderr guidance", async () => {
    await writeFile(path.join(ws, "big.txt"), BIG_TEXT, "utf8");
    const r = run(cmdPayload("cat big.txt"));
    expect(r.status).toBe(2);
    expect(r.stderr).toContain("cat big.txt");
    expect(r.stderr).toContain("bulk-read");
    expect(r.stderr).toContain("workspace-relative path: big.txt");
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

// shell 経由 large-read block の contract test（issue #42、
// TSPEC-034rBrs4K5W7pe0ixjegZk）。canonical shell request が
// shared policy の read candidate 抽出を通り、file-read と同一の
// verdict を返すことを stdin 注入で固定する。
describe("shell read blocking policy (canonical shell request)", () => {
  let dir: string;
  let ws: string;
  let env: Record<string, string>;

  // Windsurf adapter を canonical seam の入口として使う
  // （deny → exit 2 + stderr、allow → exit 0 silent）。
  const run = (command: string, e = env) =>
    runHook(
      ["pre-run-command", "windsurf"],
      JSON.stringify({
        agent_action_name: "pre_run_command",
        tool_info: { command_line: command },
      }),
      { cwd: ws, env: e },
    );
  const denied = (command: string, e = env) => {
    const r = run(command, e);
    expect(r.status, `command: ${command}`).toBe(2);
    return r;
  };
  const allowed = (command: string, e = env) => {
    const r = run(command, e);
    expect(r.status, `command: ${command}`).toBe(0);
    return r;
  };

  beforeEach(async () => {
    ({ dir, ws, env } = await makeIsolatedEnv());
    await writeFile(path.join(ws, "big.txt"), BIG_TEXT, "utf8");
    await writeFile(path.join(ws, "small.txt"), "hello\n", "utf8");
  });
  afterEach(async () => {
    await rm(dir, { recursive: true, force: true });
  });

  // deny は case ごとに計測を伴うため command 単位で分割する。
  it("denies `cat` on a file over the threshold", () => {
    denied("cat big.txt");
  });

  it("denies `head` on a file over the threshold", () => {
    denied("head -n 200 big.txt");
  });

  it("denies `tail` on a file over the threshold", () => {
    denied("tail -100 big.txt");
    denied("tail -n +5 big.txt");
  });

  it("denies `less` on a file over the threshold", () => {
    denied("less big.txt");
  });

  it("denies `more` on a file over the threshold", () => {
    denied("more big.txt");
  });

  it("allows a small file and stdin-only or unrelated commands", () => {
    for (const command of [
      "cat small.txt",
      "cat",
      "ls -la",
      "grep foo big.txt", // grep は対象外 command（read ではない扱い）
      "echo hi",
    ]) {
      allowed(command);
    }
  });

  it("denies when any one of multiple operands is large", () => {
    const r = denied("cat small.txt big.txt");
    expect(r.stderr).toContain("workspace-relative path: big.txt");
  });

  it("does not mistake options for paths", () => {
    // `-n`/`200`/`-100` を file として解釈しない。big.txt のみが判定対象。
    allowed("head -n 200 small.txt");
    allowed("tail -100 small.txt");
    denied("head --lines=200 big.txt");
  });

  it("handles quoted paths containing spaces", async () => {
    await mkdir(path.join(ws, "path with spaces"), { recursive: true });
    await writeFile(path.join(ws, "path with spaces", "big.txt"), BIG_TEXT, "utf8");
    denied('cat "path with spaces/big.txt"');
    denied("cat 'path with spaces/big.txt'");
  });

  it("evaluates each segment of piped and chained commands", () => {
    denied("cat big.txt | grep foo");
    allowed("grep foo small.txt | head"); // 後段 head は operand なし
    allowed("cd src && cat big.txt"); // cwd 遷移の後段は ambiguous → fail-open
    denied("cat big.txt && cd src"); // cwd 遷移より前の segment は従来通り判定
    denied("cat small.txt ; cat big.txt");
  });

  // cwd 遷移 command の後段にある read candidate は、実際に読まれる
  // file を hook cwd 基準で特定できないため ambiguous → fail-open と
  // する（issue #42 review 指摘）。root と src/ に同名でサイズの異なる
  // fixture を置き、誤 block・誤 pass の両方向を固定する。
  it("fails open for reads after a cwd-changing command (both fixture directions)", async () => {
    const src = path.join(ws, "src");
    await mkdir(src);

    // false-positive 側: hook cwd の big.txt は large だが実際に読まれる
    // src/big.txt は small。cwd を追跡しないまま block すると誤 block
    // になるため fail-open でなければならない。
    await writeFile(path.join(src, "big.txt"), "hello\n");
    for (const command of [
      "cd src && cat big.txt",
      "cd src; cat big.txt",
      "pushd src && cat big.txt",
      "Set-Location src && cat big.txt",
    ]) {
      allowed(command);
    }

    // false-negative 側: hook cwd に big.txt が無く実体 src/big.txt が
    // large でも、ambiguous な後段は見逃す（許容済みの既知制限）。
    // path が直接特定できる `cat src/big.txt` は変わらず deny。
    await rm(path.join(ws, "big.txt"));
    await writeFile(path.join(src, "big.txt"), BIG_TEXT);
    allowed("cd src && cat big.txt");
    denied("cat src/big.txt");
  }, 120_000);

  it("fails open for ambiguous or unsupported shell syntax", () => {
    for (const command of [
      "cat $(ls)",
      "cat *.txt",
      "cat ~/secret.txt",
      "cat < big.txt", // redirect は segment を解釈不能にする
      'cat "unclosed',
      "cat a.txt > out.txt",
    ]) {
      allowed(command);
    }
  }, 120_000);

  it("fails open for missing files, directories, and workspace-outside paths", () => {
    allowed("cat missing.txt");
    allowed(`cat ${JSON.stringify(ws)}`); // directory
    allowed(`cat ${path.join(dir, "outside.txt").split(path.sep).join("/")}`);
  });

  it("fails open when policy loading fails", async () => {
    allowed("cat big.txt", await corruptPolicyEnv(dir, env));
  });

  it("denies on FILE_TOO_LARGE (fail-closed)", () => {
    // byte fast-path を通過するため file size は threshold を超える必要
    // がある。big.txt (>8192B) に小さな byte cap を掛ける。
    const r = run("cat big.txt", { ...env, CHEAPER_COUNT_MAX_BYTES: "1024" });
    expect(r.status).toBe(2);
    expect(r.stderr).toContain("bytes");
    expect(r.stderr).toContain("bulk-read");
  });

  it("honors CHEAPER_READ_TOKEN_THRESHOLD on shell reads", async () => {
    // token 数が override 閾値を超える小さな fixture で固定する
    // （byte fast-path は file size > threshold を要求する点に注意）。
    await writeFile(path.join(ws, "medium.txt"), "one two three four five six seven eight\n");
    denied("cat medium.txt", { ...env, CHEAPER_READ_TOKEN_THRESHOLD: "4" });
    allowed("cat medium.txt", { ...env, CHEAPER_READ_TOKEN_THRESHOLD: "100" });
  });

  it("returns the same verdict for file-read and shell-read on the same file", () => {
    // 通常 read と shell-read が同一判定を共有する regression。
    const shellDeny = run("cat big.txt");
    const readDeny = runHook(
      ["pre-tool-use", "pretooluse"],
      JSON.stringify({ tool_name: "Read", tool_input: { file_path: "big.txt" } }),
      { cwd: ws, env },
    );
    expect(shellDeny.status).toBe(2);
    expect(readDeny.status).toBe(0);
    expect(readDeny.stdout).toContain('"permissionDecision":"deny"');
    const shellAllow = run("cat small.txt");
    const readAllow = runHook(
      ["pre-tool-use", "pretooluse"],
      JSON.stringify({ tool_name: "Read", tool_input: { file_path: "small.txt" } }),
      { cwd: ws, env },
    );
    expect(shellAllow.status).toBe(0);
    expect(readAllow.stdout).not.toContain('"permissionDecision":"deny"');
  });
});

// 各 target adapter が canonical shell request を native deny 表現へ
// 変換することの確認（TSPEC-034z50PbuzTYDWvQXYdvaG）。
describe("shell read blocking across target adapters", () => {
  let dir: string;
  let ws: string;
  let env: Record<string, string>;

  beforeEach(async () => {
    ({ dir, ws, env } = await makeIsolatedEnv());
    await writeFile(path.join(ws, "big.txt"), BIG_TEXT, "utf8");
    await writeFile(path.join(ws, "small.txt"), "hello\n", "utf8");
  });
  afterEach(async () => {
    await rm(dir, { recursive: true, force: true });
  });

  it("pretooluse adapter (claude/codex) denies Bash/PowerShell large reads", () => {
    for (const tool of ["Bash", "PowerShell"]) {
      const r = runHook(
        ["pre-tool-use", "pretooluse"],
        JSON.stringify({ tool_name: tool, tool_input: { command: "cat big.txt" } }),
        { cwd: ws, env },
      );
      expect(r.status).toBe(0);
      const out = JSON.parse(r.stdout.trim());
      expect(out.hookSpecificOutput.permissionDecision).toBe("deny");
      expect(out.hookSpecificOutput.permissionDecisionReason).toContain("bulk-read");
    }
    const small = runHook(
      ["pre-tool-use", "pretooluse"],
      JSON.stringify({ tool_name: "Bash", tool_input: { command: "cat small.txt" } }),
      { cwd: ws, env },
    );
    expect(small.status).toBe(0);
    expect(small.stdout).not.toContain("permissionDecision");
  });

  it("cursor adapter denies beforeShellExecution large reads", () => {
    const r = runHook(
      ["before-shell-execution", "cursor"],
      JSON.stringify({ command: "cat big.txt", cwd: ws }),
      { cwd: ws, env },
    );
    expect(r.status).toBe(0);
    const out = JSON.parse(r.stdout.trim());
    expect(out.permission).toBe("deny");
    expect(out.agent_message).toContain("bulk-read");
    const small = runHook(
      ["before-shell-execution", "cursor"],
      JSON.stringify({ command: "cat small.txt", cwd: ws }),
      { cwd: ws, env },
    );
    expect(JSON.parse(small.stdout.trim()).permission).toBe("allow");
  });

  it("copilot adapter denies shell tool large reads with exit 0", () => {
    const r = runHook(
      ["pre-tool-use", "copilot"],
      JSON.stringify({ toolName: "bash", toolArgs: { command: "cat big.txt" } }),
      { cwd: ws, env },
    );
    expect(r.status).toBe(0);
    const out = JSON.parse(r.stdout.trim());
    expect(out.permissionDecision).toBe("deny");
    const small = runHook(
      ["pre-tool-use", "copilot"],
      JSON.stringify({ toolName: "bash", toolArgs: { command: "cat small.txt" } }),
      { cwd: ws, env },
    );
    expect(small.status).toBe(0);
  });
});
