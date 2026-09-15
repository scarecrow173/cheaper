import { CheaperError, type CompiledSchema } from "@cheaper/core";
import type { Api, Model } from "@earendil-works/pi-ai";
import type { ModelRuntime } from "@earendil-works/pi-coding-agent";
import { extractOutput, type SchemaValidationError } from "./output.js";
import { createWorkerSession, type StreamFnLike } from "./session.js";

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
  streamFn?: StreamFnLike;
  /** run 全体の abort signal。abort 時は残り attempt を打ち切る。 */
  signal?: AbortSignal;
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
 * 出力 repair session（DESIGN-0004）。
 * 各 attempt は tool 無しの新規 session で、前回 raw output + Ajv errors を渡す。
 * attempts 使い切りで CheaperError INVALID_OUTPUT（details は errors 先頭 20 件・
 * rawOutput 先頭 2000 文字）。abort 分類は呼び出し側（run）の責務。
 */
export async function repairOutput(opts: RepairOptions): Promise<RepairResult> {
  let rawOutput = opts.rawOutput;
  let errors = opts.errors;
  for (let attempt = 1; attempt <= opts.attempts; attempt++) {
    if (opts.signal?.aborted) break;
    const { session } = await createWorkerSession({
      cwd: opts.cwd,
      model: opts.model,
      ...(opts.modelRuntime ? { modelRuntime: opts.modelRuntime } : {}),
      systemPrompt: repairSystemPrompt(opts.schema),
      noTools: "all",
      maxOutputTokens: opts.maxOutputTokens,
      transientRetries: 0,
      ...(opts.streamFn ? { streamFn: opts.streamFn } : {}),
    });
    const unlink = opts.signal
      ? () => {
          void session.abort();
        }
      : undefined;
    if (unlink) opts.signal?.addEventListener("abort", unlink, { once: true });
    let extracted: ReturnType<typeof extractOutput>;
    try {
      await session.prompt(repairUserPrompt(rawOutput, errors));
      extracted = extractOutput(session.messages, opts.schema);
    } finally {
      if (unlink) opts.signal?.removeEventListener("abort", unlink);
      await session.dispose();
    }
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
