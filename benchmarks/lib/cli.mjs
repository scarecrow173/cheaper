import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { performance } from "node:perf_hooks";
import { fileURLToPath } from "node:url";
import { writeFakeStreamFixture } from "../fixtures/fake-worker-stream.mjs";

export const repositoryRoot = fileURLToPath(new URL("../..", import.meta.url));

const binaryFileName = process.platform === "win32" ? "cheaper.exe" : "cheaper";
let resolvedBinary;

// cargo build 出力の native binary を解決する。`target/release` を優先
// し、無ければ `target/debug` へ fallback する。fake-stream seam
// (`CHEAPER_TEST_FAKE_STREAM`)は `dev-seam` cargo feature で gate
// されているため、benchmark 用 binary は feature 有効で build する:
//   mise exec -- cargo build --release -p cheaper-cli --features dev-seam
//   (または `cargo build --workspace --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam`
//    / debug profile なら `cargo build -p cheaper-cli --features dev-seam`)
export function cliBinaryPath() {
  if (resolvedBinary) return resolvedBinary;
  for (const profile of ["release", "debug"]) {
    const candidate = path.join(repositoryRoot, "target", profile, binaryFileName);
    if (existsSync(candidate)) {
      resolvedBinary = candidate;
      return resolvedBinary;
    }
  }
  throw new Error(
    "cheaper binary not found; build it first: " +
      "cargo build --release -p cheaper-cli --features dev-seam " +
      "(or cargo build -p cheaper-cli --features dev-seam)",
  );
}

// 結果 record の `command` 表示用 — repo 相対 path + args。
export function cliCommandLine(args) {
  return `${path.relative(repositoryRoot, cliBinaryPath())} ${args.join(" ")}`;
}

export async function prepareWorkspace(scenario, directory) {
  await mkdir(directory, { recursive: true });
  for (const [relative, content] of Object.entries(scenario.files ?? {})) {
    const target = path.resolve(directory, relative);
    if (!target.startsWith(`${directory}${path.sep}`))
      throw new TypeError(`fixture path escapes workspace: ${relative}`);
    await mkdir(path.dirname(target), { recursive: true });
    await writeFile(target, content, "utf8");
  }
  const projectConfig = path.join(directory, ".cheaper");
  await mkdir(projectConfig, { recursive: true });
  await writeFile(
    path.join(projectConfig, "models.yaml"),
    "version: 1\noverrides:\n  cheaper-fake/fake-model: low\n",
    "utf8",
  );
  if (scenario.processPolicy) {
    const policy = {};
    const processPolicy = structuredClone(scenario.processPolicy);
    for (const task of Object.values(processPolicy.tasks ?? {})) {
      task.command = task.command.map((arg) => (arg === "{node}" ? process.execPath : arg));
    }
    policy.process = processPolicy;
    await writeFile(
      path.join(projectConfig, "policy.yaml"),
      JSON.stringify({ version: 1, ...policy }),
      "utf8",
    );
  }
}

export function benchmarkEnvironment(workspace, options = {}) {
  const configDir = path.join(workspace, ".benchmark-config");
  const fixturePath = path.join(configDir, "fake-stream-fixture.json");
  const env = {
    PATH: process.env.PATH ?? "",
    HOME: workspace,
    USERPROFILE: workspace,
    // Rust seam(`fake_stream.rs`)は `NODE_ENV === "test"` のときのみ有効。
    NODE_ENV: "test",
    CHEAPER_CONFIG_DIR: configDir,
    // 生成した RunFixture JSON を指す。内容は spawn ごとに
    // `runCli` の `options.fakeStream` で書き換える(旧実装の
    // `CHEAPER_BENCH_*` env パラメータ化は process 内 module seam 前提で
    // あり、Rust の declarative fixture では env ではなく生成入力として
    // 渡す)。
    CHEAPER_TEST_FAKE_STREAM: fixturePath,
    ...(process.env.SystemRoot ? { SystemRoot: process.env.SystemRoot } : {}),
    ...(process.env.TEMP ? { TEMP: process.env.TEMP } : {}),
    ...(process.env.TMP ? { TMP: process.env.TMP } : {}),
    // observability activation の env passthrough allowlist —
    // `CHEAPER_LOG=debug node benchmarks/run.mjs …` のような ambient 注入で
    // log 有効時の overhead arm を計測できるようにする。
    // (DESIGN-034DV9ypzmxdAobGSJ6Cm6 §off-state overhead)
    ...(process.env.CHEAPER_LOG ? { CHEAPER_LOG: process.env.CHEAPER_LOG } : {}),
    ...(process.env.CHEAPER_LOG_FORMAT
      ? { CHEAPER_LOG_FORMAT: process.env.CHEAPER_LOG_FORMAT }
      : {}),
    // scenario/arm 指定の env override — ambient passthrough より優先。
    ...(options.env ?? {}),
  };
  return {
    env,
    configDir,
    fixturePath,
    // fixture 生成の scenario-level 既定値。`runCli` の
    // `options.fakeStream` で per-invocation override する。
    fakeStream: {
      responseProfiles: options.responseProfiles ?? ["compact"],
      responseDelayMs: Number(options.responseDelayMs ?? 0),
    },
  };
}

export async function runCli(args, options) {
  const env = { ...options.env };
  // fake-stream seam 有効時は spawn 直前に fixture JSON を生成する —
  // `options.fakeStream` で profile/delay/tool call を per-invocation に
  // 差し替えられる(未指定なら scenario 既定値のまま使う側が env に
  // fixture path を残していてもよいよう、常に生成して内容を確定させる)。
  if (env.CHEAPER_TEST_FAKE_STREAM) {
    await writeFakeStreamFixture(env.CHEAPER_TEST_FAKE_STREAM, options.fakeStream);
  }
  const started = performance.now();
  const child = spawn(cliBinaryPath(), args, {
    cwd: options.cwd,
    env,
    stdio: ["pipe", "pipe", "pipe"],
  });
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
  if (options.input !== undefined) child.stdin.end(options.input);
  else child.stdin.end();
  const exitCode = await new Promise((resolve, reject) => {
    child.on("error", reject);
    child.on("close", resolve);
  });
  const wallTimeMs = performance.now() - started;
  if (exitCode !== 0)
    throw new Error(`cheaper ${args.join(" ")} exited ${exitCode}: ${stderr || stdout}`);
  try {
    return { envelope: JSON.parse(stdout), stdout, stderr, exitCode, wallTimeMs };
  } catch (cause) {
    throw new Error(`cheaper ${args.join(" ")} did not emit one JSON envelope`, { cause });
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
