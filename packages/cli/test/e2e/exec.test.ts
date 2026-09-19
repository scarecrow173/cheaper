import { mkdir, readFile, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { expectEnvelopeOnly, makeIsolatedEnv, runCheaper } from "./helpers.js";

const fixturesDir = fileURLToPath(new URL("./fixtures", import.meta.url));
const fixture = (name: string) => path.join(fixturesDir, name);

interface EnvelopeMeta {
  model: { provider: string; id: string; tier: string } | null;
  changes: {
    files: Array<{ path: string; kind: string; linesAdded: number; linesDeleted: number }>;
    totals: { files: number; linesAdded: number; linesDeleted: number };
  } | null;
  usage: { totalTokens: number } | null;
  turns: number | null;
}

describe("cheaper CLI e2e (execution via CHEAPER_TEST_FAKE_STREAM)", () => {
  let dir: string;
  let ws: string;
  let env: Record<string, string>;

  beforeEach(async () => {
    ({ dir, ws, env } = await makeIsolatedEnv());
    // fake model を low tier へ pin する project models.yaml
    await mkdir(path.join(ws, ".cheaper"), { recursive: true });
    await writeFile(
      path.join(ws, ".cheaper", "models.yaml"),
      "version: 1\noverrides:\n  cheaper-fake/fake-model: low\n",
    );
    env.NODE_ENV = "test";
  });
  afterEach(async () => {
    await rm(dir, { recursive: true, force: true });
  });

  it("bulk-read returns a success envelope with empty changes", async () => {
    const r = await runCheaper(["bulk-read", "--cwd", ws], {
      env: { ...env, CHEAPER_TEST_FAKE_STREAM: fixture("bulk-read-fake.js") },
      input: '{"task":"summarize the workspace"}',
    });
    expect(r.code).toBe(0);
    const e = expectEnvelopeOnly(r.stdout);
    expect(e.ok).toBe(true);
    expect(e.worker).toBe("bulk-reader");
    const result = e.result as Record<string, unknown>;
    expect(result.summary).toBe("workspace summary");
    expect(result.findings).toHaveLength(1);
    const meta = e.meta as EnvelopeMeta;
    expect(meta.model).toEqual({
      provider: "cheaper-fake",
      id: "fake-model",
      tier: "low",
      selection: "tier",
      tierSource: "explicit-override",
    });
    expect(meta.changes?.files).toEqual([]);
    expect(meta.changes?.totals).toEqual({ files: 0, linesAdded: 0, linesDeleted: 0 });
    expect(r.stderr).toContain("model: cheaper-fake/fake-model (tier low)");
  }, 20_000);

  it("code-write measures meta.changes from the journal, not the model", async () => {
    const r = await runCheaper(["code-write", "--cwd", ws], {
      env: { ...env, CHEAPER_TEST_FAKE_STREAM: fixture("code-write-fake.js") },
      input: '{"task":"create out.txt","targets":[{"path":"out.txt","intent":"create"}]}',
    });
    expect(r.code).toBe(0);
    const e = expectEnvelopeOnly(r.stdout);
    expect(e.ok).toBe(true);
    const meta = e.meta as EnvelopeMeta;
    expect(meta.changes?.totals.files).toBeGreaterThan(0);
    expect(meta.changes?.files.map((f) => f.path)).toContain("out.txt");
    // 実際に workspace へ書き込まれたこと
    expect(await readFile(path.join(ws, "out.txt"), "utf8")).toBe("hello\n");
  });

  it("code-write cannot write outside targets or read denied files", async () => {
    await writeFile(path.join(ws, ".env"), "SECRET=1\n", "utf8");
    const r = await runCheaper(["code-write", "--cwd", ws], {
      env: { ...env, CHEAPER_TEST_FAKE_STREAM: fixture("scope-deny-fake.js") },
      input: '{"task":"x","targets":[{"path":"out.txt","intent":"create"}]}',
    });
    expect(r.code).toBe(0);
    const e = expectEnvelopeOnly(r.stdout);
    expect(e.ok).toBe(true);
    const meta = e.meta as EnvelopeMeta & { rejectedToolCalls: number | null };
    // scope 外 write + denied read の 2 件が tool error として拒否される
    expect(meta.rejectedToolCalls).toBe(2);
    expect(meta.changes?.totals.files ?? 0).toBe(0);
    // 実 FS にも書き込まれていない
    await expect(readFile(path.join(ws, "evil.txt"), "utf8")).rejects.toThrow();
  });

  it("doc-write writes a document and returns unresolved items", async () => {
    const r = await runCheaper(["doc-write", "--cwd", ws], {
      env: { ...env, CHEAPER_TEST_FAKE_STREAM: fixture("doc-write-fake.js") },
      input:
        '{"task":"write a guide","documentType":"guide","targets":[{"path":"docs/guide.md","intent":"create"}],"decisions":["d1"]}',
    });
    expect(r.code).toBe(0);
    const e = expectEnvelopeOnly(r.stdout);
    expect(e.ok).toBe(true);
    const result = e.result as Record<string, unknown>;
    expect(result.unresolved).toEqual(["release date not provided"]);
    const meta = e.meta as EnvelopeMeta;
    expect(meta.changes?.files.map((f) => f.path)).toContain("docs/guide.md");
    expect(await readFile(path.join(ws, "docs/guide.md"), "utf8")).toContain("# Guide");
  });

  it("count_tokens tool and count-tokens CLI return the same count (parity)", async () => {
    await writeFile(path.join(ws, "parity.txt"), "the quick brown fox\n", "utf8");
    const capture = path.join(dir, "tool-result.json");
    const run = await runCheaper(["bulk-read", "--cwd", ws], {
      env: {
        ...env,
        CHEAPER_TEST_FAKE_STREAM: fixture("count-tokens-fake.js"),
        CHEAPER_TEST_CAPTURE: capture,
      },
      input: '{"task":"count parity.txt"}',
    });
    expect(run.code).toBe(0);
    const toolResult = JSON.parse(await readFile(capture, "utf8")) as {
      path: string;
      tokens: number;
      bytes: number;
      tokenizer: unknown;
    };
    expect(toolResult.path).toBe("parity.txt");

    const cli = await runCheaper(["count-tokens", "parity.txt", "--cwd", ws], { env });
    expect(cli.code).toBe(0);
    const e = expectEnvelopeOnly(cli.stdout);
    const cliResult = e.result as { tokens: number; bytes: number; tokenizer: unknown };
    expect(cliResult.tokens).toBe(toolResult.tokens);
    expect(cliResult.bytes).toBe(toolResult.bytes);
    expect(cliResult.tokenizer).toEqual(toolResult.tokenizer);
  }, 20_000);

  it("appends run and count records to <CHEAPER_CONFIG_DIR>/usage.jsonl", async () => {
    // SPEC-0022: delegation 計測の継続記録（ADR-0025）。
    const run = await runCheaper(["bulk-read", "--cwd", ws], {
      env: { ...env, CHEAPER_TEST_FAKE_STREAM: fixture("bulk-read-fake.js") },
      input: '{"task":"summarize"}',
    });
    expect(run.code).toBe(0);

    await writeFile(path.join(ws, "note.txt"), "hello world\n", "utf8");
    const count = await runCheaper(["count-tokens", "note.txt", "--cwd", ws], { env });
    expect(count.code).toBe(0);

    const logPath = path.join(env.CHEAPER_CONFIG_DIR as string, "usage.jsonl");
    const records = (await readFile(logPath, "utf8"))
      .split("\n")
      .filter((l) => l !== "")
      .map((l) => JSON.parse(l) as Record<string, unknown>);

    const runRec = records.find((r) => r.kind === "run");
    expect(runRec).toMatchObject({
      schemaVersion: 1,
      worker: "bulk-reader",
      ok: true,
      model: { provider: "cheaper-fake", id: "fake-model", tier: "low" },
    });
    expect(runRec?.usage).toBeDefined();

    const countRec = records.find((r) => r.kind === "count");
    expect(countRec).toMatchObject({
      schemaVersion: 1,
      worker: null,
      ok: true,
      result: { path: "note.txt" },
    });
    const countResult = countRec?.result as { tokens?: number } | undefined;
    expect(typeof countResult?.tokens).toBe("number");
  }, 20_000);

  it("accepts the scripted-response-array fixture form", async () => {
    const r = await runCheaper(["bulk-read", "--cwd", ws], {
      env: { ...env, CHEAPER_TEST_FAKE_STREAM: fixture("array-fake.js") },
      input: '{"task":"t"}',
    });
    expect(r.code).toBe(0);
    const e = expectEnvelopeOnly(r.stdout);
    expect(e.ok).toBe(true);
    expect((e.result as Record<string, unknown>).summary).toBe("array form ok");
  });

  it("measures meta.savings from a real read tool call and records it", async () => {
    await writeFile(path.join(ws, "big-input.txt"), "alpha beta gamma delta\n".repeat(200), "utf8");
    const r = await runCheaper(["bulk-read", "--cwd", ws], {
      env: { ...env, CHEAPER_TEST_FAKE_STREAM: fixture("bulk-read-savings-fake.js") },
      input: '{"task":"summarize big-input.txt"}',
    });
    expect(r.code).toBe(0);
    const e = expectEnvelopeOnly(r.stdout);
    const meta = e.meta as {
      savings?: { sourceTokens: number; returnedTokens: number; avoidedContextTokens: number };
    };
    expect(meta.savings?.sourceTokens).toBeGreaterThan(0);
    expect(meta.savings?.returnedTokens).toBeGreaterThan(0);
    expect(meta.savings?.avoidedContextTokens).toBe(
      Math.max(0, (meta.savings?.sourceTokens ?? 0) - (meta.savings?.returnedTokens ?? 0)),
    );
    // read output が返却 result を大きく上回る fixture
    expect(meta.savings?.avoidedContextTokens).toBeGreaterThan(0);

    const logPath = path.join(env.CHEAPER_CONFIG_DIR as string, "usage.jsonl");
    const records = (await readFile(logPath, "utf8"))
      .split("\n")
      .filter((l) => l !== "")
      .map((l) => JSON.parse(l) as Record<string, unknown>);
    const runRec = records.find((rec) => rec.kind === "run");
    expect(runRec).toMatchObject({ worker: "bulk-reader", ok: true, savings: meta.savings });
    expect(runRec?.turns).toBe(2);
    expect(runRec?.toolCalls).toBe(1);
  }, 20_000);

  it("aggregates usage.jsonl via `cheaper usage` with filters", async () => {
    await writeFile(path.join(ws, "big-input.txt"), "x\n".repeat(100), "utf8");
    const run = await runCheaper(["bulk-read", "--cwd", ws], {
      env: { ...env, CHEAPER_TEST_FAKE_STREAM: fixture("bulk-read-savings-fake.js") },
      input: '{"task":"t"}',
    });
    expect(run.code).toBe(0);

    const usage = await runCheaper(["usage"], { env });
    expect(usage.code).toBe(0);
    const e = expectEnvelopeOnly(usage.stdout);
    const result = e.result as {
      records: number;
      ok: number;
      usage: { totalTokens: number; costUsd: number };
      measured: { runs: number; sourceTokens: number; avoidedContextTokens: number };
      estimated?: unknown;
    };
    expect(result.records).toBe(1);
    expect(result.ok).toBe(1);
    expect(result.usage.totalTokens).toBeGreaterThanOrEqual(0);
    expect(result.measured.runs).toBe(1);
    expect(result.measured.sourceTokens).toBeGreaterThan(0);
    // pricing.json 未提供なので estimated は出ない
    expect(result.estimated).toBeUndefined();

    // worker filter は一致/不一致を区別する
    const hit = await runCheaper(["usage", "--worker", "bulk-reader"], { env });
    expect((JSON.parse(hit.stdout) as { result: { records: number } }).result.records).toBe(1);
    const miss = await runCheaper(["usage", "--worker", "doc-writer"], { env });
    expect((JSON.parse(miss.stdout) as { result: { records: number } }).result.records).toBe(0);
    const future = await runCheaper(["usage", "--since", "1h"], { env });
    expect((JSON.parse(future.stdout) as { result: { records: number } }).result.records).toBe(1);

    // usage command 自体は usage.jsonl に記録されない
    const logPath = path.join(env.CHEAPER_CONFIG_DIR as string, "usage.jsonl");
    const lines = (await readFile(logPath, "utf8")).split("\n").filter((l) => l !== "");
    expect(lines).toHaveLength(1);
  }, 30_000);

  it("emits estimated savings only when compare-model pricing exists", async () => {
    await writeFile(path.join(ws, "big-input.txt"), "x\n".repeat(100), "utf8");
    const run = await runCheaper(["bulk-read", "--cwd", ws], {
      env: { ...env, CHEAPER_TEST_FAKE_STREAM: fixture("bulk-read-savings-fake.js") },
      input: '{"task":"t"}',
    });
    expect(run.code).toBe(0);

    // pricing.json なし → estimated 非出力
    const noPricing = await runCheaper(["usage", "--compare-model", "openai/gpt-5"], { env });
    expect(
      (JSON.parse(noPricing.stdout) as { result: { estimated?: unknown } }).result.estimated,
    ).toBeUndefined();

    // 新鮮な pricing.json → estimated 出力
    await mkdir(env.CHEAPER_CONFIG_DIR as string, { recursive: true });
    await writeFile(
      path.join(env.CHEAPER_CONFIG_DIR as string, "pricing.json"),
      JSON.stringify({
        models: {
          "openai/gpt-5": {
            inputPerMillionUsd: 3,
            outputPerMillionUsd: 15,
            checkedAt: new Date().toISOString(),
            source: "e2e-fixture",
          },
        },
      }),
    );
    const withPricing = await runCheaper(["usage", "--compare-model", "openai/gpt-5"], { env });
    const est = (
      JSON.parse(withPricing.stdout) as {
        result: {
          estimated?: {
            compareModel: string;
            estimatedParentEquivalentCostUsd: number;
            estimatedSavingsUsd: number;
            estimatedSavingsPercent: number | null;
            pricing: { source: string; stale: boolean };
          };
        };
      }
    ).result.estimated;
    expect(est?.compareModel).toBe("openai/gpt-5");
    expect(est?.estimatedParentEquivalentCostUsd).toBeGreaterThan(0);
    expect(est?.pricing).toMatchObject({ source: "e2e-fixture", stale: false });
  }, 30_000);

  it("ignores CHEAPER_TEST_FAKE_STREAM when NODE_ENV is not test", async () => {
    const r = await runCheaper(["bulk-read", "--cwd", ws], {
      env: {
        ...env,
        NODE_ENV: "production",
        CHEAPER_TEST_FAKE_STREAM: fixture("bulk-read-fake.js"),
      },
      input: '{"task":"t"}',
    });
    // fake stream が注入されないため未認証のまま MODEL_TIER_UNAVAILABLE
    expect(r.code).toBe(5);
    const e = expectEnvelopeOnly(r.stdout);
    expect((e.error as { code: string }).code).toBe("MODEL_TIER_UNAVAILABLE");
  });
});
