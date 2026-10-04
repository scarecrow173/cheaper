import { CheaperError, type ResourceManifest, validateWorkerInput } from "@cheaper/core";
import { materializeBindings, scanBindings } from "./bindings.js";
import { compileDag } from "./dag.js";
import { projectWorkflowInterface } from "./interface.js";
import { cloneJson, hasOwn, isObject } from "./json.js";
import { type JobValidation, type ValidatedDefinition, validateDefinition } from "./schema.js";
import type {
  CompiledJob,
  CompiledWorkflow,
  DependencyEdge,
  JsonValue,
  ResourceInferenceJob,
  ScopeDeclaration,
  WorkflowCompilerPorts,
  WorkflowDefinition,
  WorkflowInterface,
  WorkflowJob,
  WorkflowJobState,
  WorkflowOutput,
  WorkflowResources,
} from "./types.js";

function validateJobInput(metadata: JobValidation, input: JsonValue): void {
  if (metadata.worker) {
    validateWorkerInput(metadata.worker, input);
  } else if (metadata.input) {
    const validation = metadata.input.validate(input);
    if (!validation.ok)
      throw new CheaperError("INVALID_INPUT", "input does not match tool input schema", {
        details: { errors: validation.errors },
      });
  }
}

function invocationInputs(validated: ValidatedDefinition, raw: unknown): Record<string, JsonValue> {
  const provided = cloneJson(raw, "INVALID_INPUT");
  if (!isObject(provided))
    throw new CheaperError("INVALID_INPUT", "workflow inputs must be an object");
  for (const key of Object.keys(provided)) {
    if (!hasOwn(validated.definition.inputs, key))
      throw new CheaperError("INVALID_INPUT", `unknown workflow input: ${key}`);
  }
  return Object.fromEntries(
    Object.keys(validated.definition.inputs)
      .sort()
      .map((name) => {
        const schema = validated.definition.inputs[name] as Record<string, unknown>;
        if (!hasOwn(provided, name) && !hasOwn(schema, "default"))
          throw new CheaperError("INVALID_INPUT", `required workflow input: ${name}`);
        const value = cloneJson(
          hasOwn(provided, name) ? provided[name] : schema.default,
          "INVALID_INPUT",
        );
        const result = validated.inputs.get(name)!.validate(value);
        if (!result.ok)
          throw new CheaperError("INVALID_INPUT", `invalid workflow input: ${name}`, {
            details: { errors: result.errors },
          });
        return [name, value];
      }),
  );
}

function materializeResources(
  resources: WorkflowResources,
  inputs: Record<string, JsonValue>,
): WorkflowResources {
  return Object.fromEntries(
    Object.entries(resources).map(([kind, declarations]) => [
      kind,
      (declarations as ScopeDeclaration[]).map((resource) => {
        const path =
          typeof resource.path === "string"
            ? resource.path
            : materializeBindings(resource.path, inputs);
        if (typeof path !== "string")
          throw new CheaperError("INVALID_INPUT", "resource path must resolve to a string");
        return { path, scope: resource.scope };
      }),
    ]),
  );
}

class Compiled implements CompiledWorkflow {
  schemaVersion = 1 as const;
  inputs: Record<string, JsonValue>;
  jobs: Record<string, CompiledJob>;
  outputs: Record<string, WorkflowOutput>;
  order: readonly string[];
  edges: readonly DependencyEdge[];
  #validated: ValidatedDefinition;
  #inputs: Record<string, JsonValue>;

  constructor(
    validated: ValidatedDefinition,
    inputs: Record<string, JsonValue>,
    jobs: Record<string, CompiledJob>,
    dag: { order: string[]; edges: DependencyEdge[] },
  ) {
    this.#validated = validated;
    this.#inputs = inputs;
    this.inputs = cloneJson(inputs) as Record<string, JsonValue>;
    this.jobs = cloneJson(jobs) as unknown as Record<string, CompiledJob>;
    this.outputs = cloneJson(validated.definition.outputs) as unknown as Record<
      string,
      WorkflowOutput
    >;
    this.order = [...dag.order];
    this.edges = dag.edges.map((edge) => ({ ...edge }));
  }

