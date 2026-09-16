import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import {
  CheaperError,
  canonicalizeWorkspaceRoot,
  countFileTokens,
  loadModelConfig,
  loadPolicy,
  parseDuration,
  resolveConfigDirs,
  resolveWorker,
  resolveWorkerFile,
  restrictPolicy,
  validateWorkerInput,
  WorkspaceEscapeError,
} from "@cheaper/core";
import {
  createModelRuntime,
  type ModelCatalog,
  type RunWorkerDeps,
  runWorker,
  type StreamFnLike,
  type WorkerRunMeta,
} from "@cheaper/runtime-pi";
import { parseCliArgs, type RunCliArgs } from "./args.js";
import { executeAuth } from "./auth.js";
import {
  type CliEnvelope,
  envelopeJson,
  errorEnvelope,
  successEnvelope,
  utilitySuccessEnvelope,
} from "./envelope.js";
import { readTaskInput, type StdinLike } from "./input.js";
import { formatText } from "./text-format.js";
import { packageVersion } from "./version.js";

export interface CliIo {
  stdin: StdinLike;
  stdout: { write(s: string): unknown };
  stderr: { write(s: string): unknown };
  env: NodeJS.ProcessEnv;
  cwd: string;
}

type Runtime = Awaited<ReturnType<typeof createModelRuntime>>;

export interface CliHooks {
  /** runWorker 差し替え seam（単体テスト用）。 */
  runWorker?: (
    deps: RunWorkerDeps,
  ) => Promise<{ result: Record<string, unknown>; meta: WorkerRunMeta }>;
  /** ModelRuntime 生成差し替え seam（単体テスト用）。 */
  createModelRuntime?: () => Promise<Runtime> | Runtime;
  /** built-in worker/policy の親 dir（既定: このパッケージ root）。 */
  builtinDir?: string;
  /** 外部 abort（テスト用。本番は process の SIGINT/SIGTERM を中継）。 */
  signal?: AbortSignal;
  now?: () => number;
}

const HELP = `Usage: cheaper <command> [options]

Commands:
  run <worker>           Run a named worker (or --worker-file <path>)
  bulk-read              Alias for: run bulk-reader
  code-write             Alias for: run code-writer
  doc-write              Alias for: run doc-writer
  count-tokens <path>    Count canonical tokens of a workspace file
  auth status [provider] Show stored credentials and provider readiness
  auth login <provider>  Sign in to a provider (OAuth; e.g. openai-codex)
  auth logout <provider> Remove a provider credential

Options:
  --input <file|->       Task input JSON (default: stdin)
  --worker-file <path>   Run an explicit worker file (run only)
  --cwd <path>           Workspace root (default: process.cwd())
  --timeout <duration>   Narrow policy timeout (e.g. 30s, 5m)
  --method <id>          Auto-answer auth login select prompts (e.g. device)
  --format <json|text>   Output format (default: json)
  --quiet                Suppress stderr diagnostics
  --help                 Show this help
  --version              Show version
`;

function defaultBuiltinDir(): string {
  return fileURLToPath(new URL("..", import.meta.url));
}

function invalidRequest(message: string, details?: Record<string, unknown>): CheaperError {
  return new CheaperError("INVALID_REQUEST", message, details === undefined ? {} : { details });
}

/**
 * test-only fake stream フック（TASK-0048）。
 * `NODE_ENV === "test"` かつ `CHEAPER_TEST_FAKE_STREAM` 指定時のみ有効。
 * module の default export は `StreamFnLike を返す関数`（ctx.modelRuntime を受け取る）
 * または scripted 応答配列（runtime-pi testsupport の createFakePi で wrap）。
 */
