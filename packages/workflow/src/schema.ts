import { CheaperError, type CompiledSchema, compileSchema, type WorkerDefinition } from "@cheaper/core";
import { parseBinding, scanBindings, validatePointerSchema } from "./bindings.js";
import { compileDag } from "./dag.js";
import { cloneJson, hasOwn, isObject } from "./json.js";
import type { DependencyEdge, JsonSchema, JsonValue, WorkflowCompilerPorts, WorkflowDefinition } from "./types.js";

const namedString = { type: "string", minLength: 1 };
const inputBinding = {
  type: "object", required: ["$input"], additionalProperties: false,
  properties: { $input: namedString },
};
const fromBinding = {
  type: "object", required: ["$from", "path"], additionalProperties: false,
  properties: { $from: namedString, path: { type: "string" } },
};
const literalBinding = {
  type: "object", required: ["$literal"], additionalProperties: false,
  properties: { $literal: true },
};
const declaration = {
  type: "object", required: ["path", "scope"], additionalProperties: false,
  properties: { path: { oneOf: [namedString, inputBinding] }, scope: { enum: ["exact", "subtree"] } },
};
const resources = {
  type: "object", additionalProperties: false,
  properties: Object.fromEntries(["reads", "writes", "produces"].map((key) => [key, { type: "array", items: declaration }])),
};
function jobSchema(kind: "tool" | "worker"): JsonSchema {
  return {
    type: "object", required: ["kind", kind, "input"], additionalProperties: false,
    properties: {
      kind: { const: kind }, [kind]: namedString, input: true,
      needs: { type: "array", items: namedString }, resources,
    },
  };
}
export const WORKFLOW_JSON_SCHEMA: JsonSchema = {
  $schema: "https://json-schema.org/draft/2020-12/schema",
  type: "object", required: ["schemaVersion", "inputs", "jobs", "outputs"], additionalProperties: false,
  properties: {
    schemaVersion: { const: 1 },
    inputs: { type: "object", propertyNames: namedString, additionalProperties: { type: "object" } },
    jobs: { type: "object", propertyNames: namedString, additionalProperties: { oneOf: [jobSchema("tool"), jobSchema("worker")] } },
    outputs: {
      type: "object", propertyNames: namedString,
      additionalProperties: {
        type: "object", required: ["schema", "value"], additionalProperties: false,
        properties: { schema: { type: "object" }, value: { oneOf: [inputBinding, fromBinding, literalBinding] } },
      },
    },
  },
};

const definitionSchema = compileSchema(WORKFLOW_JSON_SCHEMA, "input");
export interface JobValidation {
  input?: CompiledSchema;
  output: CompiledSchema;
  worker?: WorkerDefinition;
}
export interface ValidatedDefinition {
  definition: WorkflowDefinition;
  inputs: Map<string, CompiledSchema>;
  outputs: Map<string, CompiledSchema>;
  jobs: Map<string, JobValidation>;
  edges: DependencyEdge[];
}

function compileDefinitionSchema(schema: JsonSchema, kind: "input" | "output"): CompiledSchema {
  try {
    return compileSchema(schema, kind);
  } catch (cause) {
    throw new CheaperError("INVALID_REQUEST", `invalid workflow ${kind} schema`, { cause });
  }
}

function workerSnapshot(worker: WorkerDefinition): WorkerDefinition {
  // validated WorkerDefinition port の schema を snapshot から再 compile する。
  return {
    ...worker, tools: [...worker.tools],
    cheaper: structuredClone(worker.cheaper),
    ...(worker.inputSchema ? { inputSchema: compileDefinitionSchema(cloneJson(worker.inputSchema.schema) as JsonSchema, "input") } : {}),
    outputSchema: compileDefinitionSchema(cloneJson(worker.outputSchema.schema) as JsonSchema, "output"),
  };
}

export function validateDefinition(raw: unknown, ports: WorkflowCompilerPorts): ValidatedDefinition {
  const snapshot = cloneJson(raw);
  const structural = definitionSchema.validate(snapshot);
  if (!structural.ok) throw new CheaperError("INVALID_REQUEST", "invalid Workflow JSON v1", { details: { errors: structural.errors } });
  const definition = snapshot as unknown as WorkflowDefinition;
  const inputs = new Map<string, CompiledSchema>();
  const outputs = new Map<string, CompiledSchema>();
  const jobs = new Map<string, JobValidation>();
  const edges: DependencyEdge[] = [];
  for (const [name, schema] of Object.entries(definition.inputs)) {
    const validator = compileDefinitionSchema(schema, "input");
    if (hasOwn(schema, "default") && !validator.validate(schema.default).ok) throw new CheaperError("INVALID_REQUEST", `invalid input default: ${name}`);
    inputs.set(name, validator);
  }
  for (const [id, job] of Object.entries(definition.jobs)) {
    if (job.kind === "tool") {
      const tool = ports.tools.get(job.tool);
      if (!tool) throw new CheaperError("INVALID_REQUEST", `unknown tool: ${job.tool}`);
      jobs.set(id, {
        input: compileDefinitionSchema(cloneJson(tool.inputSchema) as JsonSchema, "input"),
        output: compileDefinitionSchema(cloneJson(tool.outputSchema) as JsonSchema, "output"),
      });
    } else {
      const metadata = ports.workers.get(job.worker);
      if (!metadata) throw new CheaperError("INVALID_REQUEST", `unknown worker: ${job.worker}`);
      for (const tool of metadata.tools) {
        if (!ports.tools.get(tool)) throw new CheaperError("INVALID_REQUEST", `unknown worker capability tool: ${tool}`);
      }
      const worker = workerSnapshot(metadata);
      jobs.set(id, { worker, ...(worker.inputSchema ? { input: worker.inputSchema } : {}), output: worker.outputSchema });
    }
  }
  function references(value: JsonValue, consumer?: string): void {
    const bindings = scanBindings(value);
    for (const name of bindings.inputs) {
      if (!hasOwn(definition.inputs, name)) throw new CheaperError("INVALID_REQUEST", `unknown input reference: ${name}`);
    }
    for (const binding of bindings.from) {
      const producer = jobs.get(binding.$from);
      if (!producer) throw new CheaperError("INVALID_REQUEST", `unknown job reference: ${binding.$from}`);
      validatePointerSchema(producer.output.schema, binding.path);
      if (consumer !== undefined) edges.push({ from: binding.$from, to: consumer });
    }
  }
  for (const [id, job] of Object.entries(definition.jobs)) {
    references(job.input, id);
    for (const from of job.needs ?? []) edges.push({ from, to: id });
    if (job.resources) {
      for (const declarations of Object.values(job.resources)) {
        for (const resource of declarations) {
          if (isObject(resource.path)) references(resource.path as JsonValue);
        }
      }
    }
  }
  for (const [name, output] of Object.entries(definition.outputs)) {
    parseBinding(output.value);
    references(output.value as JsonValue);
    outputs.set(name, compileDefinitionSchema(output.schema, "output"));
  }
  const dag = compileDag(Object.keys(definition.jobs), edges);
  return { definition, inputs, outputs, jobs, edges: dag.edges };
}