  materializeJobInput(jobId: string, results: ReadonlyMap<string, JsonValue>): JsonValue {
    const job = this.#validated.definition.jobs[jobId];
    const metadata = this.#validated.jobs.get(jobId);
    if (!metadata || !hasOwn(this.#validated.definition.jobs, jobId) || !job)
      throw new CheaperError("INVALID_REQUEST", `unknown compiled job: ${jobId}`);
    const input = materializeBindings(job.input, this.#inputs, results);
    validateJobInput(metadata, input);
    return input;
  }

  #results(states: ReadonlyMap<string, WorkflowJobState>): ReadonlyMap<string, JsonValue> {
    const results = new Map<string, JsonValue>();
    for (const id of Object.keys(this.#validated.definition.jobs)) {
      const state = states.get(id);
      if (!state || state.terminal !== true)
        throw new CheaperError("INVALID_OUTPUT", `job is not terminal: ${id}`);
      if (hasOwn(state, "result") && state.result !== undefined) results.set(id, state.result);
    }
    return results;
  }

  #output(name: string, results: ReadonlyMap<string, JsonValue>): JsonValue {
    const output = this.#validated.definition.outputs[name];
    const validator = this.#validated.outputs.get(name);
    if (!validator || !hasOwn(this.#validated.definition.outputs, name) || !output)
      throw new CheaperError("INVALID_OUTPUT", `unknown workflow output: ${name}`);
    const value = materializeBindings(output.value as JsonValue, this.#inputs, results, {
      code: "INVALID_OUTPUT",
    });
    const validation = validator.validate(value);
    if (!validation.ok)
      throw new CheaperError("INVALID_OUTPUT", `invalid workflow output: ${name}`, {
        details: { errors: validation.errors },
      });
    return value;
  }

  materializeOutput(name: string, states: ReadonlyMap<string, WorkflowJobState>): JsonValue {
    return this.#output(name, this.#results(states));
  }

  materializeOutputs(states: ReadonlyMap<string, WorkflowJobState>): Record<string, JsonValue> {
    const results = this.#results(states);
    return Object.fromEntries(
      Object.keys(this.#validated.definition.outputs)
        .sort()
        .map((name) => [name, this.#output(name, results)]),
    );
  }
}

export class WorkflowCompiler {
  constructor(private readonly ports: WorkflowCompilerPorts) {}

  validateDefinition(raw: unknown): WorkflowDefinition {
    return validateDefinition(raw, this.ports).definition;
  }

  projectInterface(raw: unknown): WorkflowInterface {
    return projectWorkflowInterface(raw, this.ports);
  }

  async compile(raw: unknown, inputs: unknown = {}): Promise<CompiledWorkflow> {
    const validated = validateDefinition(raw, this.ports);
    const snapshot = invocationInputs(validated, inputs);
    const jobs: Record<string, CompiledJob> = Object.fromEntries(
      Object.keys(validated.definition.jobs)
        .sort()
        .map((id) => {
          const original = validated.definition.jobs[id] as WorkflowJob;
          const input = materializeBindings(original.input, snapshot, new Map(), {
            deferFrom: true,
          });
          if (scanBindings(original.input).from.length === 0)
            validateJobInput(validated.jobs.get(id)!, input);
          return [
            id,
            {
              ...original,
              input,
              ...(original.needs ? { needs: [...new Set(original.needs)].sort() } : {}),
              ...(original.resources
                ? { resources: materializeResources(original.resources, snapshot) }
                : {}),
            },
          ];
        }),
    );
    let edges = validated.edges;
    if (Object.values(jobs).some((job) => job.resources && Object.keys(job.resources).length > 0)) {
      if (!this.ports.resources)
        throw new CheaperError(
          "INVALID_REQUEST",
          "resource inference port is required for explicit resources",
        );
      const inferenceJobs: Record<string, ResourceInferenceJob> = Object.fromEntries(
        Object.keys(jobs)
          .sort()
          .map((id) => {
            const worker = validated.jobs.get(id)?.worker;
            return [
              id,
              { job: cloneJson(jobs[id]) as unknown as WorkflowJob, ...(worker ? { worker } : {}) },
            ];
          }),
      );
      // port の失敗はそのまま伝播。未実装を空辺の成功に変換しない。
      const inferred = await this.ports.resources.infer(inferenceJobs);
      for (const id of Object.keys(jobs)) {
        if (!hasOwn(inferred.manifests, id) || !inferred.manifests[id])
          throw new CheaperError("INVALID_REQUEST", `missing resource manifest: ${id}`);
        jobs[id]!.manifest = cloneJson(inferred.manifests[id]) as unknown as ResourceManifest;
      }
      edges = [...edges, ...inferred.edges];
    }
    const dag = compileDag(Object.keys(jobs), edges);
    return new Compiled(validated, snapshot, jobs, dag);
  }
}
