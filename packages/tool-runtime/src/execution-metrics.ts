import type { OperationRecord, OperationState } from "./operation.js";

/** 共有 ADR D3。時間は monotonic ms、overlap は speedup や token savings を意味しない。 */
export interface ExecutionMetrics {
  operations: number;
  byState: Record<OperationState, number>;
  /** toolName がない宣言操作は kind に集計する。 */
  byTool: Record<string, number>;
  maxConcurrency: number;
  operationMs: number;
  activeMs: number;
  overlapMs: number;
  queueMs: number;
}

/** snapshot は履歴の再生ではない。maxConcurrency は lifecycle の観測済み high-water。 */
export function collectExecutionMetrics(
  records: readonly OperationRecord[],
  snapshotAt: number,
  observedMaxConcurrency: number,
): ExecutionMetrics {
  const byState: ExecutionMetrics["byState"] = {
    queued: 0,
    running: 0,
    completed: 0,
    failed: 0,
    cancelled: 0,
    timed_out: 0,
  };
  const byTool = new Map<string, number>();
  const intervals: [number, number][] = [];
  let operationMs = 0;
  let queueMs = 0;
  for (const record of records) {
    byState[record.state]++;
    const tool = record.toolName ?? record.kind;
    byTool.set(tool, (byTool.get(tool) ?? 0) + 1);
    const queueEnd = Math.min(record.startedAt ?? record.finishedAt ?? snapshotAt, snapshotAt);
    queueMs += Math.max(0, queueEnd - record.createdAt);
    if (record.startedAt === undefined) continue;
    const end = Math.min(record.finishedAt ?? snapshotAt, snapshotAt);
    if (end <= record.startedAt) continue;
    operationMs += end - record.startedAt;
    intervals.push([record.startedAt, end]);
  }

  intervals.sort(([a], [b]) => a - b);
  let activeMs = 0;
  let coveredUntil = Number.NEGATIVE_INFINITY;
  for (const [start, end] of intervals) {
    activeMs += Math.max(0, end - Math.max(start, coveredUntil));
    coveredUntil = Math.max(coveredUntil, end);
  }
  return {
    operations: records.length,
    byState,
    byTool: Object.fromEntries(byTool),
    maxConcurrency: observedMaxConcurrency,
    operationMs,
    activeMs,
    overlapMs: operationMs - activeMs,
    queueMs,
  };
}
