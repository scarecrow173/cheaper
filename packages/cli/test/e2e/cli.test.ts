import { realpath, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { countFileTokens } from "@cheaper/core";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { expectEnvelopeOnly, makeIsolatedEnv, runCheaper } from "./helpers.js";

describe("cheaper CLI e2e (non-execution)", () => {
  let dir: string;
  let ws: string;
  let env: Record<string, string>;

  beforeEach(async () => {
    ({ dir, ws, env } = await makeIsolatedEnv());
  });
  afterEach(async () => {
    await rm(dir, { recursive: true, force: true });
  });

  it("--help exits 0 with usage on stdout", async () => {
    const r = await runCheaper(["--help"], { env, cwd: ws });
    expect(r.code).toBe(0);
    expect(r.stdout).toContain("Usage: cheaper");
  });

  it("--version exits 0 with the package version", async () => {
    const r = await runCheaper(["--version"], { env, cwd: ws });
    expect(r.code).toBe(0);
    expect(r.stdout).toMatch(/^\d+\.\d+\.\d+\n$/);
  });

  it("no stdin input → INVALID_REQUEST exit 2 with a single envelope line", async () => {
    const r = await runCheaper(["bulk-read", "--cwd", ws], { env });
    expect(r.code).toBe(2);
    const env_ = expectEnvelopeOnly(r.stdout);
    expect(env_.ok).toBe(false);
    expect((env_.error as { code: string }).code).toBe("INVALID_REQUEST");
    expect((env_.error as { exitCode: number }).exitCode).toBe(2);
  });

  it("path-like worker name → INVALID_WORKER_NAME exit 3", async () => {
    const r = await runCheaper(["run", "./foo.md", "--cwd", ws], {
      env,
      input: '{"task":"x"}',
    });
    expect(r.code).toBe(3);
    const env_ = expectEnvelopeOnly(r.stdout);
    expect((env_.error as { code: string }).code).toBe("INVALID_WORKER_NAME");
  });

  it("--timeout expansion → POLICY_VIOLATION exit 4", async () => {
    const r = await runCheaper(["bulk-read", "--cwd", ws, "--timeout", "9999s"], {
      env,
      input: '{"task":"x"}',
    });
    expect(r.code).toBe(4);
    const env_ = expectEnvelopeOnly(r.stdout);
    expect((env_.error as { code: string }).code).toBe("POLICY_VIOLATION");
    expect(env_.worker).toBe("bulk-reader");
  });

  it("unauthenticated environment → MODEL_TIER_UNAVAILABLE exit 5", async () => {
    const r = await runCheaper(["bulk-read", "--cwd", ws], {
      env,
      input: '{"task":"x"}',
    });
    expect(r.code).toBe(5);
    const env_ = expectEnvelopeOnly(r.stdout);
    expect((env_.error as { code: string }).code).toBe("MODEL_TIER_UNAVAILABLE");
    expect((env_.error as { retryable: boolean }).retryable).toBe(true);
  });

  it("error envelope is still emitted with --format text", async () => {
    const r = await runCheaper(["bulk-read", "--cwd", ws, "--format", "text"], { env });
    expect(r.code).toBe(2);
    expect(r.stdout.split("\n")[0]).toBe("error INVALID_REQUEST");
  });
});

describe("cheaper count-tokens e2e", () => {
  let dir: string;
  let ws: string;
  let env: Record<string, string>;

  beforeEach(async () => {
    ({ dir, ws, env } = await makeIsolatedEnv());
  });
  afterEach(async () => {
    await rm(dir, { recursive: true, force: true });
  });

  it("returns the canonical count as an envelope without touching a model", async () => {
    await writeFile(path.join(ws, "target.txt"), "hello world\n", "utf8");
    const r = await runCheaper(["count-tokens", "target.txt", "--cwd", ws], { env });
    expect(r.code).toBe(0);
    const e = expectEnvelopeOnly(r.stdout);
    expect(e.ok).toBe(true);
    expect(e.worker).toBeNull();
    const result = e.result as {
      path: string;
      tokens: number;
      bytes: number;
      tokenizer: { name: string; implementation: string; version: string };
    };
    expect(result.path).toBe("target.txt");
    expect(result.bytes).toBe(12);
    expect(result.tokenizer.name).toBe("o200k_base");
    expect(result.tokenizer.implementation).toBe("js-tiktoken");
    // CLI と core canonical 実装（= count_tokens tool）が同一 count を返す
    const direct = await countFileTokens({
      root: await realpath(ws),
      path: "target.txt",
      symlinkPolicy: "within-workspace",
    });
    expect(result.tokens).toBe(direct.tokens);
    expect(result.tokenizer).toEqual(direct.tokenizer);
  });

  it("rejects paths outside the workspace with INVALID_REQUEST exit 2", async () => {
    const r = await runCheaper(["count-tokens", "../outside.txt", "--cwd", ws], { env });
    expect(r.code).toBe(2);
    const e = expectEnvelopeOnly(r.stdout);
    expect((e.error as { code: string }).code).toBe("INVALID_REQUEST");
  });

  it("rejects missing files with INVALID_REQUEST exit 2", async () => {
    const r = await runCheaper(["count-tokens", "missing.txt", "--cwd", ws], { env });
    expect(r.code).toBe(2);
    const e = expectEnvelopeOnly(r.stdout);
    expect((e.error as { code: string }).code).toBe("INVALID_REQUEST");
  });

  it("requires a path argument", async () => {
    const r = await runCheaper(["count-tokens", "--cwd", ws], { env });
    expect(r.code).toBe(2);
    const e = expectEnvelopeOnly(r.stdout);
    expect((e.error as { code: string }).code).toBe("INVALID_REQUEST");
  });

  it("returns FILE_TOO_LARGE when bytes exceed CHEAPER_COUNT_MAX_BYTES", async () => {
    await writeFile(path.join(ws, "big.bin"), Buffer.alloc(64, 0x61));
    const r = await runCheaper(["count-tokens", "big.bin", "--cwd", ws], {
      env: { ...env, CHEAPER_COUNT_MAX_BYTES: "16" },
    });
    expect(r.code).toBe(2);
    const e = expectEnvelopeOnly(r.stdout);
    const err = e.error as { code: string; details?: Record<string, unknown> };
    expect(err.code).toBe("FILE_TOO_LARGE");
    expect(err.details).toMatchObject({ path: "big.bin", bytes: 64, maxBytes: 16 });
  });

  // 64 bytes file: 緩い parse（parseInt）で 16/1 に潰れる値が旧実装では
  // cap を踏んで FILE_TOO_LARGE になっていたはず、という区別が付く大きさ。
  it.each([
    "not-a-number",
    "16MB", // 数字 prefix の部分 parse を許容しない
    "1.5", // 整数でない
    "0", // 正でない
    "-1", // 負
    "   ", // 空白のみ
    "0x10", // 10 進数字列でない
    "9007199254740993", // Number.MAX_SAFE_INTEGER 超過
  ])("falls back to the default cap on an invalid CHEAPER_COUNT_MAX_BYTES (%s)", async (value) => {
    await writeFile(path.join(ws, "target.txt"), Buffer.alloc(64, 0x61));
    const r = await runCheaper(["count-tokens", "target.txt", "--cwd", ws], {
      env: { ...env, CHEAPER_COUNT_MAX_BYTES: value },
    });
    expect(r.code).toBe(0);
    const e = expectEnvelopeOnly(r.stdout);
    expect(e.ok).toBe(true);
    const result = e.result as { bytes: number };
    expect(result.bytes).toBe(64);
  });
});
