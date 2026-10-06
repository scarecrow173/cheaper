import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";

export const REQUIRED_SCENARIO_IDS = Object.freeze([
  "single-simple-tool-call",
  "independent-fanout-worker-fanin",
  "tool-to-worker-chain",
  "process-and-analysis",
  "large-intermediate-process-analysis",
  "dynamic-vs-saved-workflow",
  "definition-vs-compact-interface",
]);

const manifestPath = new URL("../scenarios/scenarios.json", import.meta.url);

export async function loadScenarioManifest() {
  const manifest = JSON.parse(await readFile(fileURLToPath(manifestPath), "utf8"));
  if (manifest.schemaVersion !== 1 || !Array.isArray(manifest.scenarios)) {
    throw new TypeError("scenario manifest must have schemaVersion 1 and a scenarios array");
  }
  const ids = manifest.scenarios.map((scenario) => scenario.id);
  if (new Set(ids).size !== ids.length) throw new TypeError("scenario ids must be unique");
  const missing = REQUIRED_SCENARIO_IDS.filter((id) => !ids.includes(id));
  const unexpected = ids.filter((id) => !REQUIRED_SCENARIO_IDS.includes(id));
  if (missing.length || unexpected.length) {
    throw new TypeError(`scenario coverage mismatch; missing=${missing.join(",")}; unexpected=${unexpected.join(",")}`);
  }
  for (const scenario of manifest.scenarios) {
    if (typeof scenario.title !== "string" || !Array.isArray(scenario.arms) || !scenario.arms.some((arm) => arm.includes("workflow"))) {
      throw new TypeError(`${scenario.id} must have a title and a workflow comparison arm`);
    }
    if (scenario.workflow?.schemaVersion !== 1 || !scenario.workflow.jobs || !Object.keys(scenario.workflow.jobs).length) {
      throw new TypeError(`${scenario.id} must define a non-empty workflow`);
    }
    if (scenario.processPolicy && typeof scenario.processPolicy !== "object") {
      throw new TypeError(`${scenario.id} processPolicy must be an object`);
    }
  }
  return manifest.scenarios;
}
