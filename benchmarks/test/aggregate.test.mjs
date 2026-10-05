import test from "node:test";
import assert from "node:assert/strict";
import { aggregateBenchmarkResult } from "../lib/aggregate.mjs";

function record(scenarioId, arm, wallTimeMs, metrics) {
  return {
    schemaVersion: 1,
    scenarioId,
    arm,
    parentUsage: { status: "unavailable", reason: "external parent telemetry unavailable" },
    wallTimeMs,
    metrics,
  };
}

test("aggregate excludes warmups, preserves unavailable parent usage, and computes measured deltas", () => {
  const result = aggregateBenchmarkResult({
    scenarioIds: ["independent-fanout-worker-fanin"],
    observations: [
      { ...record("independent-fanout-worker-fanin", "workflow", 999, { workerTurns: 90, retainedBytes: 999, materializedPromptBytes: 99, toolCalls: 0, usage: { input: 999 } }), warmup: true },
      { ...record("independent-fanout-worker-fanin", "workflow", 10, { workerTurns: 3, retainedBytes: 40, materializedPromptBytes: 50, toolCalls: 0, usage: { input: 80, output: 4, total: 84 } }), warmup: false },
      { ...record("independent-fanout-worker-fanin", "workflow", 20, { workerTurns: 3, retainedBytes: 40, materializedPromptBytes: 50, toolCalls: 0, usage: { input: 80, output: 4, total: 84 } }), warmup: false },
      { ...record("independent-fanout-worker-fanin", "direct-worker-sequence", 30, { workerTurns: 3, retainedBytes: 60, materializedPromptBytes: 70, toolCalls: 0, usage: { input: 100, output: 6, total: 106 } }), warmup: false },
    ],
  });

  const workflow = result.arms.find(({ arm }) => arm === "workflow");
  assert.equal(workflow.observationCount, 2);
  assert.equal(workflow.medianWallTimeMs, 15);
  assert.equal(workflow.p95WallTimeMs, 20);
  assert.deepEqual(workflow.parentUsage, { status: "unavailable", reason: "external parent telemetry unavailable" });
  const pair = result.comparisons.find(({ right }) => right === "direct-worker-sequence");
  assert.equal(pair.deltas.usageInput, -20);
  assert.equal(pair.deltas.retainedBytes, -20);
});

test("aggregate leaves variable deterministic metrics unavailable instead of averaging them", () => {
  const result = aggregateBenchmarkResult({
    scenarioIds: ["independent-fanout-worker-fanin"],
    observations: [10, 20].map((wallTimeMs, input) => record("independent-fanout-worker-fanin", "workflow", wallTimeMs, {
      workerTurns: 1, retainedBytes: 5, materializedPromptBytes: 6, toolCalls: 0, usage: { input, output: 1, total: input + 1 },
    })),
  });
  assert.equal(result.arms[0].stableMetrics.usageInput, null);
});
