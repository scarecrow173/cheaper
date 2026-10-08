export const RESULT_SCHEMA_VERSION = 1;

const isRecord = (value) => value !== null && typeof value === "object" && !Array.isArray(value);

function finiteCount(value, field) {
  if (value === undefined) return 0;
  if (typeof value !== "number" || !Number.isFinite(value) || value < 0) {
    throw new TypeError(`${field} must be a finite non-negative number`);
  }
  return value;
}

export function validateWorkflowEnvelope(envelope) {
  if (!isRecord(envelope) || envelope.schemaVersion !== 1) {
    throw new TypeError("workflow envelope must have schemaVersion 1");
  }
  if (envelope.workflow !== true) throw new TypeError("workflow discriminator is missing");
  if (envelope.ok !== true) throw new TypeError("workflow envelope must be successful");
  if (!Array.isArray(envelope.jobs)) throw new TypeError("workflow envelope jobs must be an array");
  for (const [index, job] of envelope.jobs.entries()) {
    if (!isRecord(job) || typeof job.id !== "string" || !isRecord(job.meta)) {
      throw new TypeError(`workflow envelope jobs[${index}] must include id and meta`);
    }
  }
  return envelope;
}

export function summarizeWorkflowEnvelope(envelope) {
  validateWorkflowEnvelope(envelope);
  const summary = {
    jobCount: envelope.jobs.length,
    workerJobCount: 0,
    toolOnlyJobCount: 0,
    workerTurns: 0,
    toolCalls: 0,
    usage: { input: 0, output: 0, cachedInput: 0, cachedOutput: 0, total: 0 },
  };

  for (const job of envelope.jobs) {
    const meta = job.meta;
    const worker = isRecord(meta.usage) || meta.turns !== undefined;
    if (worker) summary.workerJobCount += 1;
    else summary.toolOnlyJobCount += 1;
    summary.workerTurns += finiteCount(meta.turns, `${job.id}.meta.turns`);
    summary.toolCalls += finiteCount(meta.toolCalls, `${job.id}.meta.toolCalls`);
    if (isRecord(meta.usage)) {
      summary.usage.input += finiteCount(meta.usage.input, `${job.id}.meta.usage.input`);
      summary.usage.output += finiteCount(meta.usage.output, `${job.id}.meta.usage.output`);
      summary.usage.cachedInput += finiteCount(
        meta.usage.cacheRead,
        `${job.id}.meta.usage.cacheRead`,
      );
      summary.usage.cachedOutput += finiteCount(
        meta.usage.cacheWrite,
        `${job.id}.meta.usage.cacheWrite`,
      );
      summary.usage.total += finiteCount(
        meta.usage.totalTokens,
        `${job.id}.meta.usage.totalTokens`,
      );
    }
  }
  return summary;
}

export function summarizeWorkerEnvelope(envelope) {
  if (
    !isRecord(envelope) ||
    envelope.schemaVersion !== 1 ||
    envelope.ok !== true ||
    typeof envelope.worker !== "string" ||
    !isRecord(envelope.meta)
  ) {
    throw new TypeError("direct worker envelope must be successful and include worker metadata");
  }
  const usage = isRecord(envelope.meta.usage) ? envelope.meta.usage : {};
  return {
    workerTurns: finiteCount(envelope.meta.turns, "meta.turns"),
    toolCalls: finiteCount(envelope.meta.toolCalls, "meta.toolCalls"),
    usage: {
      input: finiteCount(usage.input, "meta.usage.input"),
      output: finiteCount(usage.output, "meta.usage.output"),
      cachedInput: finiteCount(usage.cacheRead, "meta.usage.cacheRead"),
      cachedOutput: finiteCount(usage.cacheWrite, "meta.usage.cacheWrite"),
      total: finiteCount(usage.totalTokens, "meta.usage.totalTokens"),
    },
  };
}

export function validateBenchmarkRecord(record) {
  if (!isRecord(record) || record.schemaVersion !== RESULT_SCHEMA_VERSION) {
    throw new TypeError(`benchmark record schemaVersion must be ${RESULT_SCHEMA_VERSION}`);
  }
  if (typeof record.scenarioId !== "string" || record.scenarioId.length === 0) {
    throw new TypeError("benchmark record scenarioId is required");
  }
  if (typeof record.arm !== "string" || record.arm.length === 0) {
    throw new TypeError("benchmark record arm is required");
  }
  if (
    !isRecord(record.parentUsage) ||
    !["observed", "unavailable"].includes(record.parentUsage.status)
  ) {
    throw new TypeError("benchmark record parentUsage must state observed or unavailable");
  }
  if (
    record.parentUsage.status === "unavailable" &&
    typeof record.parentUsage.reason !== "string"
  ) {
    throw new TypeError("unavailable parentUsage must include a reason");
  }
  if (record.parentUsage.status === "observed" && typeof record.parentUsage.source !== "string") {
    throw new TypeError("observed parentUsage must include a source");
  }
  if (!isRecord(record.metrics)) throw new TypeError("benchmark record metrics are required");
  return record;
}
