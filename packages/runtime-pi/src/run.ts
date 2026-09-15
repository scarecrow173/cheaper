import {
  CheaperError,
  checkCapabilities,
  type ErrorCode,
  type ModelConfig,
  type RuntimePolicy,
  type Tier,
  type WorkerDefinition,
} from "@cheaper/core";
import type { ModelRuntime } from "@earendil-works/pi-coding-agent";
import type { ChangesSummary } from "./journal.js";
import { MutationJournal } from "./journal.js";
import { type ModelCatalog, resolvePiModel } from "./model-runtime.js";
import { extractOutput } from "./output.js";
import { repairOutput } from "./repair.js";
import { createWorkerSession, type StreamFnLike } from "./session.js";
import { createSandboxedTools } from "./tools/registry.js";
import { UsageCollector, type WorkerUsage } from "./usage.js";

export interface WorkerRunMeta {
  model: { provider: string; id: string; tier: Tier };
  usage: WorkerUsage;
  turns: number;
  toolCalls: number;
  rejectedToolCalls: number;
  repairs: number;
  transientRetries: number;
  changes: ChangesSummary;
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
  /** session 用 ModelRuntime。通常は catalog と同じ実体。 */
  modelRuntime?: ModelRuntime;
  /** テスト用 stream seam。session.agent.streamFunction の差し替え元。 */
  streamFn?: StreamFnLike;
  /** 外部 abort（CLI の SIGINT 等）。 */
  signal?: AbortSignal;
  /** abort 後に prompt()  settle を待つ猶予 ms（既定 5000）。 */
  abortGraceMs?: number;
}

type AbortReason = "timeout" | "signal" | "maxTurns" | "maxToolCalls";

const ABORT_CODES: Record<AbortReason, ErrorCode> = {
  timeout: "TIMEOUT",
  signal: "ABORTED",
  maxTurns: "MAX_TURNS_EXCEEDED",
  maxToolCalls: "MAX_TOOL_CALLS_EXCEEDED",
};

function buildSystemPrompt(worker: WorkerDefinition, toolNames: string[]): string {
  const contract = [
    "",
    "## Output contract",
    "",
    "- The final message must be a single JSON object that validates against the output schema below.",
    "- Do not wrap the JSON in markdown code fences or any surrounding prose.",
    "- All file paths are workspace-relative POSIX paths.",
    `- Available tools: ${toolNames.length > 0 ? toolNames.join(", ") : "none"}`,
    "",
    "Output schema:",
    JSON.stringify(worker.outputSchema.schema, null, 2),
  ];
  return worker.instructions.trimEnd() + "\n" + contract.join("\n") + "\n";
}

