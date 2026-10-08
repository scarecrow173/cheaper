import { Buffer } from "node:buffer";
import { spawn } from "node:child_process";
import { randomUUID } from "node:crypto";
import { copyFile, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { arch, cpus, platform, release, tmpdir } from "node:os";
import path from "node:path";
import { performance } from "node:perf_hooks";
import {
  benchmarkEnvironment,
  cliCommandLine,
  countUsageRecords,
  prepareWorkspace,
  repositoryRoot,
  runCli,
} from "./cli.mjs";
import { interfacePayloadBytes, measureWorkflowEnvelope } from "./measurements.mjs";
import {
  summarizeWorkerEnvelope,
  validateBenchmarkRecord,
  validateWorkflowEnvelope,
} from "./result-schema.mjs";

const unavailableParentUsage = Object.freeze({
  status: "unavailable",
  reason: "no parent-agent telemetry is exposed to the benchmark runner",
});

function environmentSnapshot() {
  return {
    os: `${platform()} ${release()}`,
    architecture: arch(),
    node: process.version,
    cpuCount: cpus().length,
  };
}

async function runNative(executable, args, { cwd, env, input } = {}) {
  const started = performance.now();
  const child = spawn(executable, args, { cwd, env, stdio: ["pipe", "pipe", "pipe"] });
  let stdout = "";
  let stderr = "";
  child.stdout.setEncoding("utf8");
  child.stderr.setEncoding("utf8");
  child.stdout.on("data", (chunk) => {
    stdout += chunk;
  });
  child.stderr.on("data", (chunk) => {
    stderr += chunk;
  });
  if (input !== undefined) child.stdin.end(input);
  else child.stdin.end();
  const exitCode = await new Promise((resolve, reject) => {
    child.on("error", reject);
    child.on("close", resolve);
  });
  if (exitCode !== 0)
    throw new Error(`${executable} ${args.join(" ")} exited ${exitCode}: ${stderr || stdout}`);
  return { stdout, stderr, wallTimeMs: performance.now() - started };
}

async function writeWorkflow(scenario, workspace, arm) {
  const workflowPath = path.join(workspace, "workflow.json");
  if (arm === "saved-workflow") {
    const savedPath = path.join(
      repositoryRoot,
      "benchmarks",
      "scenarios",
      scenario.savedWorkflowFile,
    );
    await copyFile(savedPath, workflowPath);
    return {
      workflowPath,
      generationMs: 0,
      definitionBytes: Buffer.byteLength(await readFile(workflowPath)),
    };
  }
  const started = performance.now();
  const source = JSON.stringify(scenario.workflow, null, 2);
  await writeFile(workflowPath, `${source}\n`, "utf8");
  return {
    workflowPath,
    generationMs: performance.now() - started,
    definitionBytes: Buffer.byteLength(`${source}\n`),
  };
}

function workflowCommand(workflowPath, workspace) {
  return ["workflow", "run", path.basename(workflowPath), "--cwd", workspace, "--quiet"];
}

async function runWorkflow(scenario, arm, workspace, envInfo) {
  const { workflowPath, generationMs, definitionBytes } = await writeWorkflow(
    scenario,
    workspace,
    arm,
  );
  let interfaceBytes = null;
  let interfaceWallTimeMs = null;
  if (scenario.arms.includes("saved-workflow") || scenario.arms.includes("dynamic-workflow")) {
    const iface = await runCli(
      ["workflow", "interface", path.basename(workflowPath), "--cwd", workspace, "--quiet"],
      {
        cwd: workspace,
        env: envInfo.env,
        fakeStream: envInfo.fakeStream,
      },
    );
    interfaceBytes = interfacePayloadBytes(iface.envelope);
    interfaceWallTimeMs = iface.wallTimeMs;
  }
  const started = performance.now();
  const cli = await runCli(workflowCommand(workflowPath, workspace), {
    cwd: workspace,
    env: envInfo.env,
    fakeStream: envInfo.fakeStream,
  });
  const wallTimeMs = performance.now() - started;
  const envelope = validateWorkflowEnvelope(cli.envelope);
  const metrics = measureWorkflowEnvelope(envelope, scenario.workflow);
  const directUsageRecords = await countUsageRecords(envInfo.configDir);
  return {
    schemaVersion: 1,
    runId: randomUUID(),
    scenarioId: scenario.id,
    scenarioTitle: scenario.title,
    arm,
    mode: "workflow",
    parentUsage: unavailableParentUsage,
    fixtureRevision: "seeded-v1",
    seed: "seeded-v1",
    environment: environmentSnapshot(),
    command: cliCommandLine(workflowCommand(workflowPath, workspace)),
    workflowDefinitionBytes: definitionBytes,
    parentVisibleInterfaceBytes: interfaceBytes,
    interfaceDiscoveryWallTimeMs: interfaceWallTimeMs,
    discoveryMode: arm === "saved-workflow" ? "saved-definition" : "generated-definition",
    definitionGenerationMs: generationMs,
    wallTimeMs,
    directUsageRecordCount: directUsageRecords,
    metrics: {
      ...metrics,
      wallTimeMs,
      cliReportedDurationMs: envelope.meta.durationMs,
      nodeCount: metrics.jobCount,
      criticalPathMs: metrics.criticalPathMs,
    },
  };
}

async function runDirectFilesystem(scenario, workspace) {
  const filename = path.join(workspace, scenario.manual.path);
  const started = performance.now();
  const content = await readFile(filename);
  const wallTimeMs = performance.now() - started;
  return {
    schemaVersion: 1,
    runId: randomUUID(),
    scenarioId: scenario.id,
    scenarioTitle: scenario.title,
    arm: "direct-filesystem",
    mode: "direct-tool-operation",
    parentUsage: unavailableParentUsage,
    fixtureRevision: "seeded-v1",
    seed: "seeded-v1",
    environment: environmentSnapshot(),
    command: "node:fs/promises.readFile",
    workflowDefinitionBytes: null,
    parentVisibleInterfaceBytes: null,
    discoveryMode: "no-workflow-definition",
    wallTimeMs,
    directUsageRecordCount: 0,
    metrics: {
      nodeCount: 1,
      workerJobCount: 0,
      toolOnlyJobCount: 0,
      workerTurns: 0,
      toolCalls: 0,
      usage: null,
      retainedBytes: content.byteLength,
      materializedPromptBytes: 0,
      criticalPathMs: null,
      maxConcurrency: 1,
    },
  };
}

async function runDirectWorker(_scenario, workspace, envInfo, task, profile, options = {}) {
  const input = {
    task,
    ...(options.paths?.length ? { paths: options.paths } : {}),
    ...(options.hints?.length ? { hints: options.hints } : {}),
  };
  const result = await runCli(["run", "bulk-reader", "--cwd", workspace, "--quiet"], {
    cwd: workspace,
    env: envInfo.env,
    input: JSON.stringify(input),
    fakeStream: {
      ...envInfo.fakeStream,
      responseProfiles: [profile],
      ...(options.toolCallPath ? { toolCallPath: options.toolCallPath } : {}),
    },
  });
  const metric = summarizeWorkerEnvelope(result.envelope);
  return {
    envelope: result.envelope,
    metric,
    wallTimeMs: result.wallTimeMs,
    // Rust fake seam は prompt serialization を観測できないため null。
    promptBytes: null,
    outputBytes: Buffer.byteLength(JSON.stringify(result.envelope.result)),
  };
}

async function runDirectWorkerSequence(scenario, workspace, envInfo) {
  const manual = scenario.manual;
  const hints = [];
  let retainedBytes = 0;
  let processWallTimeMs = 0;
  const readPaths = [...(manual.readPath ? [manual.readPath] : []), ...(manual.readPaths ?? [])];
  for (const readPath of readPaths) {
    const source = await readFile(path.join(workspace, readPath));
    hints.push(source.toString("utf8"));
    retainedBytes += source.byteLength;
  }
  if (manual.processScript) {
    const result = await runNative(process.execPath, [manual.processScript], {
      cwd: workspace,
      env: envInfo.env,
    });
    hints.push(result.stdout);
    retainedBytes += Buffer.byteLength(result.stdout);
    processWallTimeMs = result.wallTimeMs;
  }

  const priorResults = [];
  const childRuns = [];
  for (let index = 0; index < manual.tasks.length; index += 1) {
    const responseProfile = manual.responseProfiles?.[index] ?? "compact";
    const task =
      manual.forwardFullResults && priorResults.length
        ? `${manual.tasks[index]}\nPrior full Worker results:\n${priorResults.join("\n")}`
        : manual.tasks[index];
    const child = await runDirectWorker(scenario, workspace, envInfo, task, responseProfile, {
      hints,
      paths: manual.toolCallPath ? [manual.toolCallPath] : [],
      ...(index === 0 && manual.toolCallPath ? { toolCallPath: manual.toolCallPath } : {}),
    });
    priorResults.push(JSON.stringify(child.envelope.result));
    childRuns.push(child);
  }
  const lastEnvelope = childRuns.at(-1)?.envelope;
  const outputBytes = childRuns.reduce((total, child) => total + child.outputBytes, 0);
  const usage = childRuns.reduce(
    (total, child) => ({
      input: total.input + child.metric.usage.input,
      output: total.output + child.metric.usage.output,
      cachedInput: total.cachedInput + child.metric.usage.cachedInput,
      cachedOutput: total.cachedOutput + child.metric.usage.cachedOutput,
      total: total.total + child.metric.usage.total,
    }),
    { input: 0, output: 0, cachedInput: 0, cachedOutput: 0, total: 0 },
  );
  const usageRecords = await countUsageRecords(envInfo.configDir);
  return {
    schemaVersion: 1,
    runId: randomUUID(),
    scenarioId: scenario.id,
    scenarioTitle: scenario.title,
    arm: "direct-worker-sequence",
    mode: "scripted-direct-cli-sequence",
    parentUsage: unavailableParentUsage,
    fixtureRevision: "seeded-v1",
    seed: "seeded-v1",
    environment: environmentSnapshot(),
    command: `${cliCommandLine(["run", "bulk-reader"])} (one invocation per manual task)`,
    workflowDefinitionBytes: null,
    parentVisibleInterfaceBytes: null,
    discoveryMode: "no-workflow-definition",
    wallTimeMs: processWallTimeMs + childRuns.reduce((total, child) => total + child.wallTimeMs, 0),
    directUsageRecordCount: usageRecords,
    directInvocationTurnsProxy: childRuns.length,
    metrics: {
      nodeCount: childRuns.length,
      workerJobCount: childRuns.length,
      toolOnlyJobCount: readPaths.length + (manual.processScript ? 1 : 0),
      directToolOperations: readPaths.length,
      workerTurns: childRuns.reduce((total, child) => total + child.metric.workerTurns, 0),
      toolCalls: childRuns.reduce((total, child) => total + child.metric.toolCalls, 0),
      usage,
      retainedBytes: retainedBytes + outputBytes,
      // Rust fake seam は prompt serialization を観測できないため null。
      materializedPromptBytes: null,
      criticalPathMs: null,
      maxConcurrency: 1,
      childInvocations: childRuns.map((child, index) => ({
        index,
        wallTimeMs: child.wallTimeMs,
        promptBytes: child.promptBytes,
        toolCalls: child.metric.toolCalls,
        turns: child.metric.workerTurns,
        usage: child.metric.usage,
      })),
    },
    ...(lastEnvelope
      ? { finalResultBytes: Buffer.byteLength(JSON.stringify(lastEnvelope.result)) }
      : {}),
  };
}

async function runFullDefinition(scenario, workspace, arm) {
  const definitionPath = path.join(workspace, "workflow.json");
  await writeFile(definitionPath, `${JSON.stringify(scenario.workflow, null, 2)}\n`, "utf8");
  const started = performance.now();
  const definition = await readFile(definitionPath);
  return {
    schemaVersion: 1,
    runId: randomUUID(),
    scenarioId: scenario.id,
    scenarioTitle: scenario.title,
    arm,
    mode: "full-definition-read",
    parentUsage: unavailableParentUsage,
    fixtureRevision: "seeded-v1",
    seed: "seeded-v1",
    environment: environmentSnapshot(),
    command: "node:fs/promises.readFile(workflow.json)",
    workflowDefinitionBytes: definition.byteLength,
    parentVisibleInterfaceBytes: null,
    discoveryMode: "private-dag-read",
    wallTimeMs: performance.now() - started,
    directUsageRecordCount: 0,
    metrics: { definitionReadBytes: definition.byteLength },
  };
}

async function runCompactInterface(scenario, workspace, envInfo) {
  const { workflowPath } = await writeWorkflow(scenario, workspace, "dynamic-workflow");
  const result = await runCli(
    ["workflow", "interface", path.basename(workflowPath), "--cwd", workspace, "--quiet"],
    {
      cwd: workspace,
      env: envInfo.env,
      fakeStream: envInfo.fakeStream,
    },
  );
  return {
    schemaVersion: 1,
    runId: randomUUID(),
    scenarioId: scenario.id,
    scenarioTitle: scenario.title,
    arm: "compact-interface",
    mode: "public-interface-discovery",
    parentUsage: unavailableParentUsage,
    fixtureRevision: "seeded-v1",
    seed: "seeded-v1",
    environment: environmentSnapshot(),
    command: cliCommandLine(["workflow", "interface", path.basename(workflowPath)]),
    workflowDefinitionBytes: Buffer.byteLength(JSON.stringify(scenario.workflow, null, 2)) + 1,
    parentVisibleInterfaceBytes: interfacePayloadBytes(result.envelope),
    discoveryMode: "public-interface-only",
    wallTimeMs: result.wallTimeMs,
    directUsageRecordCount: 0,
    metrics: { interfaceBytes: Buffer.byteLength(result.stdout) },
  };
}

async function runArm(scenario, arm, workspace, envInfo) {
  if (arm === "workflow" || arm === "dynamic-workflow" || arm === "saved-workflow") {
    const result = await runWorkflow(scenario, arm, workspace, envInfo);
    if (arm === "saved-workflow" && result.discoveryMode !== "saved-definition")
      throw new Error("saved workflow arm lost its definition mode");
    return result;
  }
  if (arm === "direct-filesystem") return runDirectFilesystem(scenario, workspace);
  if (arm === "direct-worker-tool-call") {
    const child = await runDirectWorker(
      scenario,
      workspace,
      envInfo,
      scenario.manual.tasks[0],
      "compact",
      {
        paths: [scenario.manual.toolCallPath],
        toolCallPath: scenario.manual.toolCallPath,
      },
    );
    return {
      schemaVersion: 1,
      runId: randomUUID(),
      scenarioId: scenario.id,
      scenarioTitle: scenario.title,
      arm,
      mode: "single-direct-worker-with-one-tool-call",
      parentUsage: unavailableParentUsage,
      fixtureRevision: "seeded-v1",
      seed: "seeded-v1",
      environment: environmentSnapshot(),
      command: `${cliCommandLine(["run", "bulk-reader"])} (one read tool call)`,
      workflowDefinitionBytes: null,
      parentVisibleInterfaceBytes: null,
      discoveryMode: "no-workflow-definition",
      wallTimeMs: child.wallTimeMs,
      directUsageRecordCount: await countUsageRecords(envInfo.configDir),
      directInvocationTurnsProxy: 1,
      metrics: {
        nodeCount: 1,
        workerJobCount: 1,
        toolOnlyJobCount: 0,
        workerTurns: child.metric.workerTurns,
        toolCalls: child.metric.toolCalls,
        usage: child.metric.usage,
        retainedBytes: child.outputBytes,
        materializedPromptBytes: child.promptBytes,
        criticalPathMs: null,
        maxConcurrency: 1,
      },
    };
  }
  if (arm === "direct-worker-sequence")
    return runDirectWorkerSequence(scenario, workspace, envInfo);
  if (arm === "full-definition") return runFullDefinition(scenario, workspace, arm);
  if (arm === "compact-interface") return runCompactInterface(scenario, workspace, envInfo);
  throw new TypeError(`unsupported benchmark arm: ${arm}`);
}

export async function runScenarioArm(scenario, arm) {
  const tempDirectory = await mkdtemp(path.join(tmpdir(), "cheaper-workflow-benchmark-"));
  const workspace = path.resolve(tempDirectory);
  const expectedPrefix = path.resolve(tmpdir(), "cheaper-workflow-benchmark-");
  if (!workspace.startsWith(expectedPrefix))
    throw new Error("temporary benchmark workspace escaped the temp directory");
  try {
    await prepareWorkspace(scenario, workspace);
    const envInfo = benchmarkEnvironment(workspace, {
      responseProfiles: scenario.responseProfiles,
      responseDelayMs: scenario.responseDelayMs,
    });
    await mkdir(envInfo.configDir, { recursive: true });
    if (scenario.workflowMaxJobs !== undefined) {
      await writeFile(
        path.join(envInfo.configDir, "policy.yaml"),
        JSON.stringify({
          version: 1,
          workflow: { maxJobs: scenario.workflowMaxJobs },
        }),
        "utf8",
      );
    }
    const result = await runArm(scenario, arm, workspace, envInfo);
    return validateBenchmarkRecord(result);
  } finally {
    await rm(workspace, { recursive: true, force: true });
  }
}
