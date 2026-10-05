import test from "node:test";
import assert from "node:assert/strict";
import {
  RESULT_SCHEMA_VERSION,
  summarizeWorkflowEnvelope,
  summarizeWorkerEnvelope,
  validateWorkflowEnvelope,
  validateBenchmarkRecord,
} from "../lib/result-schema.mjs";

test("workflow envelope summary takes usage and execution counters from jobs[].meta", () => {
  const envelope = {
    schemaVersion: 1,
    workflow: true,
    ok: true,
    jobs: [
      { id: "worker-a", status: "succeeded", meta: { turns: 2, toolCalls: 3, usage: { input: 8, output: 4, cacheRead: 1, cacheWrite: 0, totalTokens: 13 } } },
      { id: "tool-b", status: "succeeded", meta: { toolCalls: 1 } },
    ],
    meta: { durationMs: 21 },
  };

  assert.deepEqual(validateWorkflowEnvelope(envelope).jobs, envelope.jobs);
  assert.deepEqual(summarizeWorkflowEnvelope(envelope), {
    jobCount: 2,
    workerJobCount: 1,
    toolOnlyJobCount: 1,
    workerTurns: 2,
    toolCalls: 4,
    usage: { input: 8, output: 4, cachedInput: 1, cachedOutput: 0, total: 13 },
  });
});

test("direct run usage is measured from its Worker envelope once", () => {
  const envelope = {
    schemaVersion: 1,
    ok: true,
    worker: "bulk-reader",
    meta: { turns: 1, toolCalls: 1, usage: { input: 12, output: 3, cacheRead: 2, cacheWrite: 0, totalTokens: 13 } },
  };
  assert.deepEqual(summarizeWorkerEnvelope(envelope), {
    workerTurns: 1,
    toolCalls: 1,
    usage: { input: 12, output: 3, cachedInput: 2, cachedOutput: 0, total: 13 },
  });
  assert.throws(() => summarizeWorkerEnvelope({ ...envelope, ok: false }), /successful/);
});

test("workflow envelope validation fails closed for unsuccessful or non-workflow output", () => {
  assert.throws(() => validateWorkflowEnvelope({ schemaVersion: 1, ok: true, jobs: [] }), /workflow/);
  assert.throws(() => validateWorkflowEnvelope({ schemaVersion: 1, workflow: true, ok: false, jobs: [] }), /successful/);
  assert.throws(() => validateWorkflowEnvelope({ schemaVersion: 1, workflow: true, ok: true, jobs: {} }), /jobs/);
});

test("benchmark record requires explicit parent usage provenance and a version", () => {
  const record = {
    schemaVersion: RESULT_SCHEMA_VERSION,
    scenarioId: "single-tool-call",
    arm: "workflow",
    parentUsage: { status: "unavailable", reason: "parent telemetry not provided" },
    metrics: { retainedBytes: 0, materializedPromptBytes: 0 },
  };
  assert.equal(validateBenchmarkRecord(record), record);
  assert.throws(() => validateBenchmarkRecord({ ...record, parentUsage: undefined }), /parentUsage/);
  assert.throws(() => validateBenchmarkRecord({ ...record, schemaVersion: 2 }), /schemaVersion/);
});
