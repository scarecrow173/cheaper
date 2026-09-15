import { writeFile } from "node:fs/promises";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { CheaperError } from "../../src/errors.js";
import { compileSchema, loadSchema, parseSchemaRef } from "../../src/worker/schema.js";
import { cleanupTmpDir, makeTmpDir } from "../fixtures/tmp.js";

const SRC = "workers/w.md";

describe("parseSchemaRef", () => {
  it("accepts file ref", () => {
    expect(parseSchemaRef({ file: "s.json" }, "output", SRC)).toEqual({ file: "s.json" });
  });
  it("accepts inline ref", () => {
    const inline = { type: "object" };
    expect(parseSchemaRef({ inline }, "input", SRC)).toEqual({ inline });
  });
  it.each([{}, { file: "a.json", inline: {} }, "x", null, { file: 3 }, { inline: "x" }])(
    "rejects %j",
    (raw) => {
      try {
        parseSchemaRef(raw, "output", SRC);
        expect.unreachable();
      } catch (e) {
        expect((e as CheaperError).code).toBe("INVALID_WORKER_DEFINITION");
      }
    },
  );
});

describe("loadSchema / compileSchema", () => {
  let dir: string;
  beforeEach(async () => {
    dir = await makeTmpDir();
  });
  afterEach(async () => {
    await cleanupTmpDir(dir);
  });

  it("loads .json schema", async () => {
    await writeFile(path.join(dir, "s.json"), JSON.stringify({ type: "string" }));
    const s = await loadSchema({ file: "s.json" }, dir, "output");
    expect(s).toEqual({ type: "string" });
  });
  it("loads .yaml/.yml schema", async () => {
    await writeFile(path.join(dir, "s.yaml"), "type: string\n");
    expect(await loadSchema({ file: "s.yaml" }, dir, "output")).toEqual({ type: "string" });
    await writeFile(path.join(dir, "s.yml"), "type: number\n");
    expect(await loadSchema({ file: "s.yml" }, dir, "output")).toEqual({ type: "number" });
  });
  it("missing file maps to per-kind NOT_FOUND", async () => {
    await expect(loadSchema({ file: "no.json" }, dir, "output")).rejects.toMatchObject({
      code: "OUTPUT_SCHEMA_NOT_FOUND",
    });
    await expect(loadSchema({ file: "no.json" }, dir, "input")).rejects.toMatchObject({
      code: "INPUT_SCHEMA_NOT_FOUND",
    });
  });
  it("bad extension and parse failure map to INVALID_*_SCHEMA", async () => {
    await writeFile(path.join(dir, "s.txt"), "{}");
    await expect(loadSchema({ file: "s.txt" }, dir, "output")).rejects.toMatchObject({
      code: "INVALID_OUTPUT_SCHEMA",
    });
    await writeFile(path.join(dir, "bad.json"), "{not json");
    await expect(loadSchema({ file: "bad.json" }, dir, "input")).rejects.toMatchObject({
      code: "INVALID_INPUT_SCHEMA",
    });
  });
  it("inline ref returns the object itself", async () => {
    const inline = { type: "object" };
    expect(await loadSchema({ inline }, dir, "input")).toBe(inline);
  });
  it("compiled schema validates data", () => {
    const c = compileSchema(
      { type: "object", required: ["a"], properties: { a: { type: "string" } } },
      "output",
    );
    expect(c.validate({ a: "x" })).toEqual({ ok: true });
    const bad = c.validate({ a: 1 });
    expect(bad.ok).toBe(false);
    if (!bad.ok) {
      expect(bad.errors[0]).toHaveProperty("instancePath");
      expect(bad.errors[0]).toHaveProperty("message");
    }
  });
  it("rejects non-2020-12 $schema", () => {
    expect(() =>
      compileSchema(
        { $schema: "http://json-schema.org/draft-07/schema#", type: "object" },
        "output",
      ),
    ).toThrowError(expect.objectContaining({ code: "INVALID_OUTPUT_SCHEMA" }));
    expect(() =>
      compileSchema(
        { $schema: "http://json-schema.org/draft-07/schema#", type: "object" },
        "input",
      ),
    ).toThrowError(expect.objectContaining({ code: "INVALID_INPUT_SCHEMA" }));
  });
  it("rejects invalid JSON Schema", () => {
    expect(() => compileSchema({ type: "nonsense" }, "output")).toThrowError(
      expect.objectContaining({ code: "INVALID_OUTPUT_SCHEMA" }),
    );
  });
});
