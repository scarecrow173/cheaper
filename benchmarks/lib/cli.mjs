import { spawn } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { performance } from "node:perf_hooks";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const repositoryRoot = fileURLToPath(new URL("../..", import.meta.url));
export const cliEntry = path.join(repositoryRoot, "packages", "cli", "dist", "main.js");
export const fakeStreamFixture = path.join(repositoryRoot, "benchmarks", "fixtures", "fake-worker-stream.mjs");

export async function prepareWorkspace(scenario, directory) {
  await mkdir(directory, { recursive: true });
  for (const [relative, content] of Object.entries(scenario.files ?? {})) {
    const target = path.resolve(directory, relative);
    if (!target.startsWith(`${directory}${path.sep}`)) throw new TypeError(`fixture path escapes workspace: ${relative}`);
    await mkdir(path.dirname(target), { recursive: true });
    await writeFile(target, content, "utf8");
  }
  const projectConfig = path.join(directory, ".cheaper");
  await mkdir(projectConfig, { recursive: true });
  await writeFile(path.join(projectConfig, "models.yaml"), "version: 1\noverrides:\n  cheaper-fake/fake-model: low\n", "utf8");
  if (scenario.processPolicy) {
    const policy = {};
    const processPolicy = structuredClone(scenario.processPolicy);
    for (const task of Object.values(processPolicy.tasks ?? {})) {
      task.command = task.command.map((arg) => arg === "{node}" ? process.execPath : arg);
    }
    policy.process = processPolicy;
    await writeFile(path.join(projectConfig, "policy.yaml"), JSON.stringify({ version: 1, ...policy }), "utf8");
  }
}

export function benchmarkEnvironment(workspace, options = {}) {
  const configDir = path.join(workspace, ".benchmark-config");
  const env = {
    PATH: process.env.PATH ?? "",
    HOME: workspace,
    USERPROFILE: workspace,
    PI_OFFLINE: "1",
    NODE_ENV: "test",
    CHEAPER_CONFIG_DIR: configDir,
    CHEAPER_TEST_FAKE_STREAM: fakeStreamFixture,
    CHEAPER_BENCH_PROMPT_LOG: options.promptLog ?? path.join(configDir, "prompt-contexts.jsonl"),
    CHEAPER_BENCH_TIMING_LOG: options.timingLog ?? path.join(configDir, "prompt-intervals.jsonl"),
    CHEAPER_BENCH_RESPONSE_PROFILES: (options.responseProfiles ?? ["compact"]).join(","),
    CHEAPER_BENCH_RESPONSE_DELAY_MS: String(options.responseDelayMs ?? 0),
    ...(options.toolCallPath ? { CHEAPER_BENCH_TOOL_CALL_PATH: options.toolCallPath } : {}),
    ...(process.env.SystemRoot ? { SystemRoot: process.env.SystemRoot } : {}),
    ...(process.env.TEMP ? { TEMP: process.env.TEMP } : {}),
    ...(process.env.TMP ? { TMP: process.env.TMP } : {}),
  };
  return { env, configDir, promptLog: env.CHEAPER_BENCH_PROMPT_LOG, timingLog: env.CHEAPER_BENCH_TIMING_LOG };
}

export async function runCli(args, options) {
  const started = performance.now();
  const child = spawn(process.execPath, [cliEntry, ...args], {
    cwd: options.cwd,
    env: options.env,
    stdio: ["pipe", "pipe", "pipe"],
  });
  let stdout = "";
  let stderr = "";
  child.stdout.setEncoding("utf8");
  child.stderr.setEncoding("utf8");
  child.stdout.on("data", (chunk) => { stdout += chunk; });
  child.stderr.on("data", (chunk) => { stderr += chunk; });
  if (options.input !== undefined) child.stdin.end(options.input);
  else child.stdin.end();
  const exitCode = await new Promise((resolve, reject) => {
    child.on("error", reject);
    child.on("close", resolve);
  });
  const wallTimeMs = performance.now() - started;
  if (exitCode !== 0) throw new Error(`cheaper ${args.join(" ")} exited ${exitCode}: ${stderr || stdout}`);
  try {
    return { envelope: JSON.parse(stdout), stdout, stderr, exitCode, wallTimeMs };
  } catch (cause) {
    throw new Error(`cheaper ${args.join(" ")} did not emit one JSON envelope`, { cause });
  }
}

export async function readPromptLog(promptLog) {
  try {
    return await readFile(promptLog, "utf8");
  } catch (error) {
    if (error?.code === "ENOENT") return "";
    throw error;
  }
}

export async function countUsageRecords(configDir) {
  try {
    const source = await readFile(path.join(configDir, "usage.jsonl"), "utf8");
    return source.split(/\r?\n/).filter(Boolean).length;
  } catch (error) {
    if (error?.code === "ENOENT") return 0;
    throw error;
  }
}
