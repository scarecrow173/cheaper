import { mkdir, writeFile } from "node:fs/promises";
import { randomUUID } from "node:crypto";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { loadScenarioManifest } from "./lib/scenarios.mjs";
import { runScenarioArm } from "./lib/run-scenario.mjs";
import { aggregateBenchmarkResult } from "./lib/aggregate.mjs";

const root = fileURLToPath(new URL("..", import.meta.url));
const resultsDirectory = path.join(root, "benchmarks", "results");

function argumentsFrom(argv) {
  const result = { scenario: null, repetitions: 15, warmups: 2, output: null };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--scenario") result.scenario = argv[++index];
    else if (arg === "--repetitions") result.repetitions = Number(argv[++index]);
    else if (arg === "--warmups") result.warmups = Number(argv[++index]);
    else if (arg === "--output") result.output = argv[++index];
    else throw new TypeError(`unknown argument: ${arg}`);
  }
  if (!Number.isSafeInteger(result.repetitions) || result.repetitions < 1) throw new TypeError("--repetitions must be a positive integer");
  if (!Number.isSafeInteger(result.warmups) || result.warmups < 0) throw new TypeError("--warmups must be a non-negative integer");
  return result;
}

const options = argumentsFrom(process.argv.slice(2));
const scenarios = await loadScenarioManifest();
const selected = options.scenario ? scenarios.filter((scenario) => scenario.id === options.scenario) : scenarios;
if (!selected.length) throw new TypeError(`unknown scenario: ${options.scenario}`);

const observations = [];
for (const scenario of selected) {
  for (const arm of scenario.arms) {
    for (let index = 0; index < options.warmups + options.repetitions; index += 1) {
      const record = await runScenarioArm(scenario, arm);
      if (index >= options.warmups) observations.push({ ...record, repetition: index - options.warmups + 1, warmup: false });
      else observations.push({ ...record, repetition: index + 1, warmup: true });
      process.stdout.write(`\r${scenario.id} / ${arm}: ${index + 1}/${options.warmups + options.repetitions}`);
    }
    process.stdout.write("\n");
  }
}

const result = {
  schemaVersion: 1,
  runId: randomUUID(),
  createdAt: new Date().toISOString(),
  fixtureRevision: "seeded-v1",
  warmupCount: options.warmups,
  repetitionCount: options.repetitions,
  scenarioIds: selected.map(({ id }) => id),
  observations,
};
result.aggregate = aggregateBenchmarkResult(result);
await mkdir(resultsDirectory, { recursive: true });
const output = options.output ? path.resolve(options.output) : path.join(resultsDirectory, `${result.runId}.json`);
await writeFile(output, `${JSON.stringify(result, null, 2)}\n`, { flag: "wx" });
process.stdout.write(`Saved ${observations.length} observations to ${output}\n`);