async function loadTestStreamFn(
  env: NodeJS.ProcessEnv,
  runtime: Runtime,
): Promise<StreamFnLike | undefined> {
  if (env.NODE_ENV !== "test") return undefined;
  const spec = env.CHEAPER_TEST_FAKE_STREAM;
  if (spec === undefined || spec === "") return undefined;

  const href = pathToFileURL(path.resolve(spec)).href;
  const mod = (await import(href)) as { default?: unknown };
  const def = mod.default;

  if (typeof def === "function") {
    const factory = def as (ctx: { modelRuntime: Runtime }) => StreamFnLike | Promise<StreamFnLike>;
    return factory({ modelRuntime: runtime });
  }
  if (Array.isArray(def)) {
    const testsupport = (await import("@cheaper/runtime-pi/testsupport")) as {
      createFakePi: (opts: { responses: unknown[] }) => {
        provider: unknown;
        streamFn: StreamFnLike;
      };
    };
    const fake = testsupport.createFakePi({ responses: def });
    runtime.registerNativeProvider(fake.provider as never);
    await runtime.refresh({ allowNetwork: false });
    return fake.streamFn;
  }
  throw invalidRequest(
    "CHEAPER_TEST_FAKE_STREAM module default export must be a function or a response array",
    { spec },
  );
}

async function executeRun(
  args: RunCliArgs,
  io: CliIo,
  hooks: CliHooks,
  startedAt: number,
  onWorkerResolved?: (name: string) => void,
): Promise<{ envelope: CliEnvelope; exitCode: number; modelLine?: string }> {
  const builtinDir = hooks.builtinDir ?? defaultBuiltinDir();
  const now = hooks.now ?? Date.now;

  const root = await canonicalizeWorkspaceRoot(args.cwd ?? io.cwd);
  const dirs = resolveConfigDirs({ cwd: root, builtinDir, env: io.env });

  const worker =
    args.workerFile !== undefined
      ? await resolveWorkerFile(args.workerFile, root)
      : await resolveWorker(args.workerName as string, dirs);
  onWorkerResolved?.(worker.name);

  const input = await readTaskInput(args.input, io.stdin);
  validateWorkerInput(worker, input);

  const policy = await loadPolicy({
    builtinPolicyPath: path.join(builtinDir, "policy.yaml"),
    userConfigDir: dirs.userConfigDir,
    projectDir: dirs.projectDir,
  });

  let timeoutMs: number | undefined;
  if (args.timeout !== undefined) {
    try {
      timeoutMs = parseDuration(args.timeout);
    } catch {
      throw invalidRequest(`invalid --timeout duration: ${args.timeout}`, {
        timeout: args.timeout,
      });
    }
  }
  const effectivePolicy = restrictPolicy(policy, {
    ...(timeoutMs !== undefined ? { timeoutMs } : {}),
  });

  const modelConfig = await loadModelConfig({
    userConfigDir: dirs.userConfigDir,
    projectDir: dirs.projectDir,
  });

  const runtime = await (hooks.createModelRuntime ?? createModelRuntime)();
  const streamFn = await loadTestStreamFn(io.env, runtime);

  // timeout タイマーは runtime-pi 内の policy.execution.timeoutMs が担う。
  // CLI 側は SIGINT/SIGTERM（とテスト用 signal）を AbortController で中継するだけ。
  const ac = new AbortController();
  const onAbort = () => ac.abort();
  const onSigint = () => ac.abort();
  const onSigterm = () => ac.abort();
  process.on("SIGINT", onSigint);
  process.on("SIGTERM", onSigterm);
  hooks.signal?.addEventListener("abort", onAbort, { once: true });
  if (hooks.signal?.aborted === true) ac.abort();

  try {
    const deps: RunWorkerDeps = {
      worker,
      input,
      root,
      policy: effectivePolicy,
      modelConfig,
      catalog: runtime as ModelCatalog,
      modelRuntime: runtime,
      signal: ac.signal,
      ...(streamFn !== undefined ? { streamFn } : {}),
    };
    const { result, meta } = await (hooks.runWorker ?? runWorker)(deps);
    return {
      envelope: successEnvelope({
        worker: worker.name,
        result,
        meta,
        durationMs: now() - startedAt,
      }),
      exitCode: 0,
      modelLine: `model: ${meta.model.provider}/${meta.model.id} (tier ${meta.model.tier})`,
    };
  } finally {
    process.off("SIGINT", onSigint);
    process.off("SIGTERM", onSigterm);
    hooks.signal?.removeEventListener("abort", onAbort);
  }
}

