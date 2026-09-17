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
    expect(meta.model).toEqual({ provider: "cheaper-fake", id: "fake-model", tier: "low" });
    expect(meta.changes?.files).toEqual([]);
    expect(meta.changes?.totals).toEqual({ files: 0, linesAdded: 0, linesDeleted: 0 });
    expect(r.stderr).toContain("model: cheaper-fake/fake-model (tier low)");
  });

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
