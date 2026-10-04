import path from "node:path";
import type { ResourceManifestDeclaration, ToolName } from "@cheaper/core";
import { Type } from "typebox";
import { countTokensSchema } from "./count-tokens.js";
import { findSchema } from "./find.js";
import { grepSchema } from "./grep.js";
import { readSchema } from "./read.js";

export type JsonValue = null | boolean | number | string | JsonValue[] | { [k: string]: JsonValue };
export interface ToolMetadata {
  name: ToolName;
  /** 実行せず input の既知アクセス範囲を宣言する。null は未知。 */
  resources?: (input: JsonValue) => ResourceManifestDeclaration;
  inputSchema: Record<string, unknown>;
  outputSchema: Record<string, unknown>;
}
// 副作用なし。Pi session / model 初期化をしない
export interface ToolCatalog {
  get(name: string): ToolMetadata | undefined;
  list(): readonly ToolMetadata[];
}
export interface CanonicalToolResult {
  content: Array<{ type: "text"; text: string }>;
  details?: JsonValue;
}

const writeSchema = Type.Object({
  path: Type.String({ description: "Path to the file to write (relative or absolute)" }),
  content: Type.String({ description: "Content to write to the file" }),
});
const editSchema = Type.Object(
  {
    path: Type.String({ description: "Path to the file to edit (relative or absolute)" }),
    edits: Type.Array(
      Type.Object(
        {
          oldText: Type.String({
            description:
              "Exact text for one targeted replacement. It must be unique in the original file and must not overlap with any other edits[].oldText in the same call.",
          }),
          newText: Type.String({ description: "Replacement text for this targeted edit." }),
        },
        {},
      ),
      {
        description:
          "One or more targeted replacements. Each edit is matched against the original file, not incrementally. Do not include overlapping or nested edits. If two changes touch the same block or nearby lines, merge them into one edit instead.",
      },
    ),
  },
  {},
);
const lsSchema = Type.Object({
  path: Type.Optional(
    Type.String({ description: "Directory to list (default: current directory)" }),
  ),
  limit: Type.Optional(
    Type.Number({ description: "Maximum number of entries to return (default: 500)" }),
  ),
});

const resultSchema: Record<string, unknown> = {
  type: "object",
  required: ["content"],
  additionalProperties: false,
  properties: {
    content: {
      type: "array",
      items: {
        type: "object",
        required: ["type", "text"],
        additionalProperties: false,
        properties: { type: { const: "text" }, text: { type: "string" } },
      },
    },
    details: { $ref: "#/$defs/json" },
  },
  $defs: {
    json: {
      anyOf: [
        { type: "null" },
        { type: "boolean" },
        { type: "number" },
        { type: "string" },
        { type: "array", items: { $ref: "#/$defs/json" } },
        { type: "object", additionalProperties: { $ref: "#/$defs/json" } },
      ],
    },
  },
};

const schemas = {
  read: readSchema,
  write: writeSchema,
  edit: editSchema,
  ls: lsSchema,
  grep: grepSchema,
  find: findSchema,
  count_tokens: countTokensSchema,
};

const readDetails = Type.Object({
  startLine: Type.Integer(),
  endLine: Type.Integer(),
  eof: Type.Boolean(),
  truncatedBy: Type.Union([Type.Literal("limit"), Type.Literal("bytes"), Type.Null()]),
  continuation: Type.Optional(
    Type.Object({ offset: Type.Integer(), byteInLine: Type.Optional(Type.Integer()) }),
  ),
  totalLines: Type.Optional(Type.Integer()),
});
const countDetails = Type.Object({
  path: Type.String(),
  tokens: Type.Integer({ minimum: 0 }),
  bytes: Type.Integer({ minimum: 0 }),
  tokenizer: Type.Object({
    name: Type.String(),
    implementation: Type.String(),
    version: Type.String(),
  }),
});
function outputSchema(name: ToolName): Record<string, unknown> {
  const result = structuredClone(resultSchema);
  if (name === "read" || name === "count_tokens") {
    const properties = result.properties as Record<string, unknown>;
    properties.details = name === "read" ? readDetails : countDetails;
    result.required = ["content", "details"];
  }
  return result;
}

function resources(name: ToolName, input: JsonValue): ResourceManifestDeclaration {
  const subtree = name === "ls" || name === "grep" || name === "find";
  const value =
    input !== null && typeof input === "object" && !Array.isArray(input)
      ? (input.path ?? (subtree ? "." : undefined))
      : undefined;
  // absolute input は D5 relative declaration に変換せず未知として公開する。
  const known =
    typeof value === "string" &&
    value.length > 0 &&
    !path.isAbsolute(value) &&
    !path.win32.isAbsolute(value) &&
    !/^[a-z]:/i.test(value);
  const scope = known
    ? [{ path: value, scope: subtree ? ("subtree" as const) : ("exact" as const) }]
    : null;
  return {
    reads: name === "write" ? [] : scope,
    writes: name === "write" || name === "edit" ? scope : [],
    produces: [],
  };
}

/** metadata の取得に workspace/model/Pi/helper の構築は不要。caller の書換えを共有しない。 */
export function createToolCatalog(): ToolCatalog {
  const list = (): ToolMetadata[] =>
    Object.entries(schemas).map(([name, inputSchema]) => ({
      name: name as ToolName,
      resources: (input) => resources(name as ToolName, input),
      inputSchema: { ...structuredClone(inputSchema) },
      outputSchema: structuredClone(outputSchema(name as ToolName)),
    }));
  return {
    list,
    get(name) {
      return list().find((t) => t.name === name);
    },
  };
}
