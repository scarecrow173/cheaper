import assert from "node:assert/strict";
import test from "node:test";
import {
  criticalPathMs,
  interfacePayloadBytes,
  maxConcurrencyFromJsonl,
  promptBytesFromJsonl,
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

test("prompt bytes sum actual serialized fake-stream contexts", () => {
  assert.equal(promptBytesFromJsonl('{"bytes":10}\n{"bytes":7}\n'), 17);
  assert.throws(() => promptBytesFromJsonl("{bad}\n"), /JSON/);
});

test("max concurrency uses observed fake-stream interval overlap", () => {
  const log = [
    { id: "a", phase: "start", at: "1" },
    { id: "b", phase: "start", at: "2" },
    { id: "a", phase: "end", at: "3" },
    { id: "b", phase: "end", at: "4" },
  ]
    .map((entry) => JSON.stringify(entry))
    .join("\n");
  assert.equal(maxConcurrencyFromJsonl(log), 2);
  assert.throws(() => maxConcurrencyFromJsonl('{"id":"a","phase":"end","at":"2"}'), /unmatched/);
});

test("compact interface bytes exclude variable command timing metadata", () => {
  const base = { ok: true, result: { schemaVersion: 1, inputs: {}, outputs: {} } };
  assert.equal(
    interfacePayloadBytes({ ...base, meta: { durationMs: 12 } }),
    interfacePayloadBytes({ ...base, meta: { durationMs: 87 } }),
  );
  assert.throws(() => interfacePayloadBytes({ ok: false, result: {} }), /successful result/);
});
