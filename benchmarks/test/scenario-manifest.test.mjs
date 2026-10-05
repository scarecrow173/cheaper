import test from "node:test";
import assert from "node:assert/strict";
import { loadScenarioManifest } from "../lib/scenarios.mjs";

const required = [
  "single-simple-tool-call",
  "independent-fanout-worker-fanin",
  "tool-to-worker-chain",
  "process-and-analysis",
  "large-intermediate-process-analysis",
  "dynamic-vs-saved-workflow",
  "definition-vs-compact-interface",
];

test("scenario manifest defines every required benchmark arm and a runnable workflow", async () => {
  const scenarios = await loadScenarioManifest();
  assert.deepEqual(scenarios.map(({ id }) => id).sort(), [...required].sort());
  for (const scenario of scenarios) {
    assert.equal(scenario.workflow.schemaVersion, 1, scenario.id);
    assert.ok(Object.keys(scenario.workflow.jobs).length > 0, scenario.id);
    assert.ok(scenario.arms.some((arm) => arm.includes("workflow")), scenario.id);
  }
  const fanout = scenarios.find(({ id }) => id === "independent-fanout-worker-fanin");
  assert.equal(fanout.workflow.jobs.summary.needs.length, 2);
  assert.equal(fanout.workflowMaxJobs, 3);
  assert.ok(scenarios.find(({ id }) => id === "process-and-analysis").processPolicy);
});
