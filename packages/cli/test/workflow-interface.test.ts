import { mkdir, readFile, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { parseCliArgs } from "../src/args.js";
import { runCli } from "../src/cli.js";
import { defaultBuiltinDir } from "../src/count.js";
import { makeIsolatedEnv } from "./e2e/helpers.js";

export function workflowDefinition() {
  return {
    schemaVersion: 1,
    inputs: {
      target: { type: "string", minLength: 1 },
      limit: { type: "integer", minimum: 1, default: 3 },
    },
    jobs: {
      privateScan: { kind: "tool", tool: "read", input: { path: { $input: "target" }, limit: 7 } },
      privateWorker: {
        kind: "worker",
        worker: "bulk-reader",
        input: { task: "PRIVATE TASK", paths: ["PRIVATE PATH"] },
        needs: ["privateScan"],
      },
    },
    outputs: {
      result: { schema: { type: "object" }, value: { $from: "privateWorker", path: "" } },
    },
  };
}

export const publicInterface = {
  schemaVersion: 1,
  inputs: {
    limit: { schema: { type: "integer", minimum: 1, default: 3 }, required: false },
    target: { schema: { type: "string", minLength: 1 }, required: true },
  },
  outputs: { result: { schema: { type: "object" } } },
};

describe("workflow interface utility", () => {
  let isolated: Awaited<ReturnType<typeof makeIsolatedEnv>>;
  beforeEach(async () => {
    isolated = await makeIsolatedEnv();
    await writeFile(path.join(isolated.ws, "workflow.json"), JSON.stringify(workflowDefinition()));
  });
  afterEach(async () => {
    await rm(isolated.dir, { recursive: true, force: true });
  });

  async function invoke(extra: string[] = [], file = "workflow.json") {
    let stdout = "";
    let stderr = "";
    const stdin = {
      [Symbol.asyncIterator](): AsyncIterator<string> {
        throw new Error("stdin must not be read");
      },
    };
    const createModelRuntime = vi.fn(() => {
      throw new Error("model must not be created");
    });
    const runWorker = vi.fn(() => {
      throw new Error("worker must not run");
    });
    const code = await runCli(
      ["workflow", "interface", file, ...extra],
      {
        cwd: isolated.ws,
        env: isolated.env,
        stdin,
        stdout: {
          write: (s) => {
            stdout += s;
          },
        },
        stderr: {
          write: (s) => {
            stderr += s;
          },
        },
      },
      { createModelRuntime, runWorker },
    );
    expect(createModelRuntime).not.toHaveBeenCalled();
    expect(runWorker).not.toHaveBeenCalled();
    return { code, stdout, stderr };
  }

  it("returns only the canonical public interface without invocation values or side effects", async () => {
    const r = await invoke(["--quiet"]);
    expect(r.code).toBe(0);
    const envelope = JSON.parse(r.stdout);
    expect(envelope).toMatchObject({ ok: true, worker: null, result: publicInterface });
    expect(envelope.result).toEqual(publicInterface);
    expect(r.stdout.trim().split("\n")).toHaveLength(1);
    expect(r.stdout).not.toMatch(/privateScan|privateWorker|PRIVATE|needs|\$from|prompt|tool/);
    expect(r.stderr).toBe("");
    await expect(
      readFile(path.join(isolated.env.CHEAPER_CONFIG_DIR as string, "usage.jsonl")),
    ).rejects.toMatchObject({ code: "ENOENT" });
    await expect(
      readFile(path.join(isolated.env.CHEAPER_CONFIG_DIR as string, "pi", "models.json")),
    ).rejects.toMatchObject({ code: "ENOENT" });
  });

  it("uses existing text success/error formatting and exit codes", async () => {
    const success = await invoke(["--format", "text"]);
    expect(success.code).toBe(0);
    expect(success.stdout).toContain('"required": true');
    const failure = await invoke(["--format", "text"], "missing.json");
    expect(failure.code).toBe(2);
    expect(failure.stdout).toContain("INVALID_REQUEST");
    expect(failure.stderr).toContain("error INVALID_REQUEST");
  });

  it.each([
    ["missing.json", "INVALID_REQUEST"],
    [".", "INVALID_REQUEST"],
    ["../workflow.json", "INVALID_REQUEST"],
    [".env", "INVALID_REQUEST"],
  ])("fails closed for %s", async (file, code) => {
    const r = await invoke([], file);
    expect(r.code).toBe(2);
    expect(JSON.parse(r.stdout)).toMatchObject({
      ok: false,
      worker: null,
      error: { code, exitCode: 2 },
    });
  });

  it("sanitizes malformed JSON and rejects invalid definitions", async () => {
    await writeFile(path.join(isolated.ws, "workflow.json"), "PRIVATE malformed");
    const malformed = await invoke();
    expect(malformed.code).toBe(2);
    expect(JSON.parse(malformed.stdout).error.code).toBe("INVALID_INPUT");
    expect(malformed.stdout + malformed.stderr).not.toContain("PRIVATE");
    await writeFile(
      path.join(isolated.ws, "workflow.json"),
      JSON.stringify({ ...workflowDefinition(), schemaVersion: 2 }),
    );
    const invalid = await invoke();
    expect(invalid.code).toBe(2);
    expect(JSON.parse(invalid.stdout).error.code).toBe("INVALID_REQUEST");
  });

  it("preserves structural/schema/default error precedence over broken Worker metadata", async () => {
    await mkdir(path.join(isolated.ws, ".cheaper", "workers"), { recursive: true });
    await writeFile(
      path.join(isolated.ws, ".cheaper", "workers", "bulk-reader.md"),
      "broken metadata",
    );
    for (const invalid of [
      { ...workflowDefinition(), schemaVersion: 2 },
      { ...workflowDefinition(), inputs: { x: { type: "integer", default: "bad" } } },
    ]) {
      await writeFile(path.join(isolated.ws, "workflow.json"), JSON.stringify(invalid));
      const r = await invoke();
      expect(JSON.parse(r.stdout).error.code).toBe("INVALID_REQUEST");
    }
    await writeFile(path.join(isolated.ws, "workflow.json"), JSON.stringify(workflowDefinition()));
    const metadata = await invoke();
    expect(metadata.code).not.toBe(0);
    expect(JSON.parse(metadata.stdout).error.code).toBe("INVALID_WORKER_DEFINITION");
  });

  it("uses canonical definition rejection for missing and invalid Worker names", async () => {
    for (const name of ["missing", "../bad"]) {
      const raw = workflowDefinition();
      raw.jobs.privateWorker.worker = name;
      await writeFile(path.join(isolated.ws, "workflow.json"), JSON.stringify(raw));
      const r = await invoke();
      expect(r.code).toBe(2);
      expect(JSON.parse(r.stdout).error.code).toBe("INVALID_REQUEST");
    }
  });

  it("matches builtin metadata without exposing its prompt", async () => {
    expect(
      await readFile(path.join(defaultBuiltinDir, "workers", "bulk-reader.md"), "utf8"),
    ).toContain("You are");
    expect((await invoke()).stdout).not.toContain("You are");
  });
});

describe("workflow interface args", () => {
  it("accepts only the path and utility options", () => {
    expect(
      parseCliArgs([
        "workflow",
        "interface",
        "file.json",
        "--cwd",
        "ws",
        "--format",
        "text",
        "--quiet",
      ]),
    ).toEqual({
      kind: "workflow-interface",
      path: "file.json",
      cwd: "ws",
      format: "text",
      quiet: true,
    });
  });
  it.each([
    ["workflow"],
    ["workflow", "run", "file.json"],
    ["workflow", "interface"],
    ["workflow", "interface", "file.json", "extra"],
    ...[
      "input",
      "timeout",
      "model",
      "worker-file",
      "provider",
      "worker",
      "tier",
      "since",
      "compare-model",
      "method",
      "type",
    ].map((option) => ["workflow", "interface", "file.json", `--${option}`, "-"]),
  ])("rejects unrelated arguments %j", (...args) => {
    expect(() => parseCliArgs(args)).toThrowError(
      expect.objectContaining({ code: "INVALID_REQUEST" }),
    );
  });
});
