import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { apmAvailable, apmInstall, bundledCli, makeConsumerLayout } from "./apm-fixture.js";

// Real-provider e2e (DESIGN-0026 / ADR-0024): apm install した隔離
// consumer の bundled CLI が Ollama コンテナへ実到達することを検証
// する。Ollama 未到達・model 不在・apm 不在のときは describe 全体を
// skip し、CI 必須ゲートから分離する。
//
// model は IMPL-0008 で schema 適合を実測した qwen3.5:9b を default に
// し、CHEAPER_E2E_OLLAMA_MODEL で override できる。
const OLLAMA_BASE = process.env.CHEAPER_E2E_OLLAMA_BASE ?? "http://localhost:11434";
const MODEL = process.env.CHEAPER_E2E_OLLAMA_MODEL ?? "qwen3.5:9b";
const PROVIDER = "ollama";

interface OllamaModels {
  data?: Array<{ id?: string; name?: string }>;
}

async function probeOllama(): Promise<boolean> {
  try {
    const res = await fetch(`${OLLAMA_BASE}/v1/models`, {
      signal: AbortSignal.timeout(3_000),
    });
    if (!res.ok) return false;
    const body = (await res.json()) as OllamaModels;
    const ids = (body.data ?? []).flatMap((m) => [m.id, m.name].filter(Boolean) as string[]);
    return ids.some((id) => id === MODEL || id.startsWith(`${MODEL}:`) || id === `${MODEL}:latest`);
  } catch {
    return false;
  }
}

// IMPL-0008 recipe: openai-completions 経由で Ollama を custom
// provider にし、reasoning_effort:"none" + temperature:0 で小モデルの
// thinking と揺らぎを抑える。apiKey は Ollama が keyless でも Pi の
// provider-composer が要求するため literal を置く。
function modelsJson(): string {
  return JSON.stringify(
    {
      providers: {
        [PROVIDER]: {
          name: "Ollama (local)",
          api: "openai-completions",
          baseUrl: `${OLLAMA_BASE}/v1`,
          apiKey: "ollama",
          models: [
            {
              id: MODEL,
              name: `${MODEL} (e2e)`,
              reasoning: true,
              cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
              contextWindow: 262144,
              maxTokens: 32768,
              samplingParams: { reasoning_effort: "none", temperature: 0 },
            },
          ],
        },
      },
    },
    null,
    2,
  );
}

// 9b は built-in 分類表（\b(1|3|7|8)b\b）に match しないため
// overrides で low に pin し、preferences.low 先頭で第一候補にする。
function modelsYaml(): string {
  return [
    "version: 1",
    "overrides:",
    `  ${PROVIDER}/${MODEL}: low`,
    "preferences:",
    "  low:",
    `    - ${PROVIDER}/${MODEL}`,
    "",
  ].join("\n");
}

// 小モデルの repair 収束に必要な延長値（IMPL-0008 実測）。
function policyYaml(): string {
  return [
    "version: 1",
    "execution:",
    "  timeout: 10m",
    "  maxTurns: 16",
    "output:",
    "  repairAttempts: 4",
    "",
  ].join("\n");
}

interface WorkerRun {
  status: number | null;
  stdout: string;
  stderr: string;
}

