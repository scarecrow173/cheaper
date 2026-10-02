import path from "node:path";
import { pathToFileURL } from "node:url";
import {
  buildAutoTierContext,
  CheaperError,
  canonicalizeWorkspaceRoot,
  classifyModel,
  FileTooLargeError,
  isExcludedModel,
  loadModelConfig,
  loadPolicy,
  type ModelInfo,
  parseDuration,
  resolveConfigDirs,
  resolvePricing,
  resolveWorker,
  resolveWorkerFile,
  restrictPolicy,
  validateWorkerInput,
  WorkspaceEscapeError,
} from "@cheaper/core";
import {
  type CreateCheaperModelRuntimeOptions,
  createModelRuntime,
  type ModelCatalog,
  type RunWorkerDeps,
  runWorker,
  type StreamFnLike,
  toModelInfo,
  type WorkerRunMeta,
} from "@cheaper/runtime-pi";
import { type ModelsCliArgs, parseCliArgs, type RunCliArgs } from "./args.js";
import { executeAuth } from "./auth.js";
import { countWorkspaceFileTokens, defaultBuiltinDir } from "./count.js";
import {
  type CliEnvelope,
  envelopeJson,
  errorEnvelope,
  successEnvelope,
  utilitySuccessEnvelope,
} from "./envelope.js";
import { readTaskInput, type StdinLike } from "./input.js";
import { formatText, formatUsageReport } from "./text-format.js";
import { recordUsage } from "./usage-log.js";
import { buildUsageReport, splitModelRef } from "./usage-report.js";
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
  createModelRuntime?: (options?: CreateCheaperModelRuntimeOptions) => Promise<Runtime> | Runtime;
  /** browser 起動差し替え seam（単体テスト用。未指定時は OS opener）。 */
  openUrl?: (url: string) => void;
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
  usage                  Aggregate usage.jsonl (worker cost / measured savings)
  models                 List available models with tier and pricing info
  auth status [provider] Show stored credentials and provider readiness
  auth login <provider>  Sign in to a provider (OAuth; e.g. openai-codex)
  auth logout <provider> Remove a provider credential

Options:
  --input <file|->       Task input JSON (default: stdin)
  --worker-file <path>   Run an explicit worker file (run only)
  --cwd <path>           Workspace root (default: process.cwd())
  --timeout <duration>   Narrow policy timeout (e.g. 30s, 5m)
  --model <prov>/<id>    run: explicit model; usage: filter by model
  --provider <id>        models/usage: filter by provider
  --tier <low|med|high>  Filter 'models' output by effective tier
  --worker <name>        Rank 'models' auto tiers by a worker's workload
  --method <id>          Auto-answer auth login select prompts (e.g. device)
  --type <oauth|api_key> Auth method for 'auth login' (default: oauth)
  --since <duration>     usage: only records within the period (e.g. 24h)
  --worker <name>        usage: filter by worker name
  --compare-model <p/id> usage: estimate savings vs this model
  --format <json|text>   Output format (default: json)
  --quiet                Suppress stderr diagnostics
  --help                 Show this help
  --version              Show version
