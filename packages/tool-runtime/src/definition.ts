import { CheaperError, compileSchema, type ToolName } from "@cheaper/core";
import type { Static, TSchema } from "typebox";
import type { CanonicalToolResult } from "./catalog.js";

/** Pi に依存しない内部 descriptor。公開実行入口は registry.execute。 */
export interface CanonicalToolDefinition<T extends TSchema = TSchema> {
  name: ToolName;
  label: string;
  description: string;
  parameters: T;
  execute(input: Static<T>, signal?: AbortSignal): Promise<CanonicalToolResult>;
}

export function defineCanonicalTool<T extends TSchema>(
  definition: CanonicalToolDefinition<T>,
): CanonicalToolDefinition<T> {
  return definition;
}

/** validator は構築時に compile、入力検証は実行入口で一回。default は適用しない。 */
export function createCanonicalExecutor(definition: CanonicalToolDefinition) {
  const schema = compileSchema({ ...definition.parameters }, "input");
  return async (input: unknown, signal?: AbortSignal): Promise<CanonicalToolResult> => {
    validateToolInput(definition.name, schema, input);
    return definition.execute(input, signal);
  };
}

export function validateToolInput(
  name: string,
  schema: ReturnType<typeof compileSchema>,
  input: unknown,
): void {
  const result = schema.validate(input);
  if (!result.ok)
    throw new CheaperError("INVALID_INPUT", `invalid ${name} input`, {
      details: { tool: name, errors: result.errors },
    });
}
