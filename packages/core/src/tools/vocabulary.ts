import { CheaperError } from "../errors.js";

export const TOOL_NAMES = [
  "read",
  "grep",
  "find",
  "ls",
  "write",
  "edit",
  "count_tokens",
  "process",
] as const;

export type ToolName = (typeof TOOL_NAMES)[number];

export const TOOL_ALIASES: Readonly<Record<string, ToolName>> = {
  Read: "read",
  Grep: "grep",
  Glob: "find",
  Write: "write",
  Edit: "edit",
};

const CANONICAL: ReadonlySet<string> = new Set(TOOL_NAMES);

export function normalizeToolNames(raw: unknown): ToolName[] {
  if (!Array.isArray(raw) || raw.length === 0 || raw.some((v) => typeof v !== "string")) {
    throw new CheaperError("INVALID_WORKER_DEFINITION", "tools must be a non-empty string array", {
      details: { reason: "tools-non-empty-string-array" },
    });
  }
  const names = (raw as string[]).map((v) => {
    if (CANONICAL.has(v)) return v as ToolName;
    const aliased = TOOL_ALIASES[v];
    if (aliased !== undefined) return aliased;
    throw new CheaperError("UNKNOWN_TOOL", `unknown tool: ${v}`, {
      details: { tool: v, known: TOOL_NAMES },
    });
  });
  if (new Set(names).size !== names.length) {
    throw new CheaperError("INVALID_WORKER_DEFINITION", "tools contain duplicates", {
      details: { reason: "tools-duplicate" },
    });
  }
  return names;
}