`;

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
  onRunContext?: (ctx: { root: string; userConfigDir: string }) => void,
): Promise<{ envelope: CliEnvelope; exitCode: number; modelLine?: string }> {
  const builtinDir = hooks.builtinDir ?? defaultBuiltinDir;
  const now = hooks.now ?? Date.now;

  const root = await canonicalizeWorkspaceRoot(args.cwd ?? io.cwd);
  const dirs = resolveConfigDirs({ cwd: root, builtinDir, env: io.env });
  onRunContext?.({ root, userConfigDir: dirs.userConfigDir });

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

  const runtime = await (hooks.createModelRuntime ?? createModelRuntime)({ env: io.env });
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
      ...(args.model !== undefined ? { explicitModel: args.model } : {}),
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
 * `cheaper models`（issue #45 / #46）。runtime の available catalog を
 * 列挙し、各 model に exclude・effective pricing・classification を
 * 適用した一覧を `result.models[]` で返す。auto 分類は `--worker`
 * の workload profile（未指定は `DEFAULT_OUTPUT_PER_INPUT`）を基準に
 * eligible 母集団の相対 ranking で行う。worker run を伴わないため
 * utility envelope を使う。
 */
async function executeModels(
  args: ModelsCliArgs,
  io: CliIo,
  hooks: CliHooks,
  startedAt: number,
): Promise<{ envelope: CliEnvelope; exitCode: number }> {
  const now = hooks.now ?? Date.now;
  const builtinDir = hooks.builtinDir ?? defaultBuiltinDir;
  const root = await canonicalizeWorkspaceRoot(args.cwd ?? io.cwd);
  const dirs = resolveConfigDirs({ cwd: root, builtinDir, env: io.env });
  const modelConfig = await loadModelConfig({
    userConfigDir: dirs.userConfigDir,
    projectDir: dirs.projectDir,
  });
  let outputPerInput: number | undefined;
  if (args.worker !== undefined) {
    const worker = await resolveWorker(args.worker, dirs);
    outputPerInput = worker.cheaper.model.workload?.outputPerInput;
  }
  const runtime = await (hooks.createModelRuntime ?? createModelRuntime)({ env: io.env });
  const available = (await runtime.getAvailable()).map(toModelInfo);
  // ranking 母集団は exclude 適用後の eligible model。excluded 行の
  // 分類は表示用で、母集団自体には入れない（issue #46）。
  const autoContext = buildAutoTierContext(
    available.filter((m) => !isExcludedModel(m, modelConfig)),
    { ...(outputPerInput !== undefined ? { outputPerInput } : {}) },
  );
  const models = available
    .filter((m) => args.provider === undefined || m.provider === args.provider)
    .map((m) => {
      const pricing = resolvePricing(m);
      const classification = classifyModel(m, modelConfig, { pricing, autoContext });
      return {
        provider: m.provider,
        id: m.id,
        excluded: isExcludedModel(m, modelConfig),
        tier: classification.tier,
        tierSource: classification.source,
        ...(classification.auto !== undefined
          ? {
              effectiveCost: classification.auto.effectiveCost,
              effectiveCostPercentile: classification.auto.effectiveCostPercentile,
              workload: { outputPerInput: classification.auto.outputPerInput },
            }
          : {}),
        pricing: pricing ?? null,
      };
    })
    .filter((m) => args.tier === undefined || m.tier === args.tier);
  return {
    envelope: utilitySuccessEnvelope({ result: { models }, durationMs: now() - startedAt }),
    exitCode: 0,
  };
}

/**
 * `cheaper usage --compare-model` 用に runtime catalog から対象 model の
 * 観測（cost 等）を取る（issue #45 pricing layer の runtime-catalog source）。
 * 集計自体は read-only が本筋のため、catalog 不在・runtime 初期化失敗は
 * estimated を諦めるだけで report には影響させない（best-effort）。
 * `refreshOnCreate: false` で network refresh を行わず、persisted catalog
 * と builtin の static model だけを見る。
 */
async function lookupCatalogModel(
  ref: { provider: string; id: string },
  io: CliIo,
  hooks: CliHooks,
): Promise<ModelInfo | undefined> {
  try {
    const runtime = await (hooks.createModelRuntime ?? createModelRuntime)({
      env: io.env,
      refreshOnCreate: false,
    });
    const model = runtime.getModel(ref.provider, ref.id);
    return model === undefined ? undefined : toModelInfo(model);
  } catch {
    return undefined;
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
  let runRoot: string | undefined;
  let runUserConfigDir: string | undefined;
  // executeRun が途中で throw した場合の記録用 fallback（resolveConfigDirs
  // は純粋関数で、userConfigDir は env だけに依存する）。
  const resolvedUserConfigDir = () =>
    runUserConfigDir ??
    resolveConfigDirs({
      cwd: runRoot ?? io.cwd,
      builtinDir: hooks.builtinDir ?? defaultBuiltinDir,
      env: io.env,
    }).userConfigDir;

  try {
    const parsed = parseCliArgs(argv);
    if (parsed.kind === "help") {
      io.stdout.write(HELP);
      return 0;
    }
    if (parsed.kind === "version") {
      io.stdout.write(`${packageVersion(hooks.builtinDir ?? defaultBuiltinDir)}\n`);
      return 0;
    }
    if (parsed.kind === "count-tokens") {
      // LLM/model/worker 解決を伴わない測定専用経路（DESIGN-0012）。
      // effective policy から filesystem.symlinks のみ使う。
      const builtinDir = hooks.builtinDir ?? defaultBuiltinDir;
      const { count, root, userConfigDir } = await countWorkspaceFileTokens({
        cwd: parsed.cwd ?? io.cwd,
        path: parsed.path,
        builtinDir,
        env: io.env,
      }).catch((e) => {
        if (e instanceof FileTooLargeError) {
          throw new CheaperError("FILE_TOO_LARGE", e.message, {
            details: { path: e.path, bytes: e.bytes, maxBytes: e.maxBytes },
            cause: e,
          });
        }
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
      recordUsage({
        env: envelope,
        root,
        userConfigDir,
        stderr: io.stderr,
        quiet: parsed.quiet,
        now: () => new Date(now()),
      });
      return 0;
    }

    if (parsed.kind === "usage") {
      // usage.jsonl の read-only 集計。worker/model 解決を伴わないため
      // config dir のみ解決する。usage command 自体は記録対象外。
      let sinceMs: number | undefined;
      if (parsed.since !== undefined) {
        try {
          sinceMs = parseDuration(parsed.since);
        } catch {
          throw invalidRequest(`invalid --since duration: ${parsed.since}`, {
            since: parsed.since,
          });
        }
      }
      if (parsed.model !== undefined && splitModelRef(parsed.model) === null) {
        throw invalidRequest(`invalid --model: ${parsed.model} (expected <provider>/<id>)`, {
          model: parsed.model,
        });
      }
      if (parsed.compareModel !== undefined && splitModelRef(parsed.compareModel) === null) {
        throw invalidRequest(
          `invalid --compare-model: ${parsed.compareModel} (expected <provider>/<id>)`,
          { compareModel: parsed.compareModel },
        );
      }
      const dirs = resolveConfigDirs({
        cwd: io.cwd,
        builtinDir: hooks.builtinDir ?? defaultBuiltinDir,
        env: io.env,
      });
      const compareModelRef =
        parsed.compareModel !== undefined ? splitModelRef(parsed.compareModel) : null;
      const report = buildUsageReport({
        userConfigDir: dirs.userConfigDir,
        filters: {
          ...(sinceMs !== undefined ? { sinceMs } : {}),
          ...(parsed.worker !== undefined ? { worker: parsed.worker } : {}),
          ...(parsed.provider !== undefined ? { provider: parsed.provider } : {}),
          ...(parsed.model !== undefined ? { model: parsed.model } : {}),
        },
        ...(parsed.compareModel !== undefined ? { compareModel: parsed.compareModel } : {}),
        // runtime catalog の cost を estimated の第1供給源として渡す
        //（未取得でも snapshot 経路で試行される）。
        ...(compareModelRef !== null && compareModelRef !== undefined
          ? { compareCatalogModel: await lookupCatalogModel(compareModelRef, io, hooks) }
          : {}),
        now,
      });
      if (parsed.format === "text") {
        io.stdout.write(formatUsageReport(report));
      } else {
        const envelope = utilitySuccessEnvelope({
          result: report as unknown as Record<string, unknown>,
          durationMs: now() - startedAt,
        });
        io.stdout.write(envelopeJson(envelope));
      }
      return 0;
    }

    if (parsed.kind === "models") {
      const { envelope, exitCode } = await executeModels(parsed, io, hooks, startedAt);
      io.stdout.write(parsed.format === "text" ? formatText(envelope) : envelopeJson(envelope));
      return exitCode;
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
      (ctx) => {
        runRoot = ctx.root;
        runUserConfigDir = ctx.userConfigDir;
      },
    );
    if (modelLine !== undefined && !parsed.quiet) io.stderr.write(`${modelLine}\n`);
    io.stdout.write(parsed.format === "text" ? formatText(envelope) : envelopeJson(envelope));
    recordUsage({
      env: envelope,
      root: runRoot ?? parsed.cwd ?? io.cwd,
      userConfigDir: resolvedUserConfigDir(),
      stderr: io.stderr,
      quiet: parsed.quiet,
      now: () => new Date(now()),
    });
    return exitCode;
  } catch (err) {
    const envelope = errorEnvelope(err, { worker: workerName, durationMs: now() - startedAt });
    let format: "json" | "text" = "json";
    let quiet = false;
    try {
      const p = parseCliArgs(argv);
      if (
        p.kind === "run" ||
        p.kind === "count-tokens" ||
        p.kind === "usage" ||
        p.kind === "auth"
      ) {
        format = p.format;
        quiet = p.quiet;
      }
    } catch {
      // args 自体が不正なら json のまま
    }
    io.stderr.write(`error ${envelope.error.code}: ${envelope.error.message}\n`);
    io.stdout.write(format === "text" ? formatText(envelope) : envelopeJson(envelope));
    recordUsage({
      env: envelope,
      root: runRoot ?? io.cwd,
      userConfigDir: resolvedUserConfigDir(),
      stderr: io.stderr,
      quiet,
      now: () => new Date(now()),
    });
    return envelope.error.exitCode;
  }
}
