import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdir, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { cliRoot, makeIsolatedEnv } from "./helpers.js";

const repoRoot = path.join(cliRoot, "..", "..");
const hookScript = path.join(repoRoot, ".apm", "hooks", "pre-read-code.js");
const cheaperBin = path.join(cliRoot, "bin", "cheaper.js");
const bundledCli = path.join(repoRoot, ".apm", "hooks", "cli", "bin", "cheaper.mjs");

interface HookResult {
  status: number | null;
  stderr: string;
}

function runHook(
  stdin: string,
  opts: { cwd: string; env?: Record<string, string>; cliArg?: string },
): HookResult {
  const r = spawnSync(process.execPath, [hookScript, ...(opts.cliArg ? [opts.cliArg] : [])], {
    cwd: opts.cwd,
    env: opts.env,
    input: stdin,
    encoding: "utf8",
  });
  return { status: r.status, stderr: r.stderr };
}

const preRead = (filePath: string) =>
  JSON.stringify({ agent_action_name: "pre_read_code", tool_info: { file_path: filePath } });

describe("pre_read_code hook (pre-read-code.js)", () => {
  let dir: string;
  let ws: string;
  let env: Record<string, string>;

  beforeEach(async () => {
    ({ dir, ws, env } = await makeIsolatedEnv());
    env.CHEAPER_CLI = cheaperBin;
  });

  it("blocks via the bundled CLI passed as argv[2] (production command shape)", async () => {
    await writeFile(path.join(ws, "big.txt"), "lorem ipsum dolor sit amet ".repeat(4000), "utf8");
    // CHEAPER_CLI を未設定にして argv[2] の解決だけで動くことを確認
    const { CHEAPER_CLI: _dropped, ...envNoCli } = env;
    const r = runHook(preRead(path.join(ws, "big.txt")), {
      cwd: ws,
      env: envNoCli,
      cliArg: bundledCli,
    });
    expect(r.status).toBe(2);
    expect(r.stderr).toContain("bulk-read");
  });
  afterEach(async () => {
    await rm(dir, { recursive: true, force: true });
  });

  it("blocks a large file with exit 2 and delegation guidance on stderr", async () => {
    // 反復テキストで threshold を確実に超える file を作る
    await writeFile(path.join(ws, "big.txt"), "lorem ipsum dolor sit amet ".repeat(4000), "utf8");
    const r = runHook(preRead(path.join(ws, "big.txt")), { cwd: ws, env });
    expect(r.status).toBe(2);
    expect(r.stderr).toContain("tokens");
    expect(r.stderr).toContain("bulk-read");
  });

  it("emits a workspace-relative path and no shell command for absolute-path input", async () => {
    await mkdir(path.join(ws, "sub"), { recursive: true });
    await writeFile(
      path.join(ws, "sub", "big.txt"),
      "lorem ipsum dolor sit amet ".repeat(4000),
      "utf8",
    );
    // Windsurf shape: tool_info.file_path は absolute path
    const r = runHook(preRead(path.join(ws, "sub", "big.txt")), { cwd: ws, env });
    expect(r.status).toBe(2);
    expect(r.stderr).toContain("workspace-relative path: sub/big.txt");
    // guidance に実行可能な shell command / raw path interpolation が無いこと
    expect(r.stderr).not.toContain("echo ");
    expect(r.stderr).not.toContain(path.join(ws, "sub", "big.txt"));
    expect(r.stderr).not.toContain("$");
  });

  it("does not break on filenames containing quotes", async () => {
    const name = "it's a file.txt";
    await writeFile(path.join(ws, name), "lorem ipsum dolor sit amet ".repeat(4000), "utf8");
    const r = runHook(preRead(path.join(ws, name)), { cwd: ws, env });
    expect(r.status).toBe(2);
    expect(r.stderr).toContain(`workspace-relative path: ${name}`);
  });

  it("treats filenames starting with '..' as inside the workspace", async () => {
    // "..foo.txt" は `..` + path.sep ではないので outside 扱いしてはいけない
    await writeFile(path.join(ws, "..foo.txt"), "lorem ipsum dolor sit amet ".repeat(4000), "utf8");
    const r = runHook(preRead(path.join(ws, "..foo.txt")), { cwd: ws, env });
    expect(r.status).toBe(2);
    expect(r.stderr).toContain("workspace-relative path: ..foo.txt");
  });

  it("fails open for paths outside the workspace", async () => {
    const outside = path.join(dir, "outside.txt");
    await writeFile(outside, "lorem ipsum dolor sit amet ".repeat(4000), "utf8");
    expect(runHook(preRead(outside), { cwd: ws, env }).status).toBe(0);
  });

  it("allows a small file with exit 0", async () => {
    await writeFile(path.join(ws, "small.txt"), "hello\n", "utf8");
    const r = runHook(preRead(path.join(ws, "small.txt")), { cwd: ws, env });
    expect(r.status).toBe(0);
  });

  it("blocks a small file when the threshold is lowered via env", async () => {
    await writeFile(path.join(ws, "small.txt"), "hello world this is a file\n", "utf8");
    const r = runHook(preRead(path.join(ws, "small.txt")), {
      cwd: ws,
      env: { ...env, CHEAPER_READ_TOKEN_THRESHOLD: "1" },
    });
    expect(r.status).toBe(2);
  });

  it("blocks with exit 2 on a FILE_TOO_LARGE envelope (bytes over the cap)", async () => {
    // fast-path を通過させるため token threshold も byte cap も小さくする
    await writeFile(path.join(ws, "big.bin"), Buffer.alloc(64, 0x61));
    const r = runHook(preRead(path.join(ws, "big.bin")), {
      cwd: ws,
      env: {
        ...env,
        CHEAPER_READ_TOKEN_THRESHOLD: "8",
        CHEAPER_COUNT_MAX_BYTES: "16",
      },
    });
    expect(r.status).toBe(2);
    expect(r.stderr).toContain("64 bytes");
    expect(r.stderr).toContain("16-byte measurement cap");
    expect(r.stderr).toContain("bulk-read");
    expect(r.stderr).toContain("workspace-relative path: big.bin");
  });

  it("skips the CLI spawn for a file whose bytes fit the threshold", async () => {
    await writeFile(path.join(ws, "small.txt"), "hello\n", "utf8");
    const marker = path.join(dir, "cli-spawned.marker");
    const fakeCli = path.join(dir, "fake-cli.js");
    await writeFile(
      fakeCli,
      'require("node:fs").writeFileSync(process.env.FAKE_CLI_MARKER, "spawned");\n',
      "utf8",
    );
    const r = runHook(preRead(path.join(ws, "small.txt")), {
      cwd: ws,
      env: { ...env, CHEAPER_CLI: fakeCli, FAKE_CLI_MARKER: marker },
    });
    expect(r.status).toBe(0);
    expect(existsSync(marker)).toBe(false);
  });

  it("fails open for missing files, directories, and non-JSON stdin", async () => {
    expect(runHook(preRead(path.join(ws, "missing.txt")), { cwd: ws, env }).status).toBe(0);
    expect(runHook(preRead(ws), { cwd: ws, env }).status).toBe(0);
    expect(runHook("not json", { cwd: ws, env }).status).toBe(0);
  });

  it("fails open for other hook events and when the CLI is unavailable", async () => {
    await writeFile(path.join(ws, "big.txt"), "x ".repeat(20000), "utf8");
    const other = JSON.stringify({
      agent_action_name: "post_read_code",
      tool_info: { file_path: path.join(ws, "big.txt") },
    });
    expect(runHook(other, { cwd: ws, env }).status).toBe(0);
    const noCli = { ...env, CHEAPER_CLI: path.join(dir, "no-such-cli.js") };
    expect(runHook(preRead(path.join(ws, "big.txt")), { cwd: ws, env: noCli }).status).toBe(0);
  });
});
