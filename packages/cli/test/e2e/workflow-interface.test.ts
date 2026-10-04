import { mkdir, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { expectEnvelopeOnly, makeIsolatedEnv, runCheaper } from "./helpers.js";

describe("cheaper workflow interface", () => {
  let isolated: Awaited<ReturnType<typeof makeIsolatedEnv>>;
  const raw = {
    schemaVersion: 1,
    inputs: {
      target: { type: "string", minLength: 1 },
      limit: { type: "integer", minimum: 1, default: 3 },
    },
    jobs: {
      privateScan: { kind: "tool", tool: "read", input: { path: "PRIVATE PARAMETER", limit: 7 } },
    },
    outputs: { result: { schema: { type: "object" }, value: { $from: "privateScan", path: "" } } },
  };
  const expected = {
    schemaVersion: 1,
    inputs: {
      limit: { schema: { type: "integer", minimum: 1, default: 3 }, required: false },
      target: { schema: { type: "string", minLength: 1 }, required: true },
    },
    outputs: { result: { schema: { type: "object" } } },
  };
  beforeEach(async () => {
    isolated = await makeIsolatedEnv();
    await writeFile(path.join(isolated.ws, "workflow.json"), JSON.stringify(raw));
  });
  afterEach(async () => {
    await rm(isolated.dir, { recursive: true, force: true });
  });
  function invoke(file = "workflow.json", extra: string[] = []) {
    return runCheaper(["workflow", "interface", file, "--cwd", isolated.ws, ...extra], {
      env: isolated.env,
    });
  }

  it("returns compact public JSON with explicit cwd and absolute paths", async () => {
    for (const file of ["workflow.json", path.join(isolated.ws, "workflow.json")]) {
      const r = await invoke(file, ["--quiet"]);
      expect(r.code).toBe(0);
      const envelope = expectEnvelopeOnly(r.stdout);
      expect(envelope).toMatchObject({ ok: true, worker: null });
      expect(envelope.result).toEqual(expected);
      expect(r.stdout.trim().split("\n")).toHaveLength(1);
      expect(r.stdout).not.toMatch(/privateScan|PRIVATE|needs|\$from|prompt|tool/);
      expect(r.stderr).toBe("");
    }
  });

  it.each([
    ["missing.json", "INVALID_REQUEST"],
    [".", "INVALID_REQUEST"],
    ["../workflow.json", "INVALID_REQUEST"],
    [".env", "INVALID_REQUEST"],
  ])("fails closed for %s using the standard error envelope", async (file, code) => {
    const r = await invoke(file);
    expect(r.code).toBe(2);
    expect(expectEnvelopeOnly(r.stdout)).toMatchObject({
      ok: false,
      worker: null,
      error: { code, exitCode: 2 },
    });
  });

  it("rejects malformed JSON and invalid schema without disclosing source", async () => {
    for (const [source, code] of [
      ["PRIVATE malformed", "INVALID_INPUT"],
      [JSON.stringify({ ...raw, schemaVersion: 2 }), "INVALID_REQUEST"],
    ]) {
      await writeFile(path.join(isolated.ws, "workflow.json"), source as string);
      const r = await invoke();
      expect(r.code).toBe(2);
      expect(expectEnvelopeOnly(r.stdout)).toMatchObject({ ok: false, error: { code } });
      expect(r.stdout + r.stderr).not.toContain("PRIVATE");
    }
  });

  it("honors policy deny paths", async () => {
    await mkdir(path.join(isolated.ws, ".cheaper"));
    await writeFile(
      path.join(isolated.ws, ".cheaper", "policy.yaml"),
      "version: 1\nfilesystem:\n  denyPaths: [workflow.json]\n",
    );
    const r = await invoke();
    expect(r.code).toBe(2);
    expect(expectEnvelopeOnly(r.stdout)).toMatchObject({
      ok: false,
      error: { code: "INVALID_REQUEST" },
    });
  });

  it("uses existing text output for success and errors", async () => {
    const success = await invoke("workflow.json", ["--format", "text"]);
    expect(success.code).toBe(0);
    expect(success.stdout).toContain('"required": true');
    expect(success.stdout).not.toContain("privateScan");
    const error = await invoke("missing.json", ["--format", "text"]);
    expect(error.code).toBe(2);
    expect(error.stdout).toContain("INVALID_REQUEST");
  });

  it("rejects invocation options and advertises the command in help", async () => {
    const rejected = await invoke("workflow.json", ["--input", "-"]);
    expect(rejected.code).toBe(2);
    expect(expectEnvelopeOnly(rejected.stdout)).toMatchObject({
      ok: false,
      error: { code: "INVALID_REQUEST" },
    });
    const help = await runCheaper(["--help"], { env: isolated.env });
    expect(help.code).toBe(0);
    expect(help.stdout).toContain("workflow interface <path>");
  });
});
