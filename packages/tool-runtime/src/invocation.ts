import {
  CheaperError,
  type ConcurrencyPolicy,
  type ProcessPolicy,
  type ResourceManifestInput,
  resolveProcessPolicy,
  type SymlinkPolicy,
  type ToolName,
} from "@cheaper/core";
import { ResourceAccess, type ResourceScopeOptions } from "./access.js";
import type { ArtifactPorts } from "./artifact.js";
import type { ExecutionMetrics } from "./execution-metrics.js";
import { OperationManager, type OperationRecord } from "./operation.js";
import { createToolRuntime, type ToolRuntime, type ToolRuntimeOptions } from "./registry.js";
import { type ResourceLease, ResourceManager } from "./resources.js";
import { ToolScheduler } from "./scheduler.js";

export type { ResourceLease } from "./resources.js";
export interface JobLeaseOptions extends ResourceScopeOptions {
  ownerId: string;
  names: readonly ToolName[];
  signal?: AbortSignal;
}
export interface WorkerExecutionBinding {
  runtime: InvocationToolRuntime;
  ownerId: string;
  parentLease?: ResourceLease;
  /** parent safety と literal writeScope の両方を満たす canonical child manifest。 */
  resourceManifest?: ResourceManifestInput;
}
export interface WorkerScopeOptions extends ToolRuntimeOptions {
  ownerId: string;
  parentLease?: ResourceLease;
}
export interface InvocationToolRuntimeOptions {
  artifacts?: ArtifactPorts | undefined;
  process?: Partial<ProcessPolicy> | undefined;
  root: string;
  symlinkPolicy: SymlinkPolicy;
  signal?: AbortSignal;
  abortGraceMs?: number;
  concurrency?: Partial<ConcurrencyPolicy> | undefined;
  /** operation timestamp と snapshot に共通の monotonic clock。 */
  clock?: () => number;
}
export interface InvocationToolRuntime {
  readonly root: string;
  readonly symlinkPolicy: SymlinkPolicy;
  acquireJobLease(options: JobLeaseOptions): Promise<ResourceLease>;
  releaseJobLease(lease: ResourceLease): void;
  createWorkerScope(options: WorkerScopeOptions): ToolRuntime;
  operations(ownerId?: string): readonly OperationRecord[];
  executionMetrics(ownerId?: string): ExecutionMetrics;
  abort(reason?: unknown): void;
  drain(): Promise<void>;
  dispose(): Promise<void>;
}

export function createInvocationToolRuntime(
  options: InvocationToolRuntimeOptions,
): InvocationToolRuntime {
  const opts = { ...options, process: resolveProcessPolicy(options.process) };
  if (
    opts.abortGraceMs !== undefined &&
    (!Number.isFinite(opts.abortGraceMs) || opts.abortGraceMs < 0)
  ) {
    throw invalidBinding("abort grace must be a finite nonnegative number");
  }
  const controller = new AbortController();
  const manager = new OperationManager({
    signal: controller.signal,
    abortGraceMs: opts.abortGraceMs ?? 5000,
    ...(opts.clock !== undefined ? { clock: opts.clock } : {}),
  });
  const resources = new ResourceManager(opts.root, opts.symlinkPolicy);
  const scheduler = new ToolScheduler(manager, opts.concurrency, resources);
  const abort = (reason?: unknown) => controller.abort(reason);
  const onExternalAbort = () => abort(opts.signal?.reason);
  opts.signal?.addEventListener("abort", onExternalAbort, { once: true });
  if (opts.signal?.aborted) onExternalAbort();
  const scopes = new Set<ToolRuntime>();
  const owners = new Set<string>();
  let disposed: Promise<void> | undefined;
  const runtime: InvocationToolRuntime = {
    root: opts.root,
    symlinkPolicy: opts.symlinkPolicy,
    acquireJobLease(options) {
      if (!options.ownerId) throw invalidBinding("empty lease owner");
      const access = new ResourceAccess({
        ...options,
        root: resources.root,
        symlinkPolicy: opts.symlinkPolicy,
      });
      return resources.request({
        ownerId: options.ownerId,
        kind: "job",
        signal: options.signal
          ? AbortSignal.any([controller.signal, options.signal])
          : controller.signal,
        prepare: () => access.prepare(),
        start: (lease) => lease,
      });
    },
    releaseJobLease: (lease) => resources.release(lease),
    createWorkerScope(options) {
      if (controller.signal.aborted) throw new Error("Invocation aborted");
      if (options.root !== opts.root) throw invalidBinding("root mismatch");
      if (options.symlinkPolicy !== opts.symlinkPolicy)
        throw invalidBinding("symlink policy mismatch");
      if (
        options.process !== undefined &&
        JSON.stringify(resolveProcessPolicy(options.process)) !== JSON.stringify(opts.process)
      )
        throw invalidBinding("process policy mismatch");
      if (owners.has(options.ownerId) || !options.ownerId)
        throw invalidBinding("duplicate or empty owner");
      if (options.parentLease !== undefined) resources.parent(options.parentLease, options.ownerId);
      const scope = createToolRuntime(
        {
          ...options,
          artifacts: opts.artifacts,
          process: opts.process,
          signal: options.signal
            ? AbortSignal.any([controller.signal, options.signal])
            : controller.signal,
          abortGraceMs: options.abortGraceMs ?? opts.abortGraceMs,
        },
        { manager, scheduler, resources, ownerId: options.ownerId },
      );
      owners.add(options.ownerId);
      scopes.add(scope);
      const dispose = scope.dispose.bind(scope);
      scope.dispose = async () => {
        try {
          await dispose();
        } finally {
          scopes.delete(scope);
        }
      };
      return scope;
    },
    operations: (ownerId) => manager.records(ownerId),
    executionMetrics: (ownerId) => manager.executionMetrics(ownerId),
    abort,
    async drain() {
      abort();
      await Promise.all([...scopes].map((scope) => scope.drain()));
    },
    dispose() {
      disposed ??= (async () => {
        abort();
        try {
          await Promise.all([...scopes].map((scope) => scope.dispose()));
        } finally {
          await manager.dispose();
          opts.signal?.removeEventListener("abort", onExternalAbort);
          scopes.clear();
          // acquireJobLease の caller が所有する parent は late executor finally まで保持する。
        }
      })();
      return disposed;
    },
  };
  return runtime;
}

function invalidBinding(reason: string): CheaperError {
  return new CheaperError("INVALID_REQUEST", `invalid execution binding: ${reason}`, {
    details: { reason },
  });
}