function buildUserPrompt(input: unknown): string {
  return `Task input (JSON):\n${JSON.stringify(input, null, 2)}`;
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

  const resolved = await resolvePiModel({
    catalog: deps.catalog,
    requestedTier: worker.cheaper.model.tier,
    policy: policy.model,
    config: deps.modelConfig,
  });

  const journal = new MutationJournal();
  const collector = new UsageCollector();
  const graceMs = deps.abortGraceMs ?? 5000;

  let abortReason: AbortReason | undefined;
  let resolveGrace: () => void = () => {};
  const gracePromise = new Promise<void>((r) => {
    resolveGrace = r;
  });

  const sandboxed = createSandboxedTools({
    root: deps.root,
    symlinkPolicy: policy.filesystem.symlinks,
    journal,
    names: effectiveTools,
    maxToolCalls: policy.execution.maxToolCalls,
    onLimit: () => requestAbort("maxToolCalls"),
  });

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

  function requestAbort(reason: AbortReason): void {
    if (abortReason !== undefined) return;
    abortReason = reason;
    void session.abort();
    setTimeout(resolveGrace, graceMs);
  }

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

  const timeout = setTimeout(() => requestAbort("timeout"), policy.execution.timeoutMs);
  const onSignalAbort = () => requestAbort("signal");
  deps.signal?.addEventListener("abort", onSignalAbort, { once: true });
  if (deps.signal?.aborted) requestAbort("signal");

  /** abort 系エラーは journal 非空なら PARTIAL_MUTATION で包む。 */
  const throwClassified = async (err: CheaperError): Promise<never> => {
    const changes = await journal.finalize();
    if (changes.totals.files > 0) {
      throw new CheaperError("PARTIAL_MUTATION", "worker aborted after partial mutations", {
        details: { changes },
        cause: err,
      });
    }
    throw err;
  };

  try {
    const promptOutcome = session.prompt(buildUserPrompt(deps.input)).then(
      () => ({ kind: "settled" as const }),
      (err: unknown) => ({ kind: "rejected" as const, err }),
    );
    const outcome = await Promise.race([
      promptOutcome,
      gracePromise.then(() => ({ kind: "grace" as const })),
    ]);

    if (abortReason !== undefined) {
      const reason = abortReason;
      await throwClassified(
        new CheaperError(ABORT_CODES[reason], `worker run aborted: ${reason}`, {
          details: { reason },
        }),
      );
    }

    if (outcome.kind === "rejected") {
      const err = outcome.err;
      if (err instanceof CheaperError) throw err;
      throw new CheaperError(
        "PROVIDER_FAILURE",
        `session prompt failed: ${err instanceof Error ? err.message : String(err)}`,
        { cause: err instanceof Error ? err : undefined },
      );
    }

    // provider レベルの終了エラー（retry 尽き・abort 以外の error stopReason）
    const lastAssistant = lastAssistantStop(session.messages);
    if (lastAssistant?.stopReason === "error") {
      throw new CheaperError(
        "PROVIDER_FAILURE",
        `provider error: ${lastAssistant.errorMessage ?? "unknown"}`,
        { details: { errorMessage: lastAssistant.errorMessage } },
      );
    }
    if (lastAssistant?.stopReason === "aborted") {
      await throwClassified(new CheaperError("ABORTED", "worker run aborted", {}));
    }

    const extracted = extractOutput(session.messages, worker.outputSchema);
    if (extracted.ok) {
      return {
        result: extracted.result,
        meta: await buildMeta(),
      };
    }

    // repair session（retry・timeout は同じタイマー内）
    let repairs = 0;
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
        ...(deps.streamFn ? { streamFn: deps.streamFn } : {}),
        ...(deps.signal ? { signal: deps.signal } : {}),
      });
      repairs = repaired.repairs;
      result = repaired.result;
    } catch (e) {
      // repair 中に timeout/signal が落ちた場合は abort 分類を優先する
      if (abortReason !== undefined) {
        const reason = abortReason;
        await throwClassified(
          new CheaperError(ABORT_CODES[reason], `worker run aborted: ${reason}`, {
            details: { reason },
          }),
        );
      }
      throw e;
    }
    const meta = await buildMeta();
    meta.repairs = repairs;
    return { result, meta };
  } finally {
    clearTimeout(timeout);
    deps.signal?.removeEventListener("abort", onSignalAbort);
    unsubscribe();
    session.dispose();
  }

  async function buildMeta(): Promise<WorkerRunMeta> {
    const snap = collector.snapshot();
    return {
      model: { provider: resolved.info.provider, id: resolved.info.id, tier: resolved.tier },
      usage: snap.usage,
      turns: snap.turns,
      toolCalls: sandboxed.budget.toolCalls,
      rejectedToolCalls: sandboxed.budget.rejectedToolCalls,
      repairs: 0,
      transientRetries: snap.transientRetries,
      changes: await journal.finalize(),
    };
  }
}

function lastAssistantStop(
  messages: readonly unknown[],
): { stopReason: string | undefined; errorMessage: string | undefined } | undefined {
  for (let i = messages.length - 1; i >= 0; i--) {
    const m = messages[i];
    if (typeof m === "object" && m !== null && (m as { role?: unknown }).role === "assistant") {
      const mm = m as { stopReason?: unknown; errorMessage?: unknown };
      return {
        stopReason: typeof mm.stopReason === "string" ? mm.stopReason : undefined,
        errorMessage: typeof mm.errorMessage === "string" ? mm.errorMessage : undefined,
      };
    }
  }
  return undefined;
}
