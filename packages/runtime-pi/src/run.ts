import path from "node:path";
import {
  CheaperError,
  checkCapabilities,
  countTextTokens,
  createPathDenylist,
  type ErrorCode,
  isDirectlyComparableWorker,
  type ModelConfig,
  parseModelRef,
  type RuntimePolicy,
  resolveWorkspacePath,
  type SymlinkPolicy,
  type Tier,
  type TierSource,
  type WorkerDefinition,
} from "@cheaper/core";
import {
  createInvocationToolRuntime,
  type ExecutionMetrics,
  type InvocationToolRuntime,
  type ToolRuntime,
  type WorkerExecutionBinding,
} from "@cheaper/tool-runtime";
import type { ModelRuntime } from "@earendil-works/pi-coding-agent";
import type { ChangesSummary } from "./journal.js";
import { MutationJournal } from "./journal.js";
import { type ModelCatalog, resolveExplicitPiModel, resolvePiModel } from "./model-runtime.js";
import { extractOutput, lastAssistantStop } from "./output.js";
import { promptWithAbortGrace } from "./prompt-grace.js";
import { repairOutput } from "./repair.js";
import { createWorkerSession, type StreamFnLike } from "./session.js";
import { createPiToolHandlers, createSandboxedTools } from "./tools/registry.js";
import { UsageCollector, type WorkerSavings, type WorkerUsage } from "./usage.js";

export interface WorkerRunMeta {
  model: {
    provider: string;
    id: string;
    tier: Tier;
    /** "tier" = tier resolver 経路、"explicit" = `--model` 直接指定。 */
    selection: "tier" | "explicit";
    /** tier の provenance（explicit-override / curated-exception / auto / unclassified）。 */
    tierSource: TierSource;
    /** auto 分類の根拠（tierSource === "auto" のときのみ存在、issue #46）。 */
    effectiveCost?: number;
    effectiveCostPercentile?: number;
    workload?: { outputPerInput: number };
  };
  usage: WorkerUsage;
  turns: number;
  toolCalls: number;
  rejectedToolCalls: number;
  repairs: number;
  transientRetries: number;
  changes: ChangesSummary;
  /** 共有 ADR D3: この Worker の owner 集合、scope teardown 前の snapshot。 */
  execution: ExecutionMetrics;
  /** 親コンテキスト offload の実測（SPEC-034rzGnCaRQc5ceI95lf5i）。 */
  savings?: WorkerSavings;
}

/**
 * runWorker の throw 経路で「その時点の meta snapshot」を運ぶ CheaperError。
 * CLI の error envelope は `runMeta` から meta field を埋める
 * （取得不能な field のみ null）。
 */
export class WorkerRunError extends CheaperError {
  readonly runMeta: (Partial<WorkerRunMeta> & { durationMs?: number }) | undefined;

  constructor(
    code: ErrorCode,
    message: string,
    options?: {
      details?: Record<string, unknown>;
      cause?: unknown;
      runMeta?: Partial<WorkerRunMeta> & { durationMs?: number };
    },
  ) {
    super(code, message, options);
    this.name = "WorkerRunError";
    this.runMeta = options?.runMeta;
  }
}

export interface RunWorkerDeps {
  worker: WorkerDefinition;
  input: unknown;
  /** canonical 化済み workspace root。 */
  root: string;
  policy: RuntimePolicy;
  modelConfig: ModelConfig;
  /** model 解決用 catalog（実運用では ModelRuntime）。 */
  catalog: ModelCatalog;
  /**
   * `--model <provider>/<id>` 直接指定（issue #45）。指定時は tier
   * 候補選択を bypass するが exclude・availability・policy.maxTier
   * は enforce される。
   */
  explicitModel?: string;
  /** session 用 ModelRuntime。通常は catalog と同じ実体。 */
  modelRuntime?: ModelRuntime;
  /** テスト用 stream seam。session.agent.streamFunction の差し替え元。 */
  streamFn?: StreamFnLike;
  /** 外部 abort（CLI の SIGINT 等）。 */
  signal?: AbortSignal;
  /** abort 後に prompt()  settle を待つ猶予 ms（既定 5000）。 */
  abortGraceMs?: number;
  /** D7: invocation を借りる場合はこの Worker の child scope だけを所有する。 */
  execution?: WorkerExecutionBinding;
  /** owned invocation の operation timestamp/snapshot 用 monotonic clock。 */
  operationClock?: () => number;
}

