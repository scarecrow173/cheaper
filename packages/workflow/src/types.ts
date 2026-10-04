import type { ResourceManifest, WorkerDefinition } from "@cheaper/core";
import type { JsonValue, ToolCatalog } from "@cheaper/tool-runtime";

export type { JsonValue } from "@cheaper/tool-runtime";
export type JsonSchema = Record<string, unknown>;
export type InputBinding = { $input: string };
export type FromBinding = { $from: string; path: string };
export type Binding = InputBinding | FromBinding | { $literal: JsonValue };
export interface ScopeDeclaration {
  path: string | InputBinding;
  scope: "exact" | "subtree";
}
export interface WorkflowResources {
  reads?: ScopeDeclaration[];
  writes?: ScopeDeclaration[];
  produces?: ScopeDeclaration[];
}
interface JobFields {
  input: JsonValue;
  needs?: string[];
  resources?: WorkflowResources;
}
export type WorkflowJob = JobFields &
  ({ kind: "tool"; tool: string } | { kind: "worker"; worker: string });
export interface WorkflowOutput {
  schema: JsonSchema;
  value: Binding;
}
export interface WorkflowDefinition {
  schemaVersion: 1;
  inputs: Record<string, JsonSchema>;
  jobs: Record<string, WorkflowJob>;
  outputs: Record<string, WorkflowOutput>;
}
export interface DependencyEdge {
  from: string;
  to: string;
}
export interface WorkerCatalog {
  /** 既存 loader が検証した metadata。Worker を実行しない。 */
  get(name: string): WorkerDefinition | undefined;
}
export interface ResourceInferenceJob {
  job: WorkflowJob;
  worker?: WorkerDefinition;
}
export interface ResourceInferenceResult {
  manifests: Readonly<Record<string, ResourceManifest>>;
  edges: readonly DependencyEdge[];
}
export interface ResourceInference {
  /** D4 の正規化・fallback・writeScope 検査はこの port が所有する。 */
  infer(jobs: Readonly<Record<string, ResourceInferenceJob>>):
    | ResourceInferenceResult
    | Promise<ResourceInferenceResult>;
}
export interface WorkflowCompilerPorts {
  tools: ToolCatalog;
  workers: WorkerCatalog;
  resources?: ResourceInference;
}
export type CompiledJob = WorkflowJob & { manifest?: ResourceManifest };
export interface WorkflowJobState {
  terminal: boolean;
  result?: JsonValue;
}
export interface CompiledWorkflow {
  schemaVersion: 1;
  inputs: Record<string, JsonValue>;
  jobs: Record<string, CompiledJob>;
  /** 順序の正本。Record の整数風 key の enumeration 順は契約外。 */
  order: readonly string[];
  edges: readonly DependencyEdge[];
  outputs: Record<string, WorkflowOutput>;
  materializeJobInput(jobId: string, results: ReadonlyMap<string, JsonValue>): JsonValue;
  materializeOutput(name: string, states: ReadonlyMap<string, WorkflowJobState>): JsonValue;
  materializeOutputs(states: ReadonlyMap<string, WorkflowJobState>): Record<string, JsonValue>;
}
export interface WorkflowInterface {
  schemaVersion: 1;
  inputs: Record<string, { schema: JsonSchema; required: boolean }>;
  outputs: Record<string, { schema: JsonSchema }>;
}
