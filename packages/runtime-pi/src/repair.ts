import { CheaperError, type CompiledSchema } from "@cheaper/core";
import type { Api, Model } from "@earendil-works/pi-ai";
import type { ModelRuntime } from "@earendil-works/pi-coding-agent";
import { extractOutput, lastAssistantStop, type SchemaValidationError } from "./output.js";
import { promptWithAbortGrace } from "./prompt-grace.js";
import { createWorkerSession, type StreamFnLike } from "./session.js";
import type { UsageCollector } from "./usage.js";

export interface RepairOptions {
  cwd: string;
  model: Model<Api>;
  modelRuntime?: ModelRuntime;
  maxOutputTokens: number;
  schema: CompiledSchema;
  /** 初回の失敗した生出力。 */
  rawOutput: string;
  errors: SchemaValidationError[];
  /** policy.output.repairAttempts。 */
  attempts: number;
  /** Pi auto-retry 回数（policy.retry.transientAttempts）。省略時 0。 */
  transientRetries?: number;
  /** 指定時は各 repair session のイベントを usage 集計に合算する。 */
  collector?: UsageCollector;
  streamFn?: StreamFnLike;
  /** run 全体の abort signal。abort 時は残り attempt を打ち切る。 */
  signal?: AbortSignal;
  /** abort 後に prompt() settle を待つ猶予 ms（既定 5000）。 */
  abortGraceMs?: number;
  /** 各 attempt 開始時に呼ばれる。失敗・中断時も meta.repairs に消費回数を計上するためのフック。 */
  onAttempt?: (attempt: number) => void;
}

export interface RepairResult {
  result: Record<string, unknown>;
  /** 消費した repair 回数（meta.repairs に計上）。 */
  repairs: number;
}

const MAX_DETAIL_ERRORS = 20;
const MAX_DETAIL_RAW = 2000;

function repairSystemPrompt(schema: CompiledSchema): string {
  return [
    "You are a JSON repair worker. Return ONLY a JSON object that conforms to the",
    "schema below. No prose, no markdown, no code fences.",
    "",
    "Schema:",
    JSON.stringify(schema.schema, null, 2),
  ].join("\n");
}

function repairUserPrompt(rawOutput: string, errors: SchemaValidationError[]): string {
  const lines = errors.map((e) => `- ${e.instancePath || "/"}: ${e.message}`);
  return [
    "The previous output failed schema validation.",
    "",
    "Raw output:",
    rawOutput,
    "",
    "Validation errors:",
    ...(lines.length > 0 ? lines : ["- (no details)"]),
    "",
    "Return the corrected JSON object only.",
  ].join("\n");
}

/**
 * 出力 repair session（DESIGN-0004/0010）。
 * 各 attempt は tool 無しの新規 session で、前回 raw output + Ajv errors を渡す。
 * prompt は main session と同じ abort grace 付きで、prompt rejection・
 * `stopReason === "error"` は PROVIDER_FAILURE に正規化する。
 * attempts 使い切りで CheaperError INVALID_OUTPUT（details は errors 先頭 20 件・
 * rawOutput 先頭 2000 文字）。abort 分類は呼び出し側（run）の責務。
 */
export async function repairOutput(opts: RepairOptions): Promise<RepairResult> {
  let rawOutput = opts.rawOutput;
  let errors = opts.errors;
  const model: Model<Api> = {
    ...opts.model,
    samplingParams: {
      ...opts.model.samplingParams,
      response_format: {
        type: "json_schema",
        json_schema: { name: "worker_output", schema: opts.schema.schema },
      },
    },
  };
  for (let attempt = 1; attempt <= opts.attempts; attempt++) {
    if (opts.signal?.aborted) break;
    opts.onAttempt?.(attempt);
    const { session } = await createWorkerSession({
      cwd: opts.cwd,
      model,
      ...(opts.modelRuntime ? { modelRuntime: opts.modelRuntime } : {}),
      systemPrompt: repairSystemPrompt(opts.schema),
      noTools: "all",
      maxOutputTokens: opts.maxOutputTokens,
      transientRetries: opts.transientRetries ?? 0,
      ...(opts.streamFn ? { streamFn: opts.streamFn } : {}),
    });
    const collector = opts.collector;
    const unsubscribe =
      collector === undefined
        ? undefined
        : session.subscribe((event) => collector.handleSessionEvent(event));
    let extracted: ReturnType<typeof extractOutput> | undefined;
    try {
      const outcome = await promptWithAbortGrace(session, repairUserPrompt(rawOutput, errors), {
        ...(opts.signal !== undefined ? { signal: opts.signal } : {}),
        graceMs: opts.abortGraceMs ?? 5000,
      });
      if (outcome.kind === "rejected") {
        const err = outcome.err;
        if (err instanceof CheaperError) throw err;
        throw new CheaperError(
          "PROVIDER_FAILURE",
          `repair prompt failed: ${err instanceof Error ? err.message : String(err)}`,
          { ...(err instanceof Error ? { cause: err } : {}) },
        );
      }
      // abort 済みなら extraction を行わず抜ける（分類は呼び出し側の abort reason）
      if (!opts.signal?.aborted) {
        const lastAssistant = lastAssistantStop(session.messages);
        if (lastAssistant?.stopReason === "error") {
          throw new CheaperError(
            "PROVIDER_FAILURE",
            `provider error: ${lastAssistant.errorMessage ?? "unknown"}`,
            { details: { errorMessage: lastAssistant.errorMessage } },
          );
        }
        extracted = extractOutput(session.messages, opts.schema);
      }
    } finally {
      unsubscribe?.();
      await session.dispose();
    }
    if (extracted === undefined) break;
    if (extracted.ok) return { result: extracted.result, repairs: attempt };
    rawOutput = extracted.rawOutput;
    errors = extracted.errors;
  }
  throw new CheaperError("INVALID_OUTPUT", "worker output failed schema validation", {
    details: {
      errors: errors.slice(0, MAX_DETAIL_ERRORS),
      rawOutput: rawOutput.slice(0, MAX_DETAIL_RAW),
    },
  });
}
