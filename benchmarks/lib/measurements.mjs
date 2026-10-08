import { Buffer } from "node:buffer";
import { summarizeWorkflowEnvelope } from "./result-schema.mjs";

export function interfacePayloadBytes(envelope) {
  if (envelope?.ok !== true || !envelope.result || typeof envelope.result !== "object") {
    throw new TypeError("workflow interface envelope must have a successful result object");
  }
  return Buffer.byteLength(JSON.stringify(envelope.result));
}

function bindingDependencies(value, output = new Set()) {
  if (Array.isArray(value)) {
    for (const child of value) bindingDependencies(child, output);
  } else if (value && typeof value === "object") {
    if (typeof value.$from === "string") output.add(value.$from);
    else for (const child of Object.values(value)) bindingDependencies(child, output);
  }
  return output;
}

export function criticalPathMs(envelope, workflow) {
  if (!Array.isArray(envelope?.jobs) || !workflow?.jobs || typeof workflow.jobs !== "object")
    return null;
  const outcomes = new Map(envelope.jobs.map((job) => [job.id, job]));
  const durations = new Map();
  for (const id of Object.keys(workflow.jobs)) {
    const duration = outcomes.get(id)?.meta?.durationMs;
    if (typeof duration !== "number" || !Number.isFinite(duration) || duration < 0) return null;
    durations.set(id, duration);
  }

  const remaining = new Set(durations.keys());
  const pathMs = new Map();
  while (remaining.size) {
    let progressed = false;
    for (const id of [...remaining]) {
      const job = workflow.jobs[id];
      const dependencies = new Set([
        ...(Array.isArray(job.needs) ? job.needs : []),
        ...bindingDependencies(job.input),
      ]);
      if ([...dependencies].some((dependency) => !pathMs.has(dependency))) continue;
      const parentMax = Math.max(
        0,
        ...[...dependencies].map((dependency) => pathMs.get(dependency)),
      );
      pathMs.set(id, parentMax + durations.get(id));
      remaining.delete(id);
      progressed = true;
    }
    if (!progressed)
      throw new TypeError("workflow job dependency graph is cyclic or references an unknown job");
  }
  return Math.max(0, ...pathMs.values());
}

// Rust fake seam(`CHEAPER_TEST_FAKE_STREAM` の fixture JSON)には prompt
// serialization / per-stream timing の観測 hook が無い — TS 版は module
// seam が process 内で CHEAPER_BENCH_PROMPT_LOG / CHEAPER_BENCH_TIMING_LOG
// を書いていた。この 2 dimension は「未計測」を意味する null で報告する
// (0 にすると測定値と区別が付かない)。`maxConcurrency` は engine が
// `meta.execution.maxConcurrency` で計上する tool operation 側のみ。

export function measureWorkflowEnvelope(envelope, workflow) {
  const summary = summarizeWorkflowEnvelope(envelope);
  const artifacts = Array.isArray(envelope.meta.artifacts) ? envelope.meta.artifacts : [];
  const artifactBytes = artifacts.reduce((total, artifact) => {
    if (
      typeof artifact.bytes !== "number" ||
      !Number.isFinite(artifact.bytes) ||
      artifact.bytes < 0
    ) {
      throw new TypeError("workflow artifact metadata has invalid bytes");
    }
    return total + artifact.bytes;
  }, 0);
  const resultBytes = Buffer.byteLength(JSON.stringify(envelope.result));
  const execution = envelope.meta.execution;
  const toolOperationConcurrency =
    typeof execution?.maxConcurrency === "number" ? execution.maxConcurrency : null;
  return {
    ...summary,
    retainedBytes: resultBytes + artifactBytes,
    retainedResultBytes: resultBytes,
    artifactBytes,
    materializedPromptBytes: null,
    criticalPathMs: criticalPathMs(envelope, workflow),
    maxConcurrency: toolOperationConcurrency,
    workerStreamConcurrency: null,
    toolOperationConcurrency,
  };
}
