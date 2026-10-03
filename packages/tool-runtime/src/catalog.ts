import type { ToolName } from "@cheaper/core";

export type JsonValue = null | boolean | number | string | JsonValue[] | { [k: string]: JsonValue };
export interface ToolMetadata {
  name: ToolName;
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
