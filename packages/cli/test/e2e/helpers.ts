import { spawn } from "node:child_process";
import { mkdir, mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { expect } from "vitest";

export const cliRoot = fileURLToPath(new URL("../..", import.meta.url));
const bin = path.join(cliRoot, "bin", "cheaper.js");

export interface CliResult {
  code: number | null;
  stdout: string;
  stderr: string;
}

export interface RunOptions {
  input?: string;
  env?: Record<string, string>;
  cwd?: string;
}

export function runCheaper(args: string[], opts: RunOptions = {}): Promise<CliResult> {
  return new Promise((resolve, reject) => {
    const child = spawn(process.execPath, [bin, ...args], {
      cwd: opts.cwd ?? cliRoot,
      env: opts.env ?? { ...process.env },
      stdio: ["pipe", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (d) => (stdout += d));
    child.stderr.on("data", (d) => (stderr += d));
    child.on("error", reject);
    child.on("close", (code) => resolve({ code, stdout, stderr }));
    if (opts.input !== undefined) child.stdin.write(opts.input);
    child.stdin.end();
  });
}

/** stdout が envelope 1 JSON + 末尾改行 1 つのみであること。 */
export function expectEnvelopeOnly(stdout: string): Record<string, unknown> {
  expect(stdout.endsWith("\n")).toBe(true);
  expect(stdout.endsWith("\n\n")).toBe(false);
  const parsed = JSON.parse(stdout) as Record<string, unknown>;
  expect(parsed.schemaVersion).toBe(1);
  return parsed;
}

/** 認証情報・user config を持たない隔離環境を tmpdir に作る。 */
export async function makeIsolatedEnv(): Promise<{
  dir: string;
  ws: string;
  env: Record<string, string>;
}> {
  const dir = await mkdtemp(path.join(tmpdir(), "cheaper-e2e-"));
  const ws = path.join(dir, "ws");
  await mkdir(ws, { recursive: true });
  const env: Record<string, string> = {
    ...(process.env.PATH !== undefined ? { PATH: process.env.PATH } : {}),
    ...(process.env.SystemRoot !== undefined ? { SystemRoot: process.env.SystemRoot } : {}),
    ...(process.env.TEMP !== undefined ? { TEMP: process.env.TEMP } : {}),
    ...(process.env.TMP !== undefined ? { TMP: process.env.TMP } : {}),
    HOME: dir,
    USERPROFILE: dir,
    PI_OFFLINE: "1",
    CHEAPER_CONFIG_DIR: path.join(dir, "cheaper-config"),
    NODE_ENV: "production",
  };
  return { dir, ws, env };
}
