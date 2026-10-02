// `cheaper hook <event> --target <agent>` の contract test
// （TSPEC-034z50PbuzTYDWvQXYdvaG）。JS adapter との parity、
// target 別出力 contract、catch-all fail-open、usage 非記録を
// bin/cheaper.js の spawn で検証する。

import { execFileSync, spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdir, rm, symlink, writeFile } from "node:fs/promises";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { apmPackageDir } from "../e2e/apm-fixture.js";
import { cliRoot, makeIsolatedEnv } from "../e2e/helpers.js";

const cheaperBin = path.join(cliRoot, "bin", "cheaper.js");
const bundledCli = path.join(apmPackageDir, ".apm", "hooks", "cli", "bin", "cheaper.mjs");

const BIG_TEXT = "lorem ipsum dolor sit amet ".repeat(4000);

interface HookResult {
  status: number | null;
  stdout: string;
  stderr: string;
}

function runHook(
  args: string[],
  stdin: string,
  opts: { cwd: string; env?: Record<string, string>; bin?: string },
): HookResult {
  const r = spawnSync(process.execPath, [opts.bin ?? cheaperBin, "hook", ...args], {
    cwd: opts.cwd,
    env: opts.env,
    input: stdin,
    encoding: "utf8",
  });
  return { status: r.status, stdout: r.stdout, stderr: r.stderr };
}

const windsurfRead = (filePath: string) =>
  JSON.stringify({ agent_action_name: "pre_read_code", tool_info: { file_path: filePath } });

