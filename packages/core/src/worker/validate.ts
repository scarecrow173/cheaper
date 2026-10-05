import path from "node:path";
import { CheaperError } from "../errors.js";
import { isTier, type Tier } from "../model/tier.js";
import { normalizeToolNames, type ToolName } from "../tools/vocabulary.js";
import type { ParsedWorkerFile } from "./parse.js";
import {
  type CompiledSchema,
  compileSchema,
  loadSchema,
  parseSchemaRef,
  type SchemaRef,
} from "./schema.js";

export type WorkerOrigin = "project" | "user" | "builtin" | "explicit";

export interface WorkerDefinition {
  name: string;
  description: string;
  tools: ToolName[];
  cheaper: {
    model: {
      tier: Tier;
      /**
       * workload profile（issue #46）。`outputPerInput` は input
       * 1 token あたりの output token 比率で auto tier 分類の
       * effectiveCost に使う。未指定時は `DEFAULT_OUTPUT_PER_INPUT`。
       */
      workload?: { outputPerInput: number };
    };
    input?: { schema: SchemaRef };
    output: { schema: SchemaRef };
    /**
     * mutation を制限する input field 名（SPEC-0010）。
     * 指定された field は string[] または `{path}` object[] で、
     * その path 群以外への write/edit/mkdir は tool error になる。
     */
    writeScope?: string;
  };
  instructions: string;
  sourcePath: string;
  origin: WorkerOrigin;
  inputSchema?: CompiledSchema;
  outputSchema: CompiledSchema;
}

const NAME_PATTERN = /^[a-z0-9][a-z0-9-]*$/;
const CHEAPER_KEYS = new Set(["model", "input", "output", "writeScope"]);
const MODEL_KEYS = new Set(["tier", "workload"]);
const WORKLOAD_KEYS = new Set(["outputPerInput"]);
const IO_KEYS = new Set(["schema"]);

