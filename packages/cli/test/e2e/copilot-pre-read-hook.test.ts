import { spawnSync } from "node:child_process";
import { mkdir, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { apmPackageDir } from "./apm-fixture.js";
import { cliRoot, makeIsolatedEnv } from "./helpers.js";

const cheaperBin = path.join(cliRoot, "bin", "cheaper.js");
const bundledCli = path.join(apmPackageDir, ".apm", "hooks", "cli", "bin", "cheaper.mjs");

interface HookResult {
  status: number | null;
  stdout: string;
  stderr: string;
}

function runHook(
  stdin: string,
  opts: { cwd: string; env?: Record<string, string>; bin?: string },
): HookResult {
  const r = spawnSync(
    process.execPath,
    [opts.bin ?? cheaperBin, "hook", "pre-tool-use", "--target", "copilot"],
    {
      cwd: opts.cwd,
      env: opts.env,
      input: stdin,
      encoding: "utf8",
    },
  );
  return { status: r.status, stdout: r.stdout, stderr: r.stderr };
}

// Copilot preToolUse payload. toolArgs には object / JSON 文字列の
// どちらも来うる（公式 doc の揺れ — DISC-0009）。
const preToolUse = (filePath: string) =>
  JSON.stringify({ toolName: "view", toolArgs: { path: filePath } });
const preToolUseStringArgs = (filePath: string) =>
  JSON.stringify({ toolName: "view", toolArgs: JSON.stringify({ path: filePath }) });

// deny JSON を parse して契約どおりか検査する
function expectDeny(r: HookResult, relPath: string) {
  expect(r.status).toBe(0);
  const out = JSON.parse(r.stdout.trim());
  expect(out.permissionDecision).toBe("deny");
  expect(out.permissionDecisionReason).toContain("bulk-read");
  expect(out.permissionDecisionReason).toContain(`workspace-relative path: ${relPath}`);
}

describe("copilot preToolUse hook (cheaper hook --target copilot)", () => {
  let dir: string;
  let ws: string;
  let env: Record<string, string>;

  beforeEach(async () => {
    ({ dir, ws, env } = await makeIsolatedEnv());
  });

  it("denies a large file via object toolArgs with stdout deny JSON and exit 0", async () => {
    await writeFile(path.join(ws, "big.txt"), "lorem ipsum dolor sit amet ".repeat(4000), "utf8");
    const r = runHook(preToolUse(path.join(ws, "big.txt")), { cwd: ws, env });
    expectDeny(r, "big.txt");
    expect(r.stdout).not.toContain("echo ");
  });

  it("denies a large file via JSON-string toolArgs identically", async () => {
    await writeFile(path.join(ws, "big.txt"), "lorem ipsum dolor sit amet ".repeat(4000), "utf8");
    const r = runHook(preToolUseStringArgs(path.join(ws, "big.txt")), { cwd: ws, env });
    expectDeny(r, "big.txt");
  });

  it("denies via the bundled CLI (production command shape)", async () => {
    await writeFile(path.join(ws, "big.txt"), "lorem ipsum dolor sit amet ".repeat(4000), "utf8");
    const r = runHook(preToolUse(path.join(ws, "big.txt")), { cwd: ws, env, bin: bundledCli });
    expectDeny(r, "big.txt");
  });

  it("emits a workspace-relative path and no shell command for absolute-path input", async () => {
    await mkdir(path.join(ws, "sub"), { recursive: true });
    await writeFile(
      path.join(ws, "sub", "big.txt"),
      "lorem ipsum dolor sit amet ".repeat(4000),
      "utf8",
    );
    const r = runHook(preToolUse(path.join(ws, "sub", "big.txt")), { cwd: ws, env });
    expectDeny(r, "sub/big.txt");
    expect(r.stdout).not.toContain(path.join(ws, "sub", "big.txt"));
    expect(r.stdout).not.toContain("$");
  });

  it("denies with exit 0 on FILE_TOO_LARGE (bytes over the cap)", async () => {
    await writeFile(path.join(ws, "big.bin"), Buffer.alloc(64, 0x61));
    const r = runHook(preToolUse(path.join(ws, "big.bin")), {
      cwd: ws,
      env: {
        ...env,
        CHEAPER_READ_TOKEN_THRESHOLD: "8",
        CHEAPER_COUNT_MAX_BYTES: "16",
      },
    });
    expect(r.status).toBe(0);
    const out = JSON.parse(r.stdout.trim());
    expect(out.permissionDecision).toBe("deny");
    expect(out.permissionDecisionReason).toContain("64 bytes");
    expect(out.permissionDecisionReason).toContain("16-byte measurement cap");
    expect(out.permissionDecisionReason).toContain("bulk-read");
  });

  it("allows a small file silently (exit 0, no deny JSON)", async () => {
    await writeFile(path.join(ws, "small.txt"), "hello\n", "utf8");
    const r = runHook(preToolUse(path.join(ws, "small.txt")), { cwd: ws, env });
    expect(r.status).toBe(0);
    expect(r.stdout).not.toContain("permissionDecision");
  });

  it("denies a small file when the threshold is lowered via env", async () => {
    await writeFile(path.join(ws, "small.txt"), "hello world this is a file\n", "utf8");
    const r = runHook(preToolUse(path.join(ws, "small.txt")), {
      cwd: ws,
      env: { ...env, CHEAPER_READ_TOKEN_THRESHOLD: "1" },
    });
    expectDeny(r, "small.txt");
  });

  it("never exits nonzero: malformed payloads and every fail-open path exit 0", async () => {
    await writeFile(path.join(ws, "big.txt"), "x ".repeat(20000), "utf8");
    const bigPath = path.join(ws, "big.txt");
    const cases: Array<[string, string]> = [
      ["not json", "non-JSON stdin"],
      [JSON.stringify({ toolName: "write", toolArgs: { path: bigPath } }), "non-view toolName"],
      [
        JSON.stringify({ toolName: "view", toolArgs: "{broken json" }),
        "unparseable toolArgs string",
      ],
      [JSON.stringify({ toolName: "view" }), "missing toolArgs"],
      [JSON.stringify({ toolName: "view", toolArgs: { other: 1 } }), "no path key"],
      [JSON.stringify({ toolName: "view", toolArgs: { path: 42 } }), "non-string path value"],
      [preToolUse(path.join(dir, "outside.txt")), "path outside workspace"],
      [preToolUse(path.join(ws, "missing.txt")), "missing file"],
      [preToolUse(ws), "directory path"],
    ];
    for (const [stdin, label] of cases) {
      const r = runHook(stdin, { cwd: ws, env });
      expect(r.status, `case: ${label}`).toBe(0);
    }
  }, 120_000);

  it("routes shell toolNames through the canonical shell seam (exit 0, silent)", async () => {
    // `bash` / `powershell` / `shell` は canonical shell request へ
    // normalize される。小さな file への read command は allow で
    // silent exit 0。
    await writeFile(path.join(ws, "big.txt"), "hello\n", "utf8");
    for (const toolName of ["bash", "powershell", "shell"]) {
      const r = runHook(JSON.stringify({ toolName, toolArgs: { command: "cat big.txt" } }), {
        cwd: ws,
        env,
      });
      expect(r.status).toBe(0);
      expect(r.stdout).not.toContain("permissionDecision");
    }
    // toolArgs の JSON 文字列揺れも shell で許容する
    const r = runHook(
      JSON.stringify({ toolName: "bash", toolArgs: JSON.stringify({ command: "ls" }) }),
      { cwd: ws, env },
    );
    expect(r.status).toBe(0);
  });

  it("answers a dual-schema allow for foreign payload shapes (cross-fire safety)", async () => {
    // claude/cursor 風の snake_case payload が copilot entry に
    // 誤発火した場合、cursor permission hook の「exit 0 + schema 不適合
    // は block」に備えて schema 適合の allow JSON を返す
    //（DESIGN-034rA79Uog7JOa7evw6rOL）。
    const r = runHook(JSON.stringify({ tool_name: "Shell", tool_input: { command: "ls" } }), {
      cwd: ws,
      env,
    });
    expect(r.status).toBe(0);
    const out = JSON.parse(r.stdout.trim());
    expect(out.permission).toBe("allow");
    expect(out.permissionDecision).toBe("allow");
  });

  it("fails open (exit 0) when policy loading fails", async () => {
    await writeFile(path.join(ws, "big.txt"), "x ".repeat(20000), "utf8");
    const configDir = path.join(dir, "broken-config");
    await mkdir(configDir, { recursive: true });
    await writeFile(path.join(configDir, "policy.yaml"), "::: not yaml :::[", "utf8");
    const r = runHook(preToolUse(path.join(ws, "big.txt")), {
      cwd: ws,
      env: { ...env, CHEAPER_CONFIG_DIR: configDir },
    });
    expect(r.status).toBe(0);
    expect(r.stdout).not.toContain("permissionDecision");
  });

  afterEach(async () => {
    await rm(dir, { recursive: true, force: true });
  });
});