type AbortReason = "timeout" | "signal" | "maxTurns" | "maxToolCalls";

const ABORT_CODES: Record<AbortReason, ErrorCode> = {
  timeout: "TIMEOUT",
  signal: "ABORTED",
  maxTurns: "MAX_TURNS_EXCEEDED",
  maxToolCalls: "MAX_TOOL_CALLS_EXCEEDED",
};

function abortError(reason: AbortReason): CheaperError {
  return new CheaperError(ABORT_CODES[reason], `worker run aborted: ${reason}`, {
    details: { reason },
  });
}

export function buildSystemPrompt(worker: WorkerDefinition, toolNames: string[]): string {
  const contract = [
    "",
    "## Output contract",
    "",
    "- The final message must be a single JSON object that validates against the output schema below.",
    "- Do not wrap the JSON in markdown code fences or any surrounding prose.",
    "- All file paths are workspace-relative POSIX paths.",
    `- Available tools: ${toolNames.length > 0 ? toolNames.join(", ") : "none"}`,
    // same-turn batching ガイダンス（issue #55 / DESIGN-034zAKcWuB8SuFkOePsubg）。
    // tool 0 件の "none" と同居しないよう非空時のみ emit する。
    ...(toolNames.length > 0
      ? [
          "- Independent tool calls must be issued together in the same assistant turn; do not serialize independent reads, searches, or token counting across multiple turns.",
          "- Do not poll or re-ask whether a call finished: the runtime owns timeouts and process lifecycle, and every tool result is returned automatically as input for the next turn.",
          "- Do not batch unsafe mutations: mutation calls that depend on each other or apply multiple mutations to the same target, and reads that expect the result of a sibling write or edit, are not independent — issue them in separate turns.",
          "- Keep each same-turn batch and its per-call arguments small enough to fit within the output token limit applied to each response; an oversized response is truncated and every tool call in it fails unexecuted.",
          "- This batching guidance does not change the final-message contract: the last message is still a single JSON object that validates against the output schema below.",
        ]
      : []),
    "",
    "Output schema:",
    JSON.stringify(worker.outputSchema.schema, null, 2),
  ];
  return `${worker.instructions.trimEnd()}\n${contract.join("\n")}\n`;
}

function buildUserPrompt(input: unknown): string {
  return `Task input (JSON):\n${JSON.stringify(input, null, 2)}`;
}

