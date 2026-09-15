import { readFile } from "node:fs/promises";
import path from "node:path";
import Ajv2020Ns, { type ValidateFunction } from "ajv/dist/2020.js";
import { parse as parseYaml } from "yaml";
import { CheaperError, type ErrorCode } from "../errors.js";

export type SchemaRef = { file: string } | { inline: Record<string, unknown> };

export type SchemaKind = "input" | "output";

const INVALID_CODE: Record<SchemaKind, ErrorCode> = {
  input: "INVALID_INPUT_SCHEMA",
  output: "INVALID_OUTPUT_SCHEMA",
};

const Ajv2020 = Ajv2020Ns.default;

const NOT_FOUND_CODE: Record<SchemaKind, ErrorCode> = {
  input: "INPUT_SCHEMA_NOT_FOUND",
  output: "OUTPUT_SCHEMA_NOT_FOUND",
};

const JSON_SCHEMA_2020 = "https://json-schema.org/draft/2020-12/schema";

function isPlainObject(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

export function parseSchemaRef(raw: unknown, kind: SchemaKind, sourcePath: string): SchemaRef {
  const fail = (reason: string): never => {
    throw new CheaperError("INVALID_WORKER_DEFINITION", `invalid ${kind} schema ref: ${reason}`, {
      details: { sourcePath, reason, kind },
    });
  };
  if (!isPlainObject(raw)) return fail("schema-not-object");
  const hasFile = "file" in raw;
  const hasInline = "inline" in raw;
  if (hasFile === hasInline) return fail("schema-exactly-one");
  if (hasFile) {
    if (typeof raw.file !== "string" || raw.file.length === 0)
      return fail("schema-file-not-string");
    return { file: raw.file };
  }
  if (!isPlainObject(raw.inline)) return fail("schema-inline-not-object");
  return { inline: raw.inline };
}

export async function loadSchema(
  ref: SchemaRef,
  baseDir: string,
  kind: SchemaKind,
): Promise<Record<string, unknown>> {
  if ("inline" in ref) return ref.inline;
  const ext = path.extname(ref.file).toLowerCase();
  if (ext !== ".json" && ext !== ".yaml" && ext !== ".yml") {
    throw new CheaperError(INVALID_CODE[kind], `unsupported schema extension: ${ref.file}`, {
      details: { file: ref.file, reason: "schema-extension" },
    });
  }
  const filePath = path.resolve(baseDir, ref.file);
  let text: string;
  try {
    text = await readFile(filePath, "utf8");
  } catch (e) {
    throw new CheaperError(NOT_FOUND_CODE[kind], `schema file not found: ${ref.file}`, {
      details: { file: ref.file, path: filePath },
      cause: e,
    });
  }
  try {
    const parsed: unknown = ext === ".json" ? JSON.parse(text) : parseYaml(text);
    if (!isPlainObject(parsed)) throw new Error("schema is not an object");
    return parsed;
  } catch (e) {
    throw new CheaperError(INVALID_CODE[kind], `schema parse failed: ${ref.file}`, {
      details: { file: ref.file, reason: "schema-parse" },
      cause: e,
    });
  }
}

export interface CompiledSchema {
  schema: Record<string, unknown>;
  validate(
    data: unknown,
  ): { ok: true } | { ok: false; errors: Array<{ instancePath: string; message: string }> };
}

export function compileSchema(schema: Record<string, unknown>, kind: SchemaKind): CompiledSchema {
  const declared = schema.$schema;
  if (declared !== undefined && declared !== JSON_SCHEMA_2020) {
    throw new CheaperError(INVALID_CODE[kind], `unsupported $schema: ${String(declared)}`, {
      details: { reason: "schema-draft", $schema: declared },
    });
  }
  const ajv = new Ajv2020({ strict: true, allErrors: true });
  let validateFn: ValidateFunction;
  try {
    ajv.validateSchema(schema, true);
    validateFn = ajv.compile(schema);
  } catch (e) {
    throw new CheaperError(INVALID_CODE[kind], `schema compile failed`, {
      details: { reason: "schema-compile" },
      cause: e,
    });
  }
  return {
    schema,
    validate(data: unknown) {
      if (validateFn(data)) return { ok: true };
      return {
        ok: false,
        errors: (validateFn.errors ?? []).map((err) => ({
          instancePath: err.instancePath,
          message: err.message ?? "validation error",
        })),
      };
    },
  };
}