describe("cheaper hook contract", () => {
  let dir: string;
  let ws: string;
  let env: Record<string, string>;

  beforeEach(async () => {
    ({ dir, ws, env } = await makeIsolatedEnv());
  });
  afterEach(async () => {
    await rm(dir, { recursive: true, force: true });
  });

  describe("command path", () => {
    it("exits 0 silently for missing args, unknown target, unknown event, and extra flags", () => {
      for (const args of [
        [],
        ["pre-read-code"],
        ["pre-read-code", "--target", "unknown"],
        ["bogus-event", "--target", "windsurf"],
        ["pre-read-code", "--target", "windsurf", "--bogus-flag"],
      ]) {
        const r = runHook(args, "{}", { cwd: ws, env });
        expect(r.status, `args: ${args.join(" ")}`).toBe(0);
      }
    }, 120_000);

    it("exits 0 with a schema-valid allow for cursor even with missing args", () => {
      const r = runHook(["before-read-file", "--target", "cursor"], "not json", {
        cwd: ws,
        env,
      });
      expect(r.status).toBe(0);
      expect(JSON.parse(r.stdout.trim()).permission).toBe("allow");
    });

    it("rejects --target on non-hook commands", () => {
      const r = spawnSync(
        process.execPath,
        [cheaperBin, "count-tokens", "x", "--target", "windsurf"],
        { cwd: ws, env, encoding: "utf8" },
      );
      expect(r.status).not.toBe(0);
    });

    it("does not write usage.jsonl for hook invocations (including denies)", async () => {
      await writeFile(path.join(ws, "big.txt"), BIG_TEXT, "utf8");
      const configDir = path.join(dir, "cheaper-config");
      const r = runHook(["pre-read-code", "--target", "windsurf"], windsurfRead("big.txt"), {
        cwd: ws,
        env: { ...env, CHEAPER_CONFIG_DIR: configDir },
      });
      expect(r.status).toBe(2);
      expect(existsSync(path.join(configDir, "usage.jsonl"))).toBe(false);
    });

    it("fails open when the user policy is corrupt", async () => {
      await writeFile(path.join(ws, "big.txt"), BIG_TEXT, "utf8");
      const configDir = path.join(dir, "cheaper-config");
      await mkdir(configDir, { recursive: true });
      await writeFile(path.join(configDir, "policy.yaml"), "::: not yaml :::[", "utf8");
      const r = runHook(["pre-read-code", "--target", "windsurf"], windsurfRead("big.txt"), {
        cwd: ws,
        env: { ...env, CHEAPER_CONFIG_DIR: configDir },
      });
      expect(r.status).toBe(0);
    });

    it("reaches the same deny through the bundled cheaper.mjs", async () => {
      await writeFile(path.join(ws, "big.txt"), BIG_TEXT, "utf8");
      const r = runHook(["pre-read-code", "--target", "windsurf"], windsurfRead("big.txt"), {
        cwd: ws,
        env,
        bin: bundledCli,
      });
      expect(r.status).toBe(2);
      expect(r.stderr).toContain("bulk-read");
    });

    it("fails open under a symlinked cwd (parity with the JS adapters)", async () => {
      // raw cwd で resolve した path を canonical root で数えるため、
      // realpath と一致しない cwd では WorkspaceEscapeError → allow。
      // この quirk は JS adapter の挙動として維持する。
      const link = path.join(dir, "ws-link");
      await symlink(ws, link, process.platform === "win32" ? "junction" : "dir");
      await writeFile(path.join(ws, "big.txt"), BIG_TEXT, "utf8");
      const r = runHook(["pre-read-code", "--target", "windsurf"], windsurfRead("big.txt"), {
        cwd: link,
        env,
      });
      expect(r.status).toBe(0);
    });
  });

  describe("windsurf", () => {
    it("denies a large read via stderr + exit 2, allows small silently", async () => {
      await writeFile(path.join(ws, "big.txt"), BIG_TEXT, "utf8");
      await writeFile(path.join(ws, "small.txt"), "hello\n", "utf8");
      const deny = runHook(["pre-read-code", "--target", "windsurf"], windsurfRead("big.txt"), {
        cwd: ws,
        env,
      });
      expect(deny.status).toBe(2);
      expect(deny.stderr).toContain("tokens");
      expect(deny.stderr).toContain("bulk-read");
      const allow = runHook(["pre-read-code", "--target", "windsurf"], windsurfRead("small.txt"), {
        cwd: ws,
        env,
      });
      expect(allow.status).toBe(0);
      expect(allow.stdout).toBe("");
      expect(allow.stderr).toBe("");
    });

    it("exits 0 for byte-fast-path-sized files, wrong events, and missing fields", async () => {
      await writeFile(path.join(ws, "exact.bin"), Buffer.alloc(8192, 0x61));
      const cases = [
        windsurfRead("exact.bin"), // 8192 bytes <= threshold → fast path
        windsurfRead("missing.txt"),
        windsurfRead(ws), // directory
        "not json",
        "{}",
        JSON.stringify({ agent_action_name: "post_read_code", tool_info: {} }),
        JSON.stringify({ agent_action_name: "pre_read_code", tool_info: {} }),
        JSON.stringify({ agent_action_name: "pre_read_code", tool_info: { file_path: 42 } }),
        windsurfRead(path.join(dir, "outside.txt")), // outside workspace
      ];
      for (const stdin of cases) {
        const r = runHook(["pre-read-code", "--target", "windsurf"], stdin, { cwd: ws, env });
        expect(r.status, `stdin: ${stdin.slice(0, 60)}`).toBe(0);
      }
    }, 120_000);

    it.runIf(process.platform !== "win32")(
      "fails open for a FIFO (non-regular file, no hang)",
      () => {
        execFileSync("mkfifo", [path.join(ws, "pipe.fifo")]);
        const r = runHook(["pre-read-code", "--target", "windsurf"], windsurfRead("pipe.fifo"), {
          cwd: ws,
          env,
        });
        expect(r.status).toBe(0);
      },
    );
  });

  describe("copilot", () => {
    it("emits permissionDecision deny JSON with exit 0 and never exits nonzero", async () => {
      await writeFile(path.join(ws, "big.txt"), BIG_TEXT, "utf8");
      const big = path.join(ws, "big.txt");
      const cases: Array<[string, string]> = [
        [JSON.stringify({ toolName: "view", toolArgs: { path: big } }), "deny-case"],
        ["not json", "malformed"],
        [JSON.stringify({ toolName: "view" }), "missing toolArgs"],
        [JSON.stringify({ toolName: "view", toolArgs: "{broken" }), "broken toolArgs"],
        [JSON.stringify({ toolName: "write", toolArgs: { path: big } }), "unrelated tool"],
        [JSON.stringify({ tool_name: "Read", tool_input: { file_path: big } }), "foreign shape"],
      ];
      for (const [stdin, label] of cases) {
        const r = runHook(["pre-tool-use", "--target", "copilot"], stdin, { cwd: ws, env });
        expect(r.status, `case: ${label}`).toBe(0);
      }
      const deny = runHook(
        ["pre-tool-use", "--target", "copilot"],
        JSON.stringify({ toolName: "view", toolArgs: { path: big } }),
        { cwd: ws, env },
      );
      const out = JSON.parse(deny.stdout.trim());
      expect(out.permissionDecision).toBe("deny");
      expect(out.permissionDecisionReason).toContain("bulk-read");
    });

    it("emits dual-schema allow for cross-agent payload shapes", async () => {
      const r = runHook(
        ["pre-tool-use", "--target", "copilot"],
        JSON.stringify({ tool_name: "Read", tool_input: { file_path: "x.txt" } }),
        { cwd: ws, env },
      );
      expect(r.status).toBe(0);
      const out = JSON.parse(r.stdout.trim());
      expect(out.permission).toBe("allow");
      expect(out.permissionDecision).toBe("allow");
    });
  });

  describe("pretooluse", () => {
    it("emits hookSpecificOutput deny JSON with exit 0", async () => {
      await writeFile(path.join(ws, "big.txt"), BIG_TEXT, "utf8");
      const r = runHook(
        ["pre-tool-use", "--target", "pretooluse"],
        JSON.stringify({ tool_name: "Bash", tool_input: { command: "cat big.txt" } }),
        { cwd: ws, env },
      );
      expect(r.status).toBe(0);
      const out = JSON.parse(r.stdout.trim());
      expect(out.hookSpecificOutput.hookEventName).toBe("PreToolUse");
      expect(out.hookSpecificOutput.permissionDecision).toBe("deny");
      expect(out.hookSpecificOutput.permissionDecisionReason).toContain("blocked shell command");
    });

    it("is silent exit 0 on every fail-open path", async () => {
      await writeFile(path.join(ws, "big.txt"), BIG_TEXT, "utf8");
      const cases = [
        "not json",
        "{}",
        JSON.stringify({ tool_name: "Write", tool_input: { file_path: "big.txt" } }),
        JSON.stringify({ tool_name: "Read" }),
        JSON.stringify({ tool_name: "Read", tool_input: {} }),
        JSON.stringify({ tool_name: "Read", tool_input: { file_path: "missing.txt" } }),
        JSON.stringify({ tool_name: "Read", tool_input: { file_path: "small.txt" } }),
      ];
      for (const stdin of cases) {
        const r = runHook(["pre-tool-use", "--target", "pretooluse"], stdin, { cwd: ws, env });
        expect(r.status, `stdin: ${stdin.slice(0, 60)}`).toBe(0);
        expect(r.stdout, `stdin: ${stdin.slice(0, 60)}`).toBe("");
      }
    }, 120_000);
  });

  describe("cursor", () => {
    it("always emits schema-valid permission JSON, never a bare exit", async () => {
      await writeFile(path.join(ws, "big.txt"), BIG_TEXT, "utf8");
      const cases: Array<[string, string, string]> = [
        ["before-read-file", JSON.stringify({ file_path: "big.txt", cwd: ws }), "deny"],
        ["before-shell-execution", JSON.stringify({ command: "cat big.txt", cwd: ws }), "deny"],
        ["before-read-file", "not json", "allow"],
        ["before-read-file", "{}", "allow"],
        ["before-read-file", JSON.stringify({ other: 1 }), "allow"],
        ["before-read-file", JSON.stringify({ file_path: "missing.txt", cwd: ws }), "allow"],
        [
          "before-read-file",
          JSON.stringify({ file_path: path.join(dir, "out.txt"), cwd: ws }),
          "allow",
        ],
      ];
      for (const [event, stdin, expected] of cases) {
        const r = runHook([event, "--target", "cursor"], stdin, { cwd: ws, env });
        expect(r.status, `event: ${event} stdin: ${stdin.slice(0, 40)}`).toBe(0);
        const out = JSON.parse(r.stdout.trim());
        expect(["allow", "deny", "ask"]).toContain(out.permission);
        expect(out.permission).toBe(expected);
      }
    }, 120_000);
  });

  describe("shared policy", () => {
    it("denies on FILE_TOO_LARGE (fail-closed) with byte cap wording", async () => {
      await writeFile(path.join(ws, "big.bin"), Buffer.alloc(64, 0x61));
      const r = runHook(["pre-read-code", "--target", "windsurf"], windsurfRead("big.bin"), {
        cwd: ws,
        env: { ...env, CHEAPER_READ_TOKEN_THRESHOLD: "8", CHEAPER_COUNT_MAX_BYTES: "16" },
      });
      expect(r.status).toBe(2);
      expect(r.stderr).toContain("64 bytes");
      expect(r.stderr).toContain("16-byte measurement cap");
      expect(r.stderr).toContain("bulk-read");
    });

    it("denies shell reads and fails open after cwd-changing commands", async () => {
      await writeFile(path.join(ws, "big.txt"), BIG_TEXT, "utf8");
      const payload = (command: string) =>
        JSON.stringify({
          agent_action_name: "pre_run_command",
          tool_info: { command_line: command },
        });
      const deny = runHook(["pre-run-command", "--target", "windsurf"], payload("cat big.txt"), {
        cwd: ws,
        env,
      });
      expect(deny.status).toBe(2);
      expect(deny.stderr).toContain("blocked shell command 'cat big.txt'.");
      const allow = runHook(
        ["pre-run-command", "--target", "windsurf"],
        payload("cd src && cat big.txt"),
        { cwd: ws, env },
      );
      expect(allow.status).toBe(0);
    });
  });
});
