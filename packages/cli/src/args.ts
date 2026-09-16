import { parseArgs } from "node:util";
import { CheaperError } from "@cheaper/core";

export interface RunCliArgs {
  kind: "run";
  /** named Worker。`--worker-file` 指定時は undefined。 */
  workerName: string | undefined;
  /** `run` 専用の明示 Worker file。named resolution を bypass する。 */
  workerFile: string | undefined;
  /** `--input` 値。`-` は stdin。 */
  input: string;
  cwd: string | undefined;
  /** `--timeout` の生文字列。main 側で parseDuration する。 */
  timeout: string | undefined;
  format: "json" | "text";
  quiet: boolean;
}

export interface CountTokensCliArgs {
  kind: "count-tokens";
  /** 測定対象 file（workspace 相対または絶対 path）。 */
  path: string;
  cwd: string | undefined;
  format: "json" | "text";
  quiet: boolean;
}

export type ParsedCli = { kind: "help" } | { kind: "version" } | RunCliArgs | CountTokensCliArgs;

const NAME_PATTERN = /^[a-z0-9][a-z0-9-]*$/;

const ALIAS_WORKERS: Record<string, string> = {
  "bulk-read": "bulk-reader",
  "code-write": "code-writer",
  "doc-write": "doc-writer",
};

function invalidRequest(message: string, details?: Record<string, unknown>): CheaperError {
  return new CheaperError("INVALID_REQUEST", message, details === undefined ? {} : { details });
}

function parseArgsSafe(argv: readonly string[]) {
  try {
    return parseArgs({
      args: argv as string[],
      allowPositionals: true,
      strict: true,
      options: {
        input: { type: "string", default: "-" },
        "worker-file": { type: "string" },
        cwd: { type: "string" },
        timeout: { type: "string" },
        format: { type: "string", default: "json" },
        quiet: { type: "boolean", default: false },
        help: { type: "boolean", default: false },
        version: { type: "boolean", default: false },
      },
    });
  } catch (e) {
    throw invalidRequest(`invalid arguments: ${e instanceof Error ? e.message : String(e)}`, {
      reason: "parse-args",
    });
  }
}

/**
 * `util.parseArgs` ベースの CLI 引数解析（DESIGN-0005）。
 * commands: `run <worker>` / `bulk-read` / `code-write` / `doc-write`（run のエイリアス）。
 * path 風 worker name は args 層で INVALID_WORKER_NAME に分類する。
 */
export function parseCliArgs(argv: readonly string[]): ParsedCli {
  const parsed = parseArgsSafe(argv);

  const { values, positionals } = parsed;

  if (values.help === true) return { kind: "help" };
  if (values.version === true) return { kind: "version" };

  const format = values.format;
  if (format !== "json" && format !== "text") {
    throw invalidRequest(`invalid --format: ${String(format)}`, { format });
  }

  const command = positionals[0];
  if (command === undefined) {
    throw invalidRequest(
      "command required: run | bulk-read | code-write | doc-write | count-tokens",
    );
  }

  const workerFile = values["worker-file"];

  if (command === "count-tokens") {
    if (positionals.length > 2) {
      throw invalidRequest(`unexpected arguments: ${positionals.slice(2).join(" ")}`);
    }
    const target = positionals[1];
    if (target === undefined) {
      throw invalidRequest("count-tokens requires <path>");
    }
    if (workerFile !== undefined) {
      throw invalidRequest("--worker-file is only valid for `run`");
    }
    if (values.timeout !== undefined) {
      throw invalidRequest("--timeout is only valid for worker runs");
    }
    if (values.input !== "-") {
      throw invalidRequest("--input is only valid for worker runs");
    }
    return {
      kind: "count-tokens",
      path: target,
      cwd: values.cwd,
      format,
      quiet: values.quiet === true,
    };
  }

  let workerName: string | undefined;

  if (command === "run") {
    if (positionals.length > 2) {
      throw invalidRequest(`unexpected arguments: ${positionals.slice(2).join(" ")}`);
    }
    workerName = positionals[1];
    if (workerName !== undefined && workerFile !== undefined) {
      throw invalidRequest("--worker-file cannot be combined with a worker name");
    }
    if (workerName === undefined && workerFile === undefined) {
      throw invalidRequest("run requires <worker> or --worker-file");
    }
  } else if (command in ALIAS_WORKERS) {
    if (positionals.length > 1) {
      throw invalidRequest(`unexpected arguments: ${positionals.slice(1).join(" ")}`);
    }
    if (workerFile !== undefined) {
      throw invalidRequest(`--worker-file is only valid for \`run\``);
    }
    workerName = ALIAS_WORKERS[command];
  } else {
    throw invalidRequest(`unknown command: ${command}`, { command });
  }

  if (workerName !== undefined && !NAME_PATTERN.test(workerName)) {
    throw new CheaperError("INVALID_WORKER_NAME", `invalid worker name: ${workerName}`, {
      details: { name: workerName },
    });
  }

  return {
    kind: "run",
    workerName,
    workerFile,
    input: values.input ?? "-",
    cwd: values.cwd,
    timeout: values.timeout,
    format,
    quiet: values.quiet === true,
  };
}
