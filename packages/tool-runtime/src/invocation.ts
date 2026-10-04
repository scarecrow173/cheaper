import { CheaperError, type SymlinkPolicy } from "@cheaper/core";
import { createToolRuntime, type ToolRuntime, type ToolRuntimeOptions } from "./registry.js";

/** 後続 #56/#57/#61 で具体化する。値を作れる no-op service は公開しない。 */
export type ToolScheduler = never;
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
  root: string;
  symlinkPolicy: SymlinkPolicy;
  signal?: AbortSignal;
  abortGraceMs?: number;
}
export interface InvocationToolRuntime {
  readonly root: string;
  readonly symlinkPolicy: SymlinkPolicy;
  createWorkerScope(options: WorkerScopeOptions): ToolRuntime;
  abort(): void;
  drain(): Promise<void>;
  dispose(): Promise<void>;
}

export function createInvocationToolRuntime(
  options: InvocationToolRuntimeOptions,
): InvocationToolRuntime {
  const opts = { ...options };
  if (
    opts.abortGraceMs !== undefined &&
    (!Number.isFinite(opts.abortGraceMs) || opts.abortGraceMs < 0)
  ) {
    throw invalidBinding("abort grace must be a finite nonnegative number");
  }
  const controller = new AbortController();
  const abort = () => controller.abort();
  opts.signal?.addEventListener("abort", abort, { once: true });
  if (opts.signal?.aborted) abort();
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
      if (owners.has(options.ownerId) || !options.ownerId)
        throw invalidBinding("duplicate or empty owner");
      if (options.parentLease !== undefined)
        throw invalidBinding("parent lease is not implemented (#61)");
      const scope = createToolRuntime({
        ...options,
        signal: options.signal
          ? AbortSignal.any([controller.signal, options.signal])
          : controller.signal,
        abortGraceMs: options.abortGraceMs ?? opts.abortGraceMs,
      });
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
          opts.signal?.removeEventListener("abort", abort);
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
