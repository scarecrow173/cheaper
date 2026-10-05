import { Buffer } from "node:buffer";
import { summarizeWorkflowEnvelope } from "./result-schema.mjs";

export function interfacePayloadBytes(envelope) {
  if (!envelope || envelope.ok !== true || !envelope.result || typeof envelope.result !== "object") {
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
  if (!Array.isArray(envelope?.jobs) || !workflow?.jobs || typeof workflow.jobs !== "object") return null;
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
      const parentMax = Math.max(0, ...[...dependencies].map((dependency) => pathMs.get(dependency)));
      pathMs.set(id, parentMax + durations.get(id));
      remaining.delete(id);
      progressed = true;
    }
    if (!progressed) throw new TypeError("workflow job dependency graph is cyclic or references an unknown job");
  }
  return Math.max(0, ...pathMs.values());
}

export function promptBytesFromJsonl(source) {
  if (!source) return 0;
  let total = 0;
  for (const [index, line] of source.split(/\r?\n/).entries()) {
    if (!line) continue;
    const row = JSON.parse(line);
    if (typeof row.bytes !== "number" || !Number.isFinite(row.bytes) || row.bytes < 0) {
      throw new TypeError(`prompt log line ${index + 1} has invalid bytes`);
    }
    total += row.bytes;
  }
  return total;
}

export function maxConcurrencyFromJsonl(source) {
  if (!source) return 0;
  const events = [];
  const opened = new Set();
  for (const [index, line] of source.split(/\r?\n/).entries()) {
    if (!line) continue;
    const event = JSON.parse(line);
    if (typeof event.id !== "string" || !["start", "end"].includes(event.phase) || !/^\d+$/.test(event.at)) {
      throw new TypeError(`timing log line ${index + 1} is invalid`);
    }
    if (event.phase === "start") {
      if (opened.has(event.id)) throw new TypeError(`timing log duplicates start for ${event.id}`);
      opened.add(event.id);
    } else {
      if (!opened.has(event.id)) throw new TypeError(`timing log has unmatched end for ${event.id}`);
      opened.delete(event.id);
    }
    events.push({ at: BigInt(event.at), phase: event.phase });
  }
  if (opened.size) throw new TypeError("timing log has unfinished prompt intervals");
  events.sort((left, right) => left.at < right.at ? -1 : left.at > right.at ? 1 : left.phase === "end" ? -1 : 1);
  let active = 0;
  let maximum = 0;
  for (const event of events) {
    active += event.phase === "start" ? 1 : -1;
    maximum = Math.max(maximum, active);
  }
  return maximum;
}

export function measureWorkflowEnvelope(envelope, workflow, promptLog = "", timingLog = "") {
  const summary = summarizeWorkflowEnvelope(envelope);
  const artifacts = Array.isArray(envelope.meta.artifacts) ? envelope.meta.artifacts : [];
  const artifactBytes = artifacts.reduce((total, artifact) => {
    if (typeof artifact.bytes !== "number" || !Number.isFinite(artifact.bytes) || artifact.bytes < 0) {
      throw new TypeError("workflow artifact metadata has invalid bytes");
    }
    return total + artifact.bytes;
  }, 0);
  const resultBytes = Buffer.byteLength(JSON.stringify(envelope.result));
  const execution = envelope.meta.execution;
  const workerStreamConcurrency = maxConcurrencyFromJsonl(timingLog);
  const toolOperationConcurrency = typeof execution?.maxConcurrency === "number" ? execution.maxConcurrency : null;
  return {
    ...summary,
    retainedBytes: resultBytes + artifactBytes,
    retainedResultBytes: resultBytes,
    artifactBytes,
    materializedPromptBytes: promptBytesFromJsonl(promptLog),
    criticalPathMs: criticalPathMs(envelope, workflow),
    maxConcurrency: Math.max(workerStreamConcurrency, toolOperationConcurrency ?? 0),
    workerStreamConcurrency,
    toolOperationConcurrency,
  };
}
