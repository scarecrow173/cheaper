/**
 * Devin CLI adapter — cheaper 専用 data dir 環境の child process。
 * upstream pi-devin（MIT）の src/cli.ts を参考に、cheaper の
 * isolated data dir・runner seam 向けに再設計（ADR-0029）。
 */
import { execFile, spawn } from "node:child_process";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);

export interface DevinCliResult {
  stdout: string;
  stderr: string;
  code: number;
}

/**
 * `devin` child process の起動 seam（test は stub を注入する）。
 * argv 分離で起動し、shell string は使わない。
 */
export interface DevinCliRunner {
  run(
    bin: string,
    args: string[],
    opts: {
      env: NodeJS.ProcessEnv;
      timeoutMs?: number;
      inheritStdio?: boolean;
    },
  ): Promise<DevinCliResult>;
}

/** 本番 runner: execFile（pipe）/ spawn（inherit stdio）。 */
export const defaultDevinCliRunner: DevinCliRunner = {
  async run(bin, args, opts) {
    if (opts.inheritStdio === true) {
      return new Promise((resolve, reject) => {
        const child = spawn(bin, args, { stdio: "inherit", env: opts.env });
        child.on("error", reject);
        child.on("close", (code) => {
          resolve({ stdout: "", stderr: "", code: code ?? 1 });
        });
      });
    }
    try {
      const { stdout, stderr } = await execFileAsync(bin, args, {
        env: opts.env,
        timeout: opts.timeoutMs ?? 30_000,
        maxBuffer: 8 * 1024 * 1024,
      });
      return { stdout, stderr, code: 0 };
    } catch (error) {
      const err = error as {
        stdout?: string;
        stderr?: string;
        code?: number;
        message?: string;
      };
      if (typeof err.code === "number") {
        return { stdout: err.stdout ?? "", stderr: err.stderr ?? "", code: err.code };
      }
      throw error;
    }
  },
};

export interface DevinCli {
  /** 解決済み binary 識別子（`DEVIN_CLI` env または "devin"）。 */
  readonly bin: string;
  /** `devin` を isolated env で実行する。 */
  run(
    args: string[],
    opts?: { timeoutMs?: number; inheritStdio?: boolean },
  ): Promise<DevinCliResult>;
  /** `devin version`（取得不能時は null）。 */
  version(): Promise<string | null>;
}

export class DevinCliNotFoundError extends Error {
  constructor(bin: string) {
    super(
      `Devin CLI not found (${bin}). Install the Devin CLI, or set DEVIN_CLI to the devin binary.`,
    );
    this.name = "DevinCliNotFoundError";
  }
}

/**
 * child process 専用の isolated env を組み立てる。
 * `XDG_DATA_HOME`（win32 は `LOCALAPPDATA` 併用）で Devin CLI の
 * credential を cheaper data dir へ閉じ込める。`process.env` や
 * 渡された env object は変更しない。
 */
export function devinChildEnv(env: NodeJS.ProcessEnv, dataDir: string): NodeJS.ProcessEnv {
  const child: NodeJS.ProcessEnv = { ...env, XDG_DATA_HOME: dataDir };
  if (process.platform === "win32") {
    child.LOCALAPPDATA = dataDir;
  }
  return child;
}

function isNotFound(error: unknown): boolean {
  const code = (error as { code?: unknown }).code;
  return code === "ENOENT" || code === 127;
}

export function createDevinCli(opts: {
  env: NodeJS.ProcessEnv;
  dataDir: string;
  runner?: DevinCliRunner;
}): DevinCli {
  const bin = opts.env.DEVIN_CLI ?? "devin";
  const runner = opts.runner ?? defaultDevinCliRunner;
  const childEnv = devinChildEnv(opts.env, opts.dataDir);

  const run = async (
    args: string[],
    runOpts?: { timeoutMs?: number; inheritStdio?: boolean },
  ): Promise<DevinCliResult> => {
    try {
      return await runner.run(bin, args, { env: childEnv, ...runOpts });
    } catch (error) {
      if (isNotFound(error)) throw new DevinCliNotFoundError(bin);
      throw error;
    }
  };

  return {
    bin,
    run,
    async version() {
      try {
        const { stdout, code } = await run(["version"], { timeoutMs: 8_000 });
        if (code !== 0) return null;
        return stdout.trim() || null;
      } catch {
        return null;
      }
    },
  };
}
