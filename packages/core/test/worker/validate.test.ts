import { writeFile } from "node:fs/promises";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { CheaperError } from "../../src/errors.js";
import { parseWorkerFile } from "../../src/worker/parse.js";
import { validateWorkerDefinition, validateWorkerInput } from "../../src/worker/validate.js";
import { cleanupTmpDir, makeTmpDir } from "../fixtures/tmp.js";

const OUTPUT_SCHEMA = { type: "object", required: ["ok"], properties: { ok: { type: "boolean" } } };

function workerContent(overrides: Record<string, unknown> = {}): string {
  const fm = {
    name: "w",
    description: "desc",
    tools: ["read"],
    cheaper: {
      model: { tier: "low" },
      output: { schema: { inline: OUTPUT_SCHEMA } },
    },
    ...overrides,
  };
  const yaml = Object.entries(fm)
    .map(([k, v]) => `${k}: ${JSON.stringify(v)}`)
    .join("\n");
  return `---\n${yaml}\n---\nDo the thing.\n`;
}

async function validate(
  content: string,
  dir: string,
  name = "w",
): Promise<ReturnType<typeof validateWorkerDefinition>> {
  const sourcePath = path.join(dir, `${name}.md`);
  await writeFile(sourcePath, content);
  const parsed = parseWorkerFile(content, sourcePath);
  return validateWorkerDefinition(parsed, { sourcePath, origin: "project" });
}

describe("validateWorkerDefinition", () => {
  let dir: string;
  beforeEach(async () => {
    dir = await makeTmpDir();
  });
  afterEach(async () => {
    await cleanupTmpDir(dir);
  });

  it("accepts a minimal valid worker", async () => {
    const def = await validate(workerContent(), dir);
    expect(def.name).toBe("w");
    expect(def.tools).toEqual(["read"]);
    expect(def.cheaper.model.tier).toBe("low");
    expect(def.instructions).toBe("Do the thing.");
    expect(def.origin).toBe("project");
    expect(def.sourcePath).toBe(path.join(dir, "w.md"));
    expect(def.outputSchema.validate({ ok: true })).toEqual({ ok: true });
    expect(def.inputSchema).toBeUndefined();
  });

  it("ignores unknown top-level fields", async () => {
    const def = await validate(workerContent({ extra: "ignored" }), dir);
    expect(def.name).toBe("w");
  });

  it("rejects invalid name", async () => {
    await expect(validate(workerContent({ name: "Bad_Name" }), dir)).rejects.toMatchObject({
      code: "INVALID_WORKER_NAME",
    });
  });

  it("rejects stem/name mismatch", async () => {
    const sourcePath = path.join(dir, "other.md");
    const parsed = parseWorkerFile(workerContent(), sourcePath);
    await expect(
      validateWorkerDefinition(parsed, { sourcePath, origin: "project" }),
    ).rejects.toMatchObject({ code: "WORKER_NAME_MISMATCH" });
  });

  it.each([
    ["name", ""],
    ["description", ""],
  ])("rejects empty required field %s", async (key, value) => {
    await expect(validate(workerContent({ [key]: value }), dir)).rejects.toMatchObject({
      code: "INVALID_WORKER_DEFINITION",
    });
  });

  it("rejects missing tools / missing cheaper / missing output.schema", async () => {
    await expect(validate(workerContent({ tools: undefined }), dir)).rejects.toMatchObject({
      code: "INVALID_WORKER_DEFINITION",
    });
    await expect(validate(workerContent({ cheaper: undefined }), dir)).rejects.toMatchObject({
      code: "INVALID_WORKER_DEFINITION",
    });
    await expect(
      validate(workerContent({ cheaper: { model: { tier: "low" } } }), dir),
    ).rejects.toMatchObject({ code: "INVALID_WORKER_DEFINITION" });
  });

  it("rejects unknown keys inside cheaper: (strict)", async () => {
    await expect(
      validate(
        workerContent({
          cheaper: {
            model: { tier: "low" },
            output: { schema: { inline: OUTPUT_SCHEMA } },
            bogus: 1,
          },
        }),
        dir,
      ),
    ).rejects.toMatchObject({ code: "INVALID_WORKER_DEFINITION" });
    await expect(
      validate(
        workerContent({
          cheaper: {
            model: { tier: "low", extra: 1 },
            output: { schema: { inline: OUTPUT_SCHEMA } },
          },
        }),
        dir,
      ),
    ).rejects.toMatchObject({ code: "INVALID_WORKER_DEFINITION" });
  });

  it("rejects invalid tier", async () => {
    await expect(
      validate(
        workerContent({
          cheaper: { model: { tier: "ultra" }, output: { schema: { inline: OUTPUT_SCHEMA } } },
        }),
        dir,
      ),
    ).rejects.toMatchObject({ code: "INVALID_WORKER_DEFINITION" });
  });

  it("loads file-based output schema relative to worker dir", async () => {
    await writeFile(path.join(dir, "out.schema.json"), JSON.stringify(OUTPUT_SCHEMA));
    const def = await validate(
      workerContent({
        cheaper: { model: { tier: "medium" }, output: { schema: { file: "out.schema.json" } } },
      }),
      dir,
    );
    expect(def.outputSchema.validate({ ok: true })).toEqual({ ok: true });
  });

  it("validates optional input schema", async () => {
    const def = await validate(
      workerContent({
        cheaper: {
          model: { tier: "low" },
          input: {
            schema: {
              inline: { type: "object", required: ["q"], properties: { q: { type: "string" } } },
            },
          },
          output: { schema: { inline: OUTPUT_SCHEMA } },
        },
      }),
      dir,
    );
    expect(() => validateWorkerInput(def, { q: "x" })).not.toThrow();
    try {
      validateWorkerInput(def, { q: 1 });
      expect.unreachable();
    } catch (e) {
      const err = e as CheaperError;
      expect(err.code).toBe("INVALID_INPUT");
      expect(err.exitCode).toBe(2);
      expect(Array.isArray(err.details?.errors)).toBe(true);
    }
  });

  it("skips input validation when no input schema", async () => {
    const def = await validate(workerContent(), dir);
    expect(() => validateWorkerInput(def, { anything: true })).not.toThrow();
  });
});
