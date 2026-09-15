import { rm } from "node:fs/promises";
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
