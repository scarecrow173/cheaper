import {
  CheaperError,
  type ConcurrencyPolicy,
  type ProcessPolicy,
  resolveProcessPolicy,
  type SymlinkPolicy,
} from "@cheaper/core";
import { OperationManager, type OperationRecord } from "./operation.js";
import { createToolRuntime, type ToolRuntime, type ToolRuntimeOptions } from "./registry.js";
import { ToolScheduler } from "./scheduler.js";

/** 後続 #61 で具体化する。値を作れる no-op service は公開しない。 */
export type ResourceManager = never;
export type ResourceLease = never;
export interface WorkerExecutionBinding {
  runtime: InvocationToolRuntime;
  ownerId: string;
  parentLease?: ResourceLease;
}
export interface WorkerScopeOptions extends ToolRuntimeOptions {
  ownerId: string;
  parentLease?: ResourceLease;
}
export interface InvocationToolRuntimeOptions {
  process?: Partial<ProcessPolicy> | undefined;
  root: string;
  symlinkPolicy: SymlinkPolicy;
  signal?: AbortSignal;
  abortGraceMs?: number;
  concurrency?: Partial<ConcurrencyPolicy> | undefined;
}
export interface InvocationToolRuntime {
  readonly root: string;
  readonly symlinkPolicy: SymlinkPolicy;
  createWorkerScope(options: WorkerScopeOptions): ToolRuntime;
  operations(ownerId?: string): readonly OperationRecord[];
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
  });
  const scheduler = new ToolScheduler(manager, opts.concurrency);
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
      if (options.parentLease !== undefined)
        throw invalidBinding("parent lease is not implemented (#61)");
      const scope = createToolRuntime(
        {
          ...options,
          process: opts.process,
          signal: options.signal
            ? AbortSignal.any([controller.signal, options.signal])
            : controller.signal,
          abortGraceMs: options.abortGraceMs ?? opts.abortGraceMs,
        },
        { manager, scheduler, ownerId: options.ownerId },
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
