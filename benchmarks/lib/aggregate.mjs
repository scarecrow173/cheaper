import { validateBenchmarkRecord } from "./result-schema.mjs";

const COMPARISONS = Object.freeze([
  ["single-simple-tool-call", "workflow", "direct-filesystem"],
  ["single-simple-tool-call", "workflow", "direct-worker-tool-call"],
  ["independent-fanout-worker-fanin", "workflow", "direct-worker-sequence"],
  ["tool-to-worker-chain", "workflow", "direct-worker-sequence"],
  ["process-and-analysis", "workflow", "direct-worker-sequence"],
  ["large-intermediate-process-analysis", "workflow", "direct-worker-sequence"],
  ["dynamic-vs-saved-workflow", "dynamic-workflow", "saved-workflow"],
  ["dynamic-vs-saved-workflow", "saved-workflow", "direct-worker-sequence"],
  ["definition-vs-compact-interface", "full-definition", "compact-interface"],
]);

const comparableFields = [
  "jobCount",
  "workerJobCount",
  "toolOnlyJobCount",
  "workerTurns",
  "toolCalls",
  "retainedBytes",
  "materializedPromptBytes",
  "maxConcurrency",
  "criticalPathMs",
  "directToolOperations",
];
const usageFields = ["input", "output", "total"];
const sizeFields = [
  "workflowDefinitionBytes",
  "parentVisibleInterfaceBytes",
  "definitionReadBytes",
  "directInvocationTurnsProxy",
];

function percentile(values, fraction) {
  const sorted = [...values].sort((left, right) => left - right);
  return sorted[Math.max(0, Math.ceil(sorted.length * fraction) - 1)];
}

function median(values) {
  const sorted = [...values].sort((left, right) => left - right);
  const middle = Math.floor(sorted.length / 2);
  return sorted.length % 2 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2;
}

function stableValue(records, select) {
  const values = records
    .map(select)
    .filter((value) => typeof value === "number" && Number.isFinite(value));
  return values.length === records.length && values.every((value) => value === values[0])
    ? values[0]
    : null;
}

function summarizeArm(records) {
  const times = records
    .map(({ wallTimeMs }) => wallTimeMs)
    .filter((value) => Number.isFinite(value));
  if (times.length !== records.length)
    throw new TypeError("every benchmark arm record must include finite wallTimeMs");
  const metrics = Object.fromEntries(
    comparableFields.map((field) => [field, stableValue(records, ({ metrics }) => metrics[field])]),
  );
  for (const field of usageFields)
    metrics[`usage${field[0].toUpperCase()}${field.slice(1)}`] = stableValue(
      records,
      ({ metrics }) => metrics.usage?.[field],
    );
  for (const field of sizeFields) {
    metrics[field] = stableValue(
      records,
      (record) =>
        record[field] ??
        record.metrics[field] ??
        (field === "workflowDefinitionBytes" ? record.metrics.definitionReadBytes : undefined) ??
        (field === "parentVisibleInterfaceBytes" ? record.metrics.interfaceBytes : undefined),
    );
  }
  return {
    observationCount: records.length,
    parentUsage: records.every(({ parentUsage }) => parentUsage.status === "unavailable")
      ? { status: "unavailable", reason: records[0].parentUsage.reason }
      : {
          status: "observed",
          sources: [
            ...new Set(
              records
                .filter(({ parentUsage }) => parentUsage.status === "observed")
                .map(({ parentUsage }) => parentUsage.source),
            ),
          ],
        },
    medianWallTimeMs: median(times),
    p95WallTimeMs: percentile(times, 0.95),
    stableMetrics: metrics,
  };
}

function pairIndex(arms) {
  return new Map(arms.map((arm) => [`${arm.scenarioId}\0${arm.arm}`, arm]));
}

function comparison([scenarioId, leftName, rightName], index) {
  const left = index.get(`${scenarioId}\0${leftName}`);
  const right = index.get(`${scenarioId}\0${rightName}`);
  if (!left || !right)
    return {
      scenarioId,
      left: leftName,
      right: rightName,
      status: "incomplete",
      reason: "one or both comparison arms are absent",
    };
  const deltas = {};
  for (const field of comparableFields) {
    const a = left.stableMetrics[field];
    const b = right.stableMetrics[field];
    deltas[field] = a === null || b === null ? null : a - b;
  }
  for (const field of usageFields) {
    const key = `usage${field[0].toUpperCase()}${field.slice(1)}`;
    const a = left.stableMetrics[key];
    const b = right.stableMetrics[key];
    deltas[key] = a === null || b === null ? null : a - b;
  }
  for (const field of sizeFields) {
    const a = left.stableMetrics[field];
    const b = right.stableMetrics[field];
    deltas[field] = a === null || b === null ? null : a - b;
  }
  deltas.wallTimeMs = left.medianWallTimeMs - right.medianWallTimeMs;
  if (scenarioId === "definition-vs-compact-interface" && leftName === "full-definition") {
    const definitionBytes = left.stableMetrics.workflowDefinitionBytes;
    const interfaceBytes = right.stableMetrics.parentVisibleInterfaceBytes;
    deltas.parentVisibleReductionBytes =
      definitionBytes === null || interfaceBytes === null ? null : definitionBytes - interfaceBytes;
    deltas.parentVisibleReductionRatio =
      definitionBytes === null || interfaceBytes === null || definitionBytes === 0
        ? null
        : (definitionBytes - interfaceBytes) / definitionBytes;
  }
  return { scenarioId, left: leftName, right: rightName, status: "measured", deltas };
}

/** Aggregate only measured records; workflow envelopes and usage.jsonl arms remain separate. */
export function aggregateBenchmarkResult(result) {
  if (!result || !Array.isArray(result.observations))
    throw new TypeError("benchmark result observations are required");
  const groups = new Map();
  for (const record of result.observations) {
    validateBenchmarkRecord(record);
    if (record.warmup === true) continue;
    const key = `${record.scenarioId}\0${record.arm}`;
    const group = groups.get(key) ?? {
      scenarioId: record.scenarioId,
      arm: record.arm,
      records: [],
    };
    group.records.push(record);
    groups.set(key, group);
  }
  const arms = [...groups.values()].map(({ scenarioId, arm, records }) => ({
    scenarioId,
    arm,
    ...summarizeArm(records),
  }));
  const index = pairIndex(arms);
  return {
    schemaVersion: 1,
    arms,
    comparisons: COMPARISONS.filter(([scenario]) => result.scenarioIds?.includes(scenario)).map(
      (pair) => comparison(pair, index),
    ),
  };
}
