import { parseArgs } from "node:util";
import { CheaperError, parseModelRef, TIERS, type Tier } from "@cheaper/core";

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
  /** `--model <provider>/<id>` 直接指定（issue #45）。 */
  model: string | undefined;
  format: "json" | "text";
  quiet: boolean;
}

export interface ModelsCliArgs {
  kind: "models";
  /** `--provider <id>` filter。 */
  provider: string | undefined;
  /** `--tier <low|medium|high>` filter（effective tier 基準）。 */
  tier: Tier | undefined;
  cwd: string | undefined;
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

export interface AuthCliArgs {
  kind: "auth";
  /** `auth status|login|logout` の action。 */
  action: "status" | "login" | "logout";
  /** login/logout 必須、status では optional filter。 */
  provider: string | undefined;
  /** login の `select` prompt へ即答する option id（browser/device 等）。 */
  method: string | undefined;
  /** login の auth type。既定 oauth。api_key は対話 key 入力。 */
  authType: "oauth" | "api_key";
  format: "json" | "text";
  quiet: boolean;
}

export interface UsageCliArgs {
  kind: "usage";
  /** `--since` の生文字列。main 側で parseDuration する。 */
  since: string | undefined;
  worker: string | undefined;
  provider: string | undefined;
  /** `<provider>/<id>` 形式。 */
  model: string | undefined;
  /** `<provider>/<id>` 形式。estimated 算出の comparison model。 */
  compareModel: string | undefined;
  format: "json" | "text";
  quiet: boolean;
}

export type ParsedCli =
  | { kind: "help" }
  | { kind: "version" }
  | RunCliArgs
  | CountTokensCliArgs
  | AuthCliArgs
  | ModelsCliArgs
  | UsageCliArgs;

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
        method: { type: "string" },
        type: { type: "string" },
        model: { type: "string" },
        provider: { type: "string" },
        tier: { type: "string" },
        since: { type: "string" },
        worker: { type: "string" },
        "compare-model": { type: "string" },
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
      "command required: run | bulk-read | code-write | doc-write | count-tokens | usage | models | auth",
    );
  }

  const workerFile = values["worker-file"];
  // --model/--provider は run/models と意味を共有するため usage 専用には
  // 含めない（usage では record filter、run では explicit model 指定）。
  const usageOnlyOptionUsed =
    values.since !== undefined ||
    values.worker !== undefined ||
    values["compare-model"] !== undefined;

  if (command === "usage") {
    if (positionals.length > 1) {
      throw invalidRequest(`unexpected arguments: ${positionals.slice(1).join(" ")}`);
    }
    if (workerFile !== undefined) {
      throw invalidRequest("--worker-file is only valid for `run`");
    }
    if (values.timeout !== undefined || values.input !== "-") {
      throw invalidRequest("--timeout/--input are only valid for worker runs");
    }
    if (values.method !== undefined || values.type !== undefined) {
      throw invalidRequest("--method/--type are only valid for `auth login`");
    }
    if (values.tier !== undefined) {
      throw invalidRequest("--tier is only valid for `models`");
    }
    return {
      kind: "usage",
      since: values.since,
      worker: values.worker,
      provider: values.provider,
      model: values.model,
      compareModel: values["compare-model"],
      format,
      quiet: values.quiet === true,
    };
  }

  if (usageOnlyOptionUsed) {
    throw invalidRequest("--since/--worker/--compare-model are only valid for `usage`");
  }

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
    if (values.method !== undefined || values.type !== undefined) {
      throw invalidRequest("--method/--type are only valid for `auth login`");
    }
    if (values.model !== undefined) {
      throw invalidRequest("--model is only valid for worker runs or `usage`");
    }
    if (values.provider !== undefined) {
      throw invalidRequest("--provider is only valid for `models` or `usage`");
    }
    if (values.tier !== undefined) {
      throw invalidRequest("--tier is only valid for `models`");
    }
    return {
      kind: "count-tokens",
      path: target,
      cwd: values.cwd,
      format,
      quiet: values.quiet === true,
    };
  }

  if (command === "models") {
    if (positionals.length > 1) {
      throw invalidRequest(`unexpected arguments: ${positionals.slice(1).join(" ")}`);
    }
    if (workerFile !== undefined) {
      throw invalidRequest("--worker-file is only valid for `run`");
    }
    if (values.model !== undefined) {
      throw invalidRequest("--model is only valid for worker runs or `usage`");
    }
    if (values.timeout !== undefined) {
      throw invalidRequest("--timeout is only valid for worker runs");
    }
    if (values.input !== "-") {
      throw invalidRequest("--input is only valid for worker runs");
    }
    if (values.method !== undefined || values.type !== undefined) {
      throw invalidRequest("--method/--type are only valid for `auth login`");
    }
    const tier = values.tier;
    if (tier !== undefined && !(TIERS as readonly string[]).includes(tier)) {
      throw invalidRequest(`invalid --tier: ${tier} (expected ${TIERS.join("|")})`, { tier });
    }
    return {
      kind: "models",
      provider: values.provider,
      tier: tier as Tier | undefined,
      cwd: values.cwd,
      format,
      quiet: values.quiet === true,
    };
  }

  if (command === "auth") {
    if (workerFile !== undefined) {
      throw invalidRequest("--worker-file is only valid for `run`");
    }
    if (values.timeout !== undefined) {
      throw invalidRequest("--timeout is only valid for worker runs");
    }
    if (values.input !== "-") {
      throw invalidRequest("--input is only valid for worker runs");
    }
    const action = positionals[1];
    if (action !== "status" && action !== "login" && action !== "logout") {
      throw invalidRequest(`unknown auth action: ${String(action)} (expected status|login|logout)`);
    }
    if (positionals.length > 3) {
      throw invalidRequest(`unexpected arguments: ${positionals.slice(3).join(" ")}`);
    }
    const provider = positionals[2];
    if (provider === undefined && action !== "status") {
      throw invalidRequest(`auth ${action} requires <provider>`);
    }
    if (values.method !== undefined && action !== "login") {
      throw invalidRequest("--method is only valid for `auth login`");
    }
    const type = values.type;
    if (type !== undefined && action !== "login") {
      throw invalidRequest("--type is only valid for `auth login`");
    }
    if (type !== undefined && type !== "oauth" && type !== "api_key") {
      throw invalidRequest(`invalid --type: ${type} (expected oauth|api_key)`, { type });
    }
    if (values.model !== undefined) {
      throw invalidRequest("--model is only valid for worker runs or `usage`");
    }
    if (values.provider !== undefined) {
      throw invalidRequest("--provider is only valid for `models` or `usage`");
    }
    if (values.tier !== undefined) {
      throw invalidRequest("--tier is only valid for `models`");
    }
    return {
      kind: "auth",
      action,
      provider,
      method: values.method,
      authType: type ?? "oauth",
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

  if (values.method !== undefined || values.type !== undefined) {
    throw invalidRequest("--method/--type are only valid for `auth login`");
  }
  if (values.provider !== undefined) {
    throw invalidRequest("--provider is only valid for `models` or `usage`");
  }
  if (values.tier !== undefined) {
    throw invalidRequest("--tier is only valid for `models`");
  }

  if (workerName !== undefined && !NAME_PATTERN.test(workerName)) {
    throw new CheaperError("INVALID_WORKER_NAME", `invalid worker name: ${workerName}`, {
      details: { name: workerName },
    });
  }

  const model = values.model;
  if (model !== undefined && parseModelRef(model) === undefined) {
    throw invalidRequest(`invalid --model reference: ${model} (expected <provider>/<id>)`, {
      model,
    });
  }

  return {
    kind: "run",
    workerName,
    workerFile,
    input: values.input ?? "-",
    cwd: values.cwd,
    timeout: values.timeout,
    model,
    format,
    quiet: values.quiet === true,
  };
}