describe("real-provider e2e (Ollama)", () => {
  let available = false;
  let root = "";
  let consumer = "";
  let configDir = "";
  let cli: string | null = null;
  let env: Record<string, string> = {};

  beforeAll(async () => {
    if (!apmAvailable()) return;
    if (!(await probeOllama())) return;

    root = mkdtempSync(path.join(tmpdir(), "cheaper-ollama-e2e-"));
    ({ consumer } = makeConsumerLayout(root, "windsurf"));
    const install = apmInstall(consumer, root, "windsurf");
    if (install.status !== 0) return;
    cli = bundledCli(consumer, "windsurf");
    if (cli === null || !existsSync(cli)) return;

    configDir = path.join(root, "cheaper-config");
    const piDir = path.join(configDir, "pi");
    mkdirSync(piDir, { recursive: true });
    mkdirSync(configDir, { recursive: true });
    writeFileSync(path.join(piDir, "models.json"), modelsJson());
    writeFileSync(path.join(configDir, "models.yaml"), modelsYaml());
    writeFileSync(path.join(configDir, "policy.yaml"), policyYaml());

    env = {
      ...(process.env.PATH !== undefined ? { PATH: process.env.PATH } : {}),
      ...(process.env.SystemRoot !== undefined ? { SystemRoot: process.env.SystemRoot } : {}),
      HOME: root,
      USERPROFILE: root,
      CHEAPER_CONFIG_DIR: configDir,
      NODE_ENV: "production",
    };
    available = true;
  }, 180_000);

  afterAll(() => {
    if (root) rmSync(root, { recursive: true, force: true });
  });

  function runWorker(args: string[], input: string, cwd: string): WorkerRun {
    const r = spawnSync(process.execPath, [cli as string, ...args, "--cwd", cwd], {
      cwd,
      env,
      input,
      encoding: "utf8",
      timeout: 600_000,
    });
    return { status: r.status, stdout: r.stdout, stderr: r.stderr };
  }

  function expectRealEnvelope(r: WorkerRun, worker: string) {
    expect(r.status, r.stderr).toBe(0);
    const e = JSON.parse(r.stdout.trim()) as Record<string, unknown>;
    expect(e.schemaVersion).toBe(1);
    expect(e.ok).toBe(true);
    expect(e.worker).toBe(worker);
    const meta = e.meta as Record<string, unknown>;
    const model = meta.model as Record<string, unknown>;
    expect(model.provider).toBe(PROVIDER);
    expect(model.id).toBe(MODEL);
    const usage = meta.usage as Record<string, unknown> | undefined;
    expect(usage, "real usage metrics present").toBeDefined();
    expect(Number(usage?.input)).toBeGreaterThan(0);
    return e;
  }

  // SPEC-0022: delegation ごとの実測 usage が <CHEAPER_CONFIG_DIR>/
  // usage.jsonl に継続記録されることを検証する。
  function expectUsageRecord(worker: string) {
    const logPath = path.join(configDir, "usage.jsonl");
    expect(existsSync(logPath), `usage.jsonl exists for ${worker}`).toBe(true);
    const records = readFileSync(logPath, "utf8")
      .split("\n")
      .filter((l) => l !== "")
      .map((l) => JSON.parse(l) as Record<string, unknown>);
    const rec = records.find(
      (r) =>
        r.kind === "run" &&
        r.worker === worker &&
        (r.model as Record<string, unknown> | undefined)?.provider === PROVIDER,
    );
    expect(rec, `usage.jsonl run record for ${worker}`).toBeDefined();
    const usage = rec?.usage as Record<string, unknown> | undefined;
    expect(Number(usage?.input)).toBeGreaterThan(0);
  }

  it("bulk-read reaches Ollama and returns findings", async (t) => {
    if (!available) {
      t.skip("Ollama container or model not reachable; set up per AGENTS.md");
      return;
    }
    mkdirSync(path.join(consumer, "src"), { recursive: true });
    writeFileSync(
      path.join(consumer, "src", "greeter.ts"),
      'export function greet(name: string): string { return "hi " + name; }\nexport const DEFAULT_GREETING = "hi";\n',
    );
    const r = runWorker(
      ["bulk-read"],
      JSON.stringify({
        task: "List the exported names in src/greeter.ts",
        paths: ["src/greeter.ts"],
        maxFindings: 3,
      }),
      consumer,
    );
    const e = expectRealEnvelope(r, "bulk-reader");
    const result = e.result as Record<string, unknown>;
    expect(Array.isArray(result.findings)).toBe(true);
    expectUsageRecord("bulk-reader");
  }, 600_000);

  it("code-write reaches Ollama and creates a file", async (t) => {
    if (!available) {
      t.skip("Ollama container or model not reachable; set up per AGENTS.md");
      return;
    }
    const r = runWorker(
      ["code-write"],
      JSON.stringify({
        task: "Create src/farewell.ts exporting a farewell() function that returns a goodbye string.",
        targets: [{ path: "src/farewell.ts", intent: "create" }],
      }),
      consumer,
    );
    const e = expectRealEnvelope(r, "code-writer");
    const meta = e.meta as Record<string, unknown>;
    const changes = meta.changes as Record<string, unknown> | undefined;
    const files = changes?.files;
    expect(Array.isArray(files) && files.length > 0).toBe(true);
    expect(existsSync(path.join(consumer, "src", "farewell.ts"))).toBe(true);
    expectUsageRecord("code-writer");
  }, 600_000);

  it("doc-write reaches Ollama and creates a document", async (t) => {
    if (!available) {
      t.skip("Ollama container or model not reachable; set up per AGENTS.md");
      return;
    }
    const r = runWorker(
      ["doc-write"],
      JSON.stringify({
        task: "Write a short readme describing this workspace.",
        documentType: "readme",
        targets: [{ path: "docs/hello.md", intent: "create" }],
      }),
      consumer,
    );
    expectRealEnvelope(r, "doc-writer");
    expect(existsSync(path.join(consumer, "docs", "hello.md"))).toBe(true);
    expectUsageRecord("doc-writer");
  }, 600_000);
});
