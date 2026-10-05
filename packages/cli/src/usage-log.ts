import fs from "node:fs";
import path from "node:path";
import type { WorkerSavings, WorkerUsage } from "@cheaper/runtime-pi";
import type { CliEnvelope, WorkflowEnvelope } from "./envelope.js";

/** usage.jsonl の 1 行レコード（ADR-0025）。 */
export interface UsageLogRecord {
  schemaVersion: 1;
  /** ISO 8601 timestamp。 */
  ts: string;
  kind: "run" | "count";
  /** count record では null。 */
  worker: string | null;
  /** canonical 化済み workspace root。 */
  root: string;
  ok: boolean;
  durationMs: number | null;
  model?: { provider: string; id: string; tier: string };
  usage?: WorkerUsage;
  /**
   * 親コンテキスト offload の実測（run record のみ、optional。
   * 旧 schema の record / count record には存在しない）。
   */
  savings?: WorkerSavings;
  /** run record のみ。旧 schema の record には存在しない。 */
  turns?: number;
  toolCalls?: number;
  error?: { code: string };
  result?: { path?: string; tokens?: number; bytes?: number };
}

export const USAGE_LOG_FILENAME = "usage.jsonl";

/**
 * CliEnvelope から計測 record を構築する（DESIGN-0027）。
 * 対象は usage を持つ run envelope（success + meta.usage 付き
 * error）と count-tokens の utility envelope。それ以外は null。
 */
export function buildUsageRecord(
  env: CliEnvelope,
  ctx: { root: string; now?: (() => Date) | undefined },
): UsageLogRecord | null {
  if (isWorkflowEnvelope(env)) return null;
  const ts = (ctx.now ?? (() => new Date()))().toISOString();
  const base = {
    schemaVersion: 1 as const,
    ts,
    root: ctx.root,
    durationMs: env.meta.durationMs ?? null,
  };

  if (env.ok) {
    if (env.worker === null) {
      // utility envelope: count-tokens のみ記録（tokens を持つもの）。
      const r = env.result;
      if (typeof r.tokens !== "number") return null;
      return {
        ...base,
        kind: "count",
        worker: null,
        ok: true,
        result: {
          ...(typeof r.path === "string" ? { path: r.path } : {}),
          tokens: r.tokens,
          ...(typeof r.bytes === "number" ? { bytes: r.bytes } : {}),
        },
      };
    }
    return {
      ...base,
      kind: "run",
      worker: env.worker,
      ok: true,
      model: env.meta.model,
      usage: env.meta.usage,
      turns: env.meta.turns,
      toolCalls: env.meta.toolCalls,
      // savings は runtime が提供したときだけ載せる（後方互換）
      ...(env.meta.savings ? { savings: env.meta.savings } : {}),
    };
  }

  // error envelope: runMeta 由来の usage がある run 失敗のみ記録。
  if (env.meta.usage === null || env.meta.model === null) return null;
  return {
    ...base,
    kind: "run",
    worker: env.worker,
    ok: false,
    model: env.meta.model,
    usage: env.meta.usage,
    ...(env.meta.turns !== null ? { turns: env.meta.turns } : {}),
    ...(env.meta.toolCalls !== null ? { toolCalls: env.meta.toolCalls } : {}),
    ...(env.meta.savings ? { savings: env.meta.savings } : {}),
    error: { code: env.error.code },
  };
}

function isWorkflowEnvelope(env: CliEnvelope): env is WorkflowEnvelope {
  return "workflow" in env && env.workflow;
}

/** usage.jsonl へ 1 行 append（dir は必要に応じて作成）。 */
export function appendUsageLog(opts: { userConfigDir: string; record: UsageLogRecord }): void {
  fs.mkdirSync(opts.userConfigDir, { recursive: true });
  fs.appendFileSync(
    path.join(opts.userConfigDir, USAGE_LOG_FILENAME),
    `${JSON.stringify(opts.record)}\n`,
  );
}

/**
 * best-effort 記録（ADR-0025）。build → append を行い、失敗しても
 * exit code / stdout を変えず、quiet でなければ stderr に 1 行
 * diagnostic を出すのみ。
 */
export function recordUsage(opts: {
  env: CliEnvelope;
  root: string;
  userConfigDir: string;
  stderr: { write(s: string): unknown };
  quiet: boolean;
  now?: () => Date;
}): void {
  try {
    const record = buildUsageRecord(opts.env, { root: opts.root, now: opts.now });
    if (record === null) return;
    appendUsageLog({ userConfigDir: opts.userConfigDir, record });
  } catch (err) {
    if (!opts.quiet) {
      const msg = err instanceof Error ? err.message : String(err);
      opts.stderr.write(`usage-log: ${msg}\n`);
    }
  }
}
