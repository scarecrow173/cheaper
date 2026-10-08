import assert from "node:assert/strict";
import test from "node:test";
import {
  criticalPathMs,
  interfacePayloadBytes,
  measureWorkflowEnvelope,
} from "../lib/measurements.mjs";

test("critical path follows workflow dependencies and measured job durations", () => {
  const workflow = {
    jobs: {
      left: { kind: "tool", tool: "read", input: {} },
      right: { kind: "tool", tool: "read", input: {} },
      summary: { kind: "tool", tool: "read", needs: ["left", "right"], input: {} },
    },
  };
  const envelope = {
    jobs: [
      { id: "left", meta: { durationMs: 10 } },
      { id: "right", meta: { durationMs: 20 } },
      { id: "summary", meta: { durationMs: 5 } },
    ],
  };
  assert.equal(criticalPathMs(envelope, workflow), 25);
});

test("critical path is unavailable if a job duration is not observed", () => {
  assert.equal(
    criticalPathMs({ jobs: [{ id: "a", meta: { durationMs: null } }] }, { jobs: { a: {} } }),
    null,
  );
});

// Rust fake seam は prompt serialization / per-stream timing を観測
// できないため、この 2 dimension は null(未計測)で固定される。
test("workflow metrics report stream-side dimensions as unmeasured null", () => {
  const envelope = {
    schemaVersion: 1,
    workflow: true,
    ok: true,
    jobs: [{ id: "a", meta: {} }],
    meta: { artifacts: [], execution: { maxConcurrency: 2 } },
    result: {},
  };
  const metrics = measureWorkflowEnvelope(envelope, { jobs: { a: {} } });
  assert.equal(metrics.materializedPromptBytes, null);
  assert.equal(metrics.workerStreamConcurrency, null);
  assert.equal(metrics.toolOperationConcurrency, 2);
  assert.equal(metrics.maxConcurrency, 2);
});

test("compact interface bytes exclude variable command timing metadata", () => {
  const base = { ok: true, result: { schemaVersion: 1, inputs: {}, outputs: {} } };
  assert.equal(
    interfacePayloadBytes({ ...base, meta: { durationMs: 12 } }),
    interfacePayloadBytes({ ...base, meta: { durationMs: 87 } }),
  );
  assert.throws(() => interfacePayloadBytes({ ok: false, result: {} }), /successful result/);
});