function isPlainObject(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

/**
 * `cheaper.writeScope` で宣言された input field から write 許可 path 集合を
 * 抽出し workspace 相対 display path に正規化する（DESIGN-0015）。
 * field 不在・非 array・item 不正は fail closed で INVALID_INPUT。
 */
async function extractWriteScope(
  worker: WorkerDefinition,
  input: unknown,
  root: string,
  symlinkPolicy: SymlinkPolicy,
): Promise<Set<string> | undefined> {
  const field = worker.cheaper.writeScope;
  if (field === undefined) return undefined;
  const value = isPlainObject(input) ? input[field] : undefined;
  const invalid = (reason: string) =>
    new CheaperError("INVALID_INPUT", `writeScope field invalid: ${reason}`, {
      details: { field, reason },
    });
  if (!Array.isArray(value)) throw invalid("missing-or-not-array");
  const allowed = new Set<string>();
  for (const item of value) {
    const p =
      typeof item === "string"
        ? item
        : isPlainObject(item) && typeof item.path === "string"
          ? item.path
          : undefined;
    if (p === undefined) throw invalid("item-lacks-path");
    const resolved = await resolveWorkspacePath(root, path.join(root, p), symlinkPolicy);
    allowed.add(resolved.display);
  }
  return allowed;
}

/**
 * Worker を Pi session で 1 回実行する（DESIGN-0004）。
 * 制限（timeout/maxTurns/maxToolCalls/外部 signal）は cheaper 側で記録した
 * abort reason により分類する。journal 非空の abort は PARTIAL_MUTATION で包み、
 * cause に元エラーを保持する。
 */
export async function runWorker(
  deps: RunWorkerDeps,
): Promise<{ result: Record<string, unknown>; meta: WorkerRunMeta }> {
  const { worker, policy } = deps;
  const effectiveTools = checkCapabilities(worker, policy);

  const journal = new MutationJournal();
  const collector = new UsageCollector();
  const graceMs = deps.abortGraceMs ?? 5000;
  // run 全体の内部 abort。timeout・外部 signal・maxTurns/maxToolCalls の
  // 全 trigger が requestAbort を経由してこれを fire する。repair session は
  // runAbort.signal を受け取り、main session とは別に自分の session.abort() を呼ぶ。
  const runAbort = new AbortController();
  let repairs = 0;

  let abortReason: AbortReason | undefined;

  function requestAbort(reason: AbortReason): void {
    if (abortReason !== undefined) return;
    abortReason = reason;
    runAbort.abort(abortError(reason));
  }

  // policy timeout は agent execution 区間（model 解決・session 構築を含む
  // run 全体）を計測する。CLI 前段は対象外（DESIGN-0010 / ADR-0010）。
  const timeout = setTimeout(() => requestAbort("timeout"), policy.execution.timeoutMs);
  const onSignalAbort = () => requestAbort("signal");
  deps.signal?.addEventListener("abort", onSignalAbort, { once: true });
  if (deps.signal?.aborted) requestAbort("signal");

  const ownsInvocation = deps.execution === undefined;
  const ownerId = deps.execution?.ownerId ?? "worker";
  let invocation: InvocationToolRuntime | undefined;
  let toolScope: ToolRuntime | undefined;

  try {
    invocation =
      deps.execution?.runtime ??
      createInvocationToolRuntime({
        root: deps.root,
        symlinkPolicy: policy.filesystem.symlinks,
        concurrency: policy.execution.concurrency,
        process: policy.process,
        abortGraceMs: graceMs,
        ...(deps.operationClock !== undefined ? { clock: deps.operationClock } : {}),
      });
    const executionRuntime = invocation;
    const outputPerInput = worker.cheaper.model.workload?.outputPerInput;
    const resolved = await (async () => {
      if (deps.explicitModel === undefined) {
        return resolvePiModel({
          catalog: deps.catalog,
          requestedTier: worker.cheaper.model.tier,
          policy: policy.model,
          config: deps.modelConfig,
          ...(outputPerInput !== undefined ? { outputPerInput } : {}),
        });
      }
      const ref = parseModelRef(deps.explicitModel);
      if (ref === undefined) {
        throw new CheaperError(
          "INVALID_REQUEST",
          `invalid --model reference: ${deps.explicitModel} (expected <provider>/<id>)`,
          { details: { reason: "invalid-model-ref", model: deps.explicitModel } },
        );
      }
      return resolveExplicitPiModel({
        catalog: deps.catalog,
        ref,
        policy: { maxTier: policy.model.maxTier },
        config: deps.modelConfig,
        ...(outputPerInput !== undefined ? { outputPerInput } : {}),
      });
    })();

    const toolOptions = {
      process: policy.process,
      root: deps.root,
      symlinkPolicy: policy.filesystem.symlinks,
      journal,
      names: effectiveTools,
      maxToolCalls: policy.execution.maxToolCalls,
      onLimit: () => requestAbort("maxToolCalls"),
      denylist: createPathDenylist(policy.filesystem.denyPaths),
      writeScope: await extractWriteScope(
        worker,
        deps.input,
        deps.root,
        policy.filesystem.symlinks,
      ),
    };
    toolScope = invocation.createWorkerScope({
      ...toolOptions,
      ownerId: ownerId,
      handlers: createPiToolHandlers(toolOptions),
      signal: runAbort.signal,
      abortGraceMs: graceMs,
      ...(deps.execution?.parentLease !== undefined
        ? { parentLease: deps.execution.parentLease }
        : {}),
    });
    const sandboxed = createSandboxedTools({ ...toolOptions, runtime: toolScope });

    const { session } = await createWorkerSession({
      cwd: deps.root,
      model: resolved.model,
      ...(deps.modelRuntime ? { modelRuntime: deps.modelRuntime } : {}),
      systemPrompt: buildSystemPrompt(worker, sandboxed.toolNames),
      customTools: sandboxed.tools,
      toolNames: sandboxed.toolNames,
      maxOutputTokens: policy.output.maxTokens,
      transientRetries: policy.retry.transientAttempts,
      ...(deps.streamFn ? { streamFn: deps.streamFn } : {}),
    });

    const unsubscribe = session.subscribe((event) => {
      collector.handleSessionEvent(event);
      if (
        event.type === "turn_end" &&
        collector.snapshot().turns >= policy.execution.maxTurns &&
        // run が継続する場合のみ abort（stop/error で終わる最終 turn では記録しない）
        (event.message as { stopReason?: unknown }).stopReason === "toolUse"
      ) {
        requestAbort("maxTurns");
      }
    });

    async function buildMeta(result?: Record<string, unknown>): Promise<WorkerRunMeta> {
      const snap = collector.snapshot();
      // result があるときのみ returnedTokens を計測する。
      // 失敗・中断では result が無いため 0（sourceTokens は全量 avoided 扱い）。
      const sourceTokens = sandboxed.meter.sourceTokens;
      const returnedTokens = result ? countTextTokens(JSON.stringify(result)) : 0;
      return {
        model: {
          provider: resolved.info.provider,
          id: resolved.info.id,
          tier: resolved.tier,
          selection: resolved.selection,
          tierSource: resolved.tierSource,
          ...(resolved.auto !== undefined
            ? {
                effectiveCost: resolved.auto.effectiveCost,
                effectiveCostPercentile: resolved.auto.effectiveCostPercentile,
                workload: { outputPerInput: resolved.auto.outputPerInput },
              }
            : {}),
        },
        usage: snap.usage,
        turns: snap.turns,
        toolCalls: sandboxed.budget.toolCalls,
        rejectedToolCalls: sandboxed.budget.rejectedToolCalls,
        repairs,
        transientRetries: snap.transientRetries,
        changes: await journal.finalize(),
        execution: executionRuntime.executionMetrics(ownerId),
        savings: {
          sourceTokens,
          returnedTokens,
          avoidedContextTokens: Math.max(0, sourceTokens - returnedTokens),
          directlyComparable: isDirectlyComparableWorker(worker),
        },
      };
    }

    /** CheaperError にその時点の meta snapshot を載せて WorkerRunError 化する。 */
    const withRunMeta = async (err: CheaperError): Promise<WorkerRunError> => {
      if (err instanceof WorkerRunError) return err;
      return new WorkerRunError(err.code, err.message, {
        ...(err.details !== undefined ? { details: err.details } : {}),
        ...(err.cause !== undefined ? { cause: err.cause } : {}),
        runMeta: await buildMeta(),
      });
    };

    /** abort 系エラーは journal 非空なら PARTIAL_MUTATION で包む。 */
    const throwClassified = async (err: CheaperError): Promise<never> => {
      const meta = await buildMeta();
      if (meta.changes.totals.files > 0) {
        throw new WorkerRunError("PARTIAL_MUTATION", "worker aborted after partial mutations", {
          details: { changes: meta.changes },
          cause: err,
          runMeta: meta,
        });
      }
      throw await withRunMeta(err);
    };

    try {
      if (abortReason !== undefined) await throwClassified(abortError(abortReason));
      const outcome = await promptWithAbortGrace(session, buildUserPrompt(deps.input), {
        signal: runAbort.signal,
        graceMs,
      });

      if (abortReason !== undefined) {
        const reason = abortReason;
        await throwClassified(abortError(reason));
      }

      if (outcome.kind === "rejected") {
        const err = outcome.err;
        if (err instanceof CheaperError) throw await withRunMeta(err);
        throw new WorkerRunError(
          "PROVIDER_FAILURE",
          `session prompt failed: ${err instanceof Error ? err.message : String(err)}`,
          {
            cause: err instanceof Error ? err : undefined,
            runMeta: await buildMeta(),
          },
        );
      }

      // provider レベルの終了エラー（retry 尽き・abort 以外の error stopReason）
      const lastAssistant = lastAssistantStop(session.messages);
      if (lastAssistant?.stopReason === "error") {
        throw new WorkerRunError(
          "PROVIDER_FAILURE",
          `provider error: ${lastAssistant.errorMessage ?? "unknown"}`,
          {
            details: { errorMessage: lastAssistant.errorMessage },
            runMeta: await buildMeta(),
          },
        );
      }
      if (lastAssistant?.stopReason === "aborted") {
        await throwClassified(new CheaperError("ABORTED", "worker run aborted", {}));
      }

      const extracted = extractOutput(session.messages, worker.outputSchema);
      if (extracted.ok) {
        return {
          result: extracted.result,
          meta: await buildMeta(extracted.result),
        };
      }

      // repair session（retry・timeout は同じタイマー内）。
      // repair には外部 signal ではなく runAbort.signal を渡し、
      // timeout/limit を含む全 abort trigger を repair にも伝える。
      let result: Record<string, unknown>;
      try {
        const repaired = await repairOutput({
          cwd: deps.root,
          model: resolved.model,
          ...(deps.modelRuntime ? { modelRuntime: deps.modelRuntime } : {}),
          maxOutputTokens: policy.output.maxTokens,
          schema: worker.outputSchema,
          rawOutput: extracted.rawOutput,
          errors: extracted.errors,
          attempts: policy.output.repairAttempts,
          transientRetries: policy.retry.transientAttempts,
          collector,
          ...(deps.streamFn ? { streamFn: deps.streamFn } : {}),
          signal: runAbort.signal,
          abortGraceMs: graceMs,
          onAttempt: (n) => {
            repairs = n;
          },
        });
        result = repaired.result;
      } catch (e) {
        // repair 中に timeout/signal が落ちた場合は abort 分類を優先する
        if (abortReason !== undefined) {
          const reason = abortReason;
          await throwClassified(abortError(reason));
        }
        if (e instanceof CheaperError) throw await withRunMeta(e);
        throw e;
      }
      // repair が成功で解決しても、その間に abort が落ちていれば success を返さない
      if (abortReason !== undefined) {
        const reason = abortReason;
        await throwClassified(abortError(reason));
      }
      const meta = await buildMeta(result);
      return { result, meta };
    } finally {
      unsubscribe();
      session.dispose();
    }
  } catch (error) {
    if (!invocation) throw error;
    const err =
      error instanceof CheaperError
        ? error
        : new CheaperError(
            "INTERNAL_ERROR",
            error instanceof Error ? error.message : String(error),
            {
              cause: error instanceof Error ? error : undefined,
            },
          );
    throw new WorkerRunError(err.code, err.message, {
      ...(err.details !== undefined ? { details: err.details } : {}),
      ...(err.cause !== undefined ? { cause: err.cause } : {}),
      runMeta: {
        ...(err instanceof WorkerRunError ? err.runMeta : {}),
        execution: invocation.executionMetrics(ownerId),
      },
    });
  } finally {
    clearTimeout(timeout);
    deps.signal?.removeEventListener("abort", onSignalAbort);
    if (ownsInvocation) await invocation?.dispose();
    else await toolScope?.dispose();
  }
}
