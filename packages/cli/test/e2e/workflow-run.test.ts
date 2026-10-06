import { mkdir, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { expectEnvelopeOnly, makeIsolatedEnv, runCheaper } from "./helpers.js";

describe("built CLI workflow run", () => {
  let isolated: Awaited<ReturnType<typeof makeIsolatedEnv>>;
  beforeEach(async () => {
    isolated = await makeIsolatedEnv();
  });
  afterEach(async () => {
    await rm(isolated.dir, { recursive: true, force: true });
  });

  it("emits one public envelope, preserves declared output and omits private job data", async () => {
    await writeFile(
      path.join(isolated.ws, "workflow.json"),
      JSON.stringify({
        schemaVersion: 1,
        inputs: {},
        jobs: { private: { kind: "tool", tool: "read", input: { path: "private.txt" } } },
        outputs: { present: { schema: { type: "boolean" }, value: { $literal: false } } },
      }),
    );
    await writeFile(path.join(isolated.ws, "private.txt"), "PRIVATE_E2E_SENTINEL");
    const r = await runCheaper(
      ["workflow", "run", "workflow.json", "--cwd", isolated.ws, "--quiet"],
      { env: isolated.env },
    );
    expect(r.code).toBe(0);
    expect(expectEnvelopeOnly(r.stdout)).toMatchObject({
      workflow: true,
      ok: true,
      worker: null,
      result: { present: false },
    });
    expect(r.stdout).not.toContain("PRIVATE_E2E_SENTINEL");
    expect(r.stderr).toBe("");
  });

  it("keeps workflow interface non-executing after sharing context preparation", async () => {
    await mkdir(path.join(isolated.ws, ".cheaper"), { recursive: true });
    await writeFile(
      path.join(isolated.ws, "workflow.json"),
      JSON.stringify({ schemaVersion: 1, inputs: {}, jobs: {}, outputs: {} }),
    );
    const r = await runCheaper(
      ["workflow", "interface", "workflow.json", "--cwd", isolated.ws, "--quiet"],
      { env: isolated.env },
    );
    expect(r.code).toBe(0);
    expect(expectEnvelopeOnly(r.stdout)).toMatchObject({
      ok: true,
      worker: null,
      result: { schemaVersion: 1 },
    });
    const configDir = isolated.env.CHEAPER_CONFIG_DIR;
    expect(configDir).toBeDefined();
    expect(
      await import("node:fs/promises")
        .then(({ access }) => access(path.join(configDir ?? "", "pi")))
        .then(
          () => true,
          () => false,
        ),
    ).toBe(false);
  });
});