function isPlainObject(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

function invalid(sourcePath: string, reason: string): CheaperError {
  return new CheaperError("INVALID_WORKER_DEFINITION", `invalid worker definition: ${reason}`, {
    details: { sourcePath, reason },
  });
}

function requireStrictKeys(
  obj: Record<string, unknown>,
  allowed: Set<string>,
  where: string,
  sourcePath: string,
) {
  for (const key of Object.keys(obj)) {
    if (!allowed.has(key)) throw invalid(sourcePath, `unknown-field-${where}-${key}`);
  }
}

export async function validateWorkerDefinition(
  parsed: ParsedWorkerFile,
  ctx: { sourcePath: string; origin: WorkerOrigin },
): Promise<WorkerDefinition> {
  const { sourcePath, origin } = ctx;
  const fm = parsed.frontmatter;
  if (!isPlainObject(fm)) throw invalid(sourcePath, "frontmatter-not-object");

  const name = fm.name;
  if (typeof name !== "string" || name.length === 0) throw invalid(sourcePath, "name-missing");
  if (!NAME_PATTERN.test(name)) {
    throw new CheaperError("INVALID_WORKER_NAME", `invalid worker name: ${name}`, {
      details: { sourcePath, name },
    });
  }
  const stem = path.basename(sourcePath, path.extname(sourcePath));
  if (stem !== name) {
    throw new CheaperError(
      "WORKER_NAME_MISMATCH",
      `worker name ${name} does not match file stem ${stem}`,
      {
        details: { sourcePath, name, stem },
      },
    );
  }

  if (typeof fm.description !== "string" || fm.description.trim().length === 0) {
    throw invalid(sourcePath, "description-missing");
  }
  const tools = normalizeToolNames(fm.tools);

  const cheaper = fm.cheaper;
  if (!isPlainObject(cheaper)) throw invalid(sourcePath, "cheaper-missing");
  requireStrictKeys(cheaper, CHEAPER_KEYS, "cheaper", sourcePath);

  const model = cheaper.model;
  if (!isPlainObject(model)) throw invalid(sourcePath, "model-missing");
  requireStrictKeys(model, MODEL_KEYS, "model", sourcePath);
  if (!isTier(model.tier)) throw invalid(sourcePath, "tier-invalid");
  const tier = model.tier;

  const workload = model.workload;
  let outputPerInput: number | undefined;
  if (workload !== undefined) {
    if (!isPlainObject(workload)) throw invalid(sourcePath, "workload-invalid");
    requireStrictKeys(workload, WORKLOAD_KEYS, "workload", sourcePath);
    const opi = workload.outputPerInput;
    if (typeof opi !== "number" || !Number.isFinite(opi) || opi < 0) {
      throw invalid(sourcePath, "workload-outputPerInput-invalid");
    }
    outputPerInput = opi;
  }

  const output = cheaper.output;
  if (!isPlainObject(output)) throw invalid(sourcePath, "output-missing");
  requireStrictKeys(output, IO_KEYS, "output", sourcePath);
  const outputRef = parseSchemaRef(output.schema, "output", sourcePath);

  let inputRef: SchemaRef | undefined;
  if (cheaper.input !== undefined) {
    if (!isPlainObject(cheaper.input)) throw invalid(sourcePath, "input-invalid");
    requireStrictKeys(cheaper.input, IO_KEYS, "input", sourcePath);
    inputRef = parseSchemaRef(cheaper.input.schema, "input", sourcePath);
  }

  const baseDir = path.dirname(sourcePath);
  const outputSchema = compileSchema(await loadSchema(outputRef, baseDir, "output"), "output");
  const inputSchema = inputRef
    ? compileSchema(await loadSchema(inputRef, baseDir, "input"), "input")
    : undefined;

  const writeScope = cheaper.writeScope;
  if (writeScope !== undefined && (typeof writeScope !== "string" || writeScope.length === 0)) {
    throw invalid(sourcePath, "writeScope-invalid");
  }

  const def: WorkerDefinition = {
    name,
    description: fm.description,
    tools,
    cheaper: {
      model: {
        tier,
        ...(outputPerInput !== undefined ? { workload: { outputPerInput } } : {}),
      },
      output: { schema: outputRef },
    },
    instructions: parsed.body,
    sourcePath,
    origin,
    outputSchema,
  };
  if (inputRef !== undefined) def.cheaper.input = { schema: inputRef };
  if (inputSchema !== undefined) def.inputSchema = inputSchema;
  if (writeScope !== undefined) def.cheaper.writeScope = writeScope;
  return def;
}

/** workspace を mutate しうる tool。直接比較可否の判定に使う。 */
const MUTATING_TOOL_NAMES: ReadonlySet<ToolName> = new Set(["write", "edit", "process"]);

/** mutation capability の正本。resource inference と accounting で同じ集合を使う。 */
export function isMutatingTool(name: ToolName): boolean {
  return MUTATING_TOOL_NAMES.has(name);
}

/**
 * measured offload（`savings`）の source↔returned が「親 model の
 * 直接実行」と比較可能な workload の worker か（SPEC-034rzGnCaRQc5ceI95lf5i
 * requirement 2）。write 系 tool または `writeScope` を持つ mutation 系
 * worker（code-writer / doc-writer 等）では、返却 result が ingest 内容の
 * 圧縮表現にならないため直接比較は成立せず false を返す。
 * read-only worker（bulk-reader 等）のみ true。
 */
export function isDirectlyComparableWorker(worker: WorkerDefinition): boolean {
  return (
    worker.cheaper.writeScope === undefined &&
    !worker.tools.some((tool) => MUTATING_TOOL_NAMES.has(tool))
  );
}

export function validateWorkerInput(def: WorkerDefinition, input: unknown): void {
  if (def.inputSchema === undefined) return;
  const result = def.inputSchema.validate(input);
  if (!result.ok) {
    throw new CheaperError("INVALID_INPUT", `input does not match worker input schema`, {
      details: { errors: result.errors },
    });
  }
}
