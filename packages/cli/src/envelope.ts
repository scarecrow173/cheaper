import { CheaperError, isCheaperError } from "@cheaper/core";
import type { ChangesSummary, WorkerRunMeta, WorkerUsage } from "@cheaper/runtime-pi";

export const SCHEMA_VERSION = 1 as const;

/** error envelope の meta。取得できなかった field は null（DESIGN-0005）。 */
export interface ErrorEnvelopeMeta {
  durationMs: number | null;
  model: WorkerRunMeta["model"] | null;
  usage: WorkerUsage | null;
  turns: number | null;
  toolCalls: number | null;
  rejectedToolCalls: number | null;
  repairs: number | null;
  transientRetries: number | null;
  changes: ChangesSummary | null;
}

export interface SuccessEnvelope {
  schemaVersion: typeof SCHEMA_VERSION;
  ok: true;
  worker: string;
  result: Record<string, unknown>;
  meta: { durationMs: number } & WorkerRunMeta;
}

/** worker run を伴わない utility command（count-tokens 等）の success envelope。 */
export interface UtilitySuccessEnvelope {
  schemaVersion: typeof SCHEMA_VERSION;
  ok: true;
  worker: null;
  result: Record<string, unknown>;
  meta: { durationMs: number };
}

export interface ErrorEnvelope {
  schemaVersion: typeof SCHEMA_VERSION;
  ok: false;
  worker: string | null;
  error: {
    code: string;
    message: string;
    exitCode: number;
    retryable: boolean;
    details?: Record<string, unknown>;
    cause?: { code: string; message: string };
  };
  meta: ErrorEnvelopeMeta;
}

export type CliEnvelope = SuccessEnvelope | UtilitySuccessEnvelope | ErrorEnvelope;

export function successEnvelope(opts: {
  worker: string;
  result: Record<string, unknown>;
  meta: WorkerRunMeta;
  durationMs: number;
}): SuccessEnvelope {
  return {
    schemaVersion: SCHEMA_VERSION,
    ok: true,
    worker: opts.worker,
    result: opts.result,
    meta: { durationMs: opts.durationMs, ...opts.meta },
  };
}

export function utilitySuccessEnvelope(opts: {
  result: Record<string, unknown>;
  durationMs: number;
}): UtilitySuccessEnvelope {
  return {
    schemaVersion: SCHEMA_VERSION,
    ok: true,
    worker: null,
    result: opts.result,
    meta: { durationMs: opts.durationMs },
  };
}

function errorMeta(durationMs: number | null, err: unknown): ErrorEnvelopeMeta {
  let changes: ChangesSummary | null = null;
  let runMeta: (Partial<WorkerRunMeta> & { durationMs?: number }) | undefined;
  if (isCheaperError(err)) {
    const c = err.details?.changes;
    if (typeof c === "object" && c !== null) changes = c as ChangesSummary;
    // WorkerRunError（run 失敗時の meta snapshot 付き CheaperError）から meta を埋める
    const rm = (err as { runMeta?: unknown }).runMeta;
    if (typeof rm === "object" && rm !== null) {
      runMeta = rm as Partial<WorkerRunMeta> & { durationMs?: number };
    }
  }
  return {
    durationMs,
    model: runMeta?.model ?? null,
    usage: runMeta?.usage ?? null,
    turns: runMeta?.turns ?? null,
    toolCalls: runMeta?.toolCalls ?? null,
    rejectedToolCalls: runMeta?.rejectedToolCalls ?? null,
    repairs: runMeta?.repairs ?? null,
    transientRetries: runMeta?.transientRetries ?? null,
    changes: runMeta?.changes ?? changes,
  };
}

/**
 * error envelope を組み立てる（DESIGN-0005）。
 * CheaperError は code/exitCode/retryable/details/cause をそのまま移す。
 * それ以外は INTERNAL_ERROR(70)。cause は CheaperError のときのみ
 * {code,message} として出す（envelope schema の形を保つため）。
 */
export function errorEnvelope(
  err: unknown,
  opts: { worker: string | null; durationMs: number | null },
): ErrorEnvelope {
  const e: CheaperError = isCheaperError(err)
    ? err
    : new CheaperError("INTERNAL_ERROR", err instanceof Error ? err.message : String(err), {
        cause: err instanceof Error ? err : undefined,
      });

  const error: ErrorEnvelope["error"] = {
    code: e.code,
    message: e.message,
    exitCode: e.exitCode,
    retryable: e.retryable,
  };
  if (e.details !== undefined) error.details = e.details;
  if (isCheaperError(e.cause)) {
    error.cause = { code: e.cause.code, message: e.cause.message };
  }
  return {
    schemaVersion: SCHEMA_VERSION,
    ok: false,
    worker: opts.worker,
    error,
    meta: errorMeta(opts.durationMs, e),
  };
}

/** stdout 契約: envelope 1 JSON + 末尾改行 1 つのみ。 */
export function envelopeJson(env: CliEnvelope): string {
  return `${JSON.stringify(env)}\n`;
}
