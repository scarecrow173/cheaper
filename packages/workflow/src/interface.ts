import { cloneJson, hasOwn } from "./json.js";
import { validateDefinition } from "./schema.js";
import type { WorkflowCompilerPorts, WorkflowInterface } from "./types.js";

/** definition 検証だけを行い、invocation/resource/execution を要求しない。 */
export function projectWorkflowInterface(
  raw: unknown,
  ports: WorkflowCompilerPorts,
): WorkflowInterface {
  const { definition } = validateDefinition(raw, ports);
  return {
    schemaVersion: 1,
    inputs: Object.fromEntries(
      Object.keys(definition.inputs)
        .sort()
        .map((name) => {
          const schema = definition.inputs[name] as Record<string, unknown>;
          return [
            name,
            {
              schema: cloneJson(schema) as Record<string, unknown>,
              required: !hasOwn(schema, "default"),
            },
          ];
        }),
    ),
    outputs: Object.fromEntries(
      Object.keys(definition.outputs)
        .sort()
        .map((name) => [
          name,
          { schema: cloneJson(definition.outputs[name]?.schema) as Record<string, unknown> },
        ]),
    ),
  };
}
