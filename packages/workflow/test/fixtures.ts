import { compileSchema, type WorkerDefinition } from "@cheaper/core";
import type { ToolCatalog, ToolMetadata } from "@cheaper/tool-runtime";
import {
  WorkflowCompiler,
  type WorkflowCompilerPorts,
  type WorkflowDefinition,
} from "../src/index.js";

export const canonicalSchema = {
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
    details: { type: "object", additionalProperties: true },
  },
};
export function fixtures(overrides: Partial<WorkflowCompilerPorts> = {}) {
  const tools: ToolMetadata[] = [
    {
      name: "read",
      inputSchema: {
        type: "object",
        required: ["path"],
        additionalProperties: false,
        properties: { path: { type: "string" } },
      },
      outputSchema: canonicalSchema,
    },
    { name: "grep", inputSchema: {}, outputSchema: canonicalSchema },
    {
      name: "count_tokens",
      inputSchema: {
        type: "object",
        required: ["path"],
        additionalProperties: false,
        properties: { path: { type: "string" } },
      },
      outputSchema: canonicalSchema,
    },
  ];
  const worker: WorkerDefinition = {
    name: "bulk-reader",
    description: "fake validated metadata",
    tools: ["read"],
    cheaper: { model: { tier: "low" }, output: { schema: { inline: {} } } },
    instructions: "PRIVATE PROMPT",
    sourcePath: "fake/bulk-reader.md",
    origin: "builtin",
    inputSchema: compileSchema(
      {
        type: "object",
        required: ["task"],
        additionalProperties: false,
        properties: { task: { type: "string" } },
      },
      "input",
    ),
    outputSchema: compileSchema({}, "output"),
  };
  const catalog: ToolCatalog = {
    get: (name) => tools.find((tool) => tool.name === name),
    list: () => tools,
  };
  const ports: WorkflowCompilerPorts = {
    tools: catalog,
    workers: { get: (name) => (name === worker.name ? worker : undefined) },
    ...overrides,
  };
  return { compiler: new WorkflowCompiler(ports), ports, worker, tools };
}
export function definition(): WorkflowDefinition {
  return { schemaVersion: 1, inputs: {}, jobs: {}, outputs: {} };
}