/**
 * CLI オーケストレーション（DESIGN-0005）。
 * stdout には envelope 1 つ（+末尾改行）のみ書く。未捕捉例外は INTERNAL_ERROR(70)。
 */
export async function runCli(
  argv: readonly string[],
  io: CliIo,
  hooks: CliHooks = {},
): Promise<number> {
  const now = hooks.now ?? Date.now;
  const startedAt = now();
  let workerName: string | null = null;

  try {
    const parsed = parseCliArgs(argv);
    if (parsed.kind === "help") {
      io.stdout.write(HELP);
      return 0;
    }
    if (parsed.kind === "version") {
      io.stdout.write(`${packageVersion()}\n`);
      return 0;
    }
    if (parsed.kind === "count-tokens") {
      // LLM/model/worker 解決を伴わない測定専用経路（DESIGN-0012）。
      // effective policy から filesystem.symlinks のみ使う。
      const builtinDir = hooks.builtinDir ?? defaultBuiltinDir();
      const root = await canonicalizeWorkspaceRoot(parsed.cwd ?? io.cwd);
      const dirs = resolveConfigDirs({ cwd: root, builtinDir, env: io.env });
      const policy = await loadPolicy({
        builtinPolicyPath: path.join(builtinDir, "policy.yaml"),
        userConfigDir: dirs.userConfigDir,
        projectDir: dirs.projectDir,
      });
      const count = await countFileTokens({
        root,
        path: parsed.path,
        symlinkPolicy: policy.filesystem.symlinks,
      }).catch((e) => {
        if (e instanceof WorkspaceEscapeError) {
          throw new CheaperError("INVALID_REQUEST", e.message, {
            details: { path: parsed.path },
            cause: e,
          });
        }
        if ((e as NodeJS.ErrnoException).code === "ENOENT") {
          throw new CheaperError("INVALID_REQUEST", `file not found: ${parsed.path}`, {
            details: { path: parsed.path },
            cause: e,
          });
        }
        throw e;
      });
      const envelope = utilitySuccessEnvelope({
        result: { ...count },
        durationMs: now() - startedAt,
      });
      io.stdout.write(parsed.format === "text" ? formatText(envelope) : envelopeJson(envelope));
      return 0;
    }

    if (parsed.kind === "auth") {
      const { envelope, exitCode } = await executeAuth(parsed, io, hooks, startedAt);
      io.stdout.write(parsed.format === "text" ? formatText(envelope) : envelopeJson(envelope));
      return exitCode;
    }

    const { envelope, exitCode, modelLine } = await executeRun(
      parsed,
      io,
      hooks,
      startedAt,
      (name) => {
        workerName = name;
      },
    );
    if (modelLine !== undefined && !parsed.quiet) io.stderr.write(`${modelLine}\n`);
    io.stdout.write(parsed.format === "text" ? formatText(envelope) : envelopeJson(envelope));
    return exitCode;
  } catch (err) {
    const envelope = errorEnvelope(err, { worker: workerName, durationMs: now() - startedAt });
    let format: "json" | "text" = "json";
    try {
      const p = parseCliArgs(argv);
      if (p.kind === "run" || p.kind === "count-tokens" || p.kind === "auth") format = p.format;
    } catch {
      // args 自体が不正なら json のまま
    }
    io.stderr.write(`error ${envelope.error.code}: ${envelope.error.message}\n`);
    io.stdout.write(format === "text" ? formatText(envelope) : envelopeJson(envelope));
    return envelope.error.exitCode;
  }
}
