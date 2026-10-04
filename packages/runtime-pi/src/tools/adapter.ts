import {
  type CanonicalToolDefinition,
  type CanonicalToolResult,
  createCanonicalExecutor,
} from "@cheaper/tool-runtime";
import { defineTool, type ToolDefinition } from "@earendil-works/pi-coding-agent";

/** Pi は schema/metadata と同じ canonical execute を session に露出する。 */
export function toPiTool(
  definition: CanonicalToolDefinition,
  execute?: (input: unknown, signal?: AbortSignal) => Promise<CanonicalToolResult>,
): ToolDefinition {
  const call = execute ?? createCanonicalExecutor(definition);
  return defineTool({
    ...definition,
    async execute(_toolCallId, input, signal) {
      const result = await call(input, signal);
      return { ...result, details: result.details };
    },
  });
}
