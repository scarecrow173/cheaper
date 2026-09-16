import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  apmAvailable,
  apmInstall,
  bundledCli,
  bundleRoot,
  makeConsumerLayout,
} from "./apm-fixture.js";
import { expectEnvelopeOnly } from "./helpers.js";

// import 不要の scripted-response-array fixture（bundled CLI が内蔵する
// createFakePi が wrap する）。fauxAssistantMessage 相当の raw 形。
const fakeStreamFixture = (body: unknown) => `export default [
  {
    role: "assistant",
    content: [{ type: "text", text: ${JSON.stringify(JSON.stringify(body))} }],
    api: "faux",
    provider: "faux",
    model: "faux-1",
    usage: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, totalTokens: 0,
      cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: 0 } },
    stopReason: "stop",
    timestamp: 0,
  },
];
`;

describe("delegation loop e2e: pre_read_code block -> bulk-read via bundled CLI", () => {
  let root: string;
  let consumer: string;

  beforeEach(() => {
    if (!apmAvailable()) return;
    root = mkdtempSync(path.join(tmpdir(), "cheaper-loop-"));
    ({ consumer } = makeConsumerLayout(root));
  });
  afterEach(() => {
    if (root) rmSync(root, { recursive: true, force: true });
  });

  it("large file read is blocked and delegated to a compressed bulk-read result", async (t) => {
    if (!apmAvailable()) {
      t.skip("apm CLI not available on PATH");
      return;
    }
    expect(apmInstall(consumer, root).status).toBe(0);
    const cli = bundledCli(consumer);
    const hookScript = path.join(bundleRoot(consumer), "pre-read-code.js");

    // 1. parent が large file を read しようとする → hook が block
    const bigContent = "lorem ipsum dolor sit amet ".repeat(4000);
    const bigPath = path.join(consumer, "big.txt");
    writeFileSync(bigPath, bigContent, "utf8");
    const hook = spawnSync(process.execPath, [hookScript, cli], {
      cwd: consumer,
      input: JSON.stringify({
        agent_action_name: "pre_read_code",
        tool_info: { file_path: bigPath },
      }),
      encoding: "utf8",
    });
    expect(hook.status).toBe(2);
    expect(hook.stderr).toContain("bulk-read");
    // file 本文は block 時点で parent context に入っていない

    // 2. parent が guidance に従い bundled CLI で bulk-read を呼ぶ
    mkdirSync(path.join(consumer, ".cheaper"), { recursive: true });
    writeFileSync(
      path.join(consumer, ".cheaper", "models.yaml"),
      "version: 1\noverrides:\n  cheaper-fake/fake-model: low\n",
    );
    const fixture = path.join(root, "fake-stream.mjs");
    writeFileSync(
      fixture,
      fakeStreamFixture({
        summary: "big.txt repeats a lorem-ipsum sentence ~4000 times",
        findings: [
          {
            title: "repetitive filler file",
            detail: "No real content beyond a repeated sentence.",
            confidence: "high",
            locations: [{ path: "big.txt", lines: { start: 1, end: 1 } }],
          },
        ],
        relevantFiles: [{ path: "big.txt", reason: "delegated large file" }],
        suggestedReads: [],
        openQuestions: [],
      }),
    );
    const run = spawnSync(process.execPath, [cli, "bulk-read", "--cwd", consumer], {
      cwd: consumer,
      env: {
        PATH: process.env.PATH ?? "",
        SystemRoot: process.env.SystemRoot ?? "",
        HOME: root,
        USERPROFILE: root,
        PI_CODING_AGENT_DIR: path.join(root, "pi-agent"),
        PI_OFFLINE: "1",
        CHEAPER_CONFIG_DIR: path.join(root, "cheaper-config"),
        NODE_ENV: "test",
        CHEAPER_TEST_FAKE_STREAM: fixture,
      },
      input: JSON.stringify({ task: "summarize big.txt", paths: ["big.txt"] }),
      encoding: "utf8",
    });

    // 3. parent は file 本文ではなく圧縮結果だけを受け取る
    expect(run.status).toBe(0);
    const e = expectEnvelopeOnly(run.stdout);
    expect(e.ok).toBe(true);
    expect(e.worker).toBe("bulk-reader");
    const result = e.result as {
      summary: string;
      findings: Array<{ title: string }>;
    };
    expect(result.summary).toContain("lorem-ipsum");
    expect(result.findings).toHaveLength(1);
    // 圧縮結果に file 本文（4000 反復）が含まれないこと
    expect(run.stdout.length).toBeLessThan(bigContent.length / 10);
    const meta = e.meta as { model: { provider: string; id: string } | null };
    expect(meta.model).toEqual(
      expect.objectContaining({ provider: "cheaper-fake", id: "fake-model" }),
    );
  }, 180_000);
});
