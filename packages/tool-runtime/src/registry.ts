import { AsyncLocalStorage } from "node:async_hooks";
import {
  CheaperError,
  type ConcurrencyPolicy,
  claimContains,
  compileSchema,
  isMutatingTool,
  type PathDenylist,
  type ProcessPolicy,
  type ResourceClaim,
  type ResourceScope,
  resolveProcessPolicy,
  type SymlinkPolicy,
  type ToolName,
} from "@cheaper/core";
import {
  type AccessSnapshot,
  type ActiveResourceAccess,
  ResourceAccess,
  type ResourceScopeOptions,
} from "./access.js";
import {
  createToolBudget,
  createToolMeter,
  type ToolBudget,
  type ToolMeter,
} from "./accounting.js";
import { type CanonicalToolResult, createToolCatalog, type ToolCatalog } from "./catalog.js";
import { createCountTokensTool } from "./count-tokens.js";
import { type CanonicalToolDefinition, validateToolInput } from "./definition.js";
import { createFindTool } from "./find.js";
import { createGrepTool } from "./grep.js";
import { MutationJournal } from "./journal.js";
import { OperationManager, type OperationRecord } from "./operation.js";
import { createProcessTool, processOperationDeclaration } from "./process.js";
import { createReadTool } from "./read.js";
import { invalidResource, type ResourceLease, ResourceManager } from "./resources.js";
import { ToolScheduler } from "./scheduler.js";

export type ToolHandler = (input: unknown, signal: AbortSignal) => Promise<CanonicalToolResult>;
export type InjectedToolHandlers = Partial<Record<"write" | "edit" | "ls", ToolHandler>>;
export interface ToolRuntimeOptions extends ResourceScopeOptions {
  process?: Partial<ProcessPolicy> | undefined;
  root: string;
  symlinkPolicy: SymlinkPolicy;
  names: readonly ToolName[];
  maxToolCalls: number;
  onLimit: () => void;
  journal?: MutationJournal;
  handlers?: InjectedToolHandlers;
  denylist?: PathDenylist | undefined;
  parentLease?: ResourceLease | undefined;
  signal?: AbortSignal | undefined;
  abortGraceMs?: number | undefined;
  concurrency?: Partial<ConcurrencyPolicy> | undefined;
}
export interface ToolCallContext {
  signal?: AbortSignal | undefined;
}
export interface ToolRuntime {
  readonly catalog: ToolCatalog;
  readonly journal: MutationJournal;
  readonly budget: ToolBudget;
  readonly meter: ToolMeter;
  readonly definitions: readonly CanonicalToolDefinition[];
  ready(): Promise<ResourceLease>;
  observeAccess(
    path: string,
    mode: ResourceClaim["mode"],
    scope?: ResourceScope,
    support?: boolean,
  ): Promise<void>;
  accesses(): AccessSnapshot;
  execute(name: string, input: unknown, context?: ToolCallContext): Promise<CanonicalToolResult>;
  operations(): readonly OperationRecord[];
  abort(reason?: unknown): void;
  drain(): Promise<void>;
  dispose(): Promise<void>;
}

interface OperationBinding {
  manager: OperationManager;
  scheduler: ToolScheduler;
  ownerId: string;
  resources: ResourceManager;
}

export function createToolRuntime(
  options: ToolRuntimeOptions,
  binding?: OperationBinding,
): ToolRuntime {
  const opts = {
    ...options,
    process: resolveProcessPolicy(options.process),
    names: [...options.names],
    handlers: { ...options.handlers },
  };
  if (opts.names.includes("process") && Object.keys(opts.process.tasks).length === 0) {
    throw new CheaperError(
      "POLICY_VIOLATION",
      "process exposure requires nonempty authorized tasks",
    );
  }
  if (
    opts.abortGraceMs !== undefined &&
    (!Number.isFinite(opts.abortGraceMs) || opts.abortGraceMs < 0)
  ) {
    throw new CheaperError("INVALID_REQUEST", "abort grace must be a finite nonnegative number");
  }
  const catalog = createToolCatalog({ process: opts.process });
  const journal = opts.journal ?? new MutationJournal();
  const accounting = createToolBudget(opts);
  const metering = createToolMeter();
  const controller = new AbortController();
  const manager =
    binding?.manager ?? new OperationManager({ abortGraceMs: opts.abortGraceMs ?? 5000 });
  const ownerId = binding?.ownerId ?? "tool-runtime";
  const resources = binding?.resources ?? new ResourceManager(opts.root, opts.symlinkPolicy);
  const scheduler = binding?.scheduler ?? new ToolScheduler(manager, opts.concurrency, resources);
  const inherited = opts.parentLease
    ? resources.parent(opts.parentLease, ownerId).readStrictness
    : undefined;
  const access = new ResourceAccess({ ...opts, root: resources.root }, inherited);
  const active = new AsyncLocalStorage<ActiveResourceAccess>();
  let parentLease = opts.parentLease;
  let readyPromise: Promise<ResourceLease> | undefined;
  const observeAccess = async (
    p: string,
    mode: ResourceClaim["mode"],
    scope?: ResourceScope,
    support?: boolean,
  ) => {
    const operation = active.getStore();
    if (!operation) throw invalidResource("observation requires active operation");
    if (manager.get(operation.id).state !== "running")
      throw invalidResource("operation has settled");
    await access.observe(p, mode, scope, operation, support);
    if (manager.get(operation.id).state !== "running")
      throw invalidResource("operation has settled");
  };
  const abort = (reason?: unknown) => controller.abort(reason);
  const onScopeAbort = () => manager.cancelOwner(ownerId, controller.signal.reason);
  const onExternalAbort = () => abort(opts.signal?.reason);
  const definitions: CanonicalToolDefinition[] = opts.names.map((name) => {
    switch (name) {
      case "read":
        return createReadTool({ ...opts, observeAccess });
      case "grep":
        return createGrepTool({ ...opts, observeAccess });
      case "find":
        return createFindTool({ ...opts, observeAccess });
      case "count_tokens":
        return createCountTokensTool({ ...opts, observeAccess });
      case "process":
        return createProcessTool(opts);
      default: {
        const metadata = catalog.get(name);
        if (!metadata) throw unknownTool(name);
        return {
          name,
          label: name,
          description: `${name} workspace tool`,
          parameters: metadata.inputSchema,
          execute: async (input, signal) => {
            const handler = opts.handlers?.[name];
            if (!handler) throw unknownTool(name);
            return handler(input, signal ?? controller.signal);
          },
        };
      }
    }
  });
  const registered = new Map(
    definitions.map((definition) => [
      definition.name,
      { definition, schema: compileSchema({ ...definition.parameters }, "input") },
    ]),
  );
  let disposed: Promise<void> | undefined;
  controller.signal.addEventListener("abort", onScopeAbort, { once: true });
  opts.signal?.addEventListener("abort", onExternalAbort, { once: true });
  if (opts.signal?.aborted) onExternalAbort();
  const runtime: ToolRuntime = {
    catalog,
    journal,
    budget: accounting.budget,
    meter: metering.meter,
    ready() {
      readyPromise ??= (async () => {
        if (controller.signal.aborted) throw controller.signal.reason;
        if (opts.parentLease) {
          const prepared = await access.prepare();
          const parent = resources.parent(opts.parentLease, ownerId);
          if (!prepared.claims.every((claim) => parent.claims.some((p) => claimContains(p, claim))))
            throw invalidResource("scope claims are outside parent");
          return opts.parentLease;
        }
        return resources.request({
          ownerId,
          kind: "job",
          signal: controller.signal,
          prepare: () => access.prepare(),
          start: (lease) => {
            parentLease = lease;
            return lease;
          },
        });
      })();
      return readyPromise;
    },
    observeAccess,
    accesses: () => access.snapshot(),
    definitions: definitions.map((definition) => ({
      ...definition,
      execute: (input, signal) => runtime.execute(definition.name, input, { signal }),
    })),
    async execute(name, input, context) {
      if (controller.signal.aborted) throw controller.signal.reason;
      const entry = registered.get(name as ToolName);
      if (
        !entry ||
        (["write", "edit", "ls"].includes(name) &&
          !opts.handlers?.[name as keyof InjectedToolHandlers])
      )
        throw unknownTool(name);
      validateToolInput(name, entry.schema, input);
      // admission と実行は同じ submitted input snapshot を使う。
      const submittedInput = structuredClone(input);
      const readiness = readyPromise;
      const scoped = access.explicit || parentLease !== undefined || readiness !== undefined;
      const signal = context?.signal
        ? AbortSignal.any([controller.signal, context.signal])
        : controller.signal;
      if (signal.aborted) throw signal.reason;
      const declaration =
        name === "process"
          ? processOperationDeclaration(opts.process, submittedInput as { task: string })
          : {
              access: isMutatingTool(entry.definition.name)
                ? ("exclusive" as const)
                : ("shared" as const),
            };
      return accounting.run(() =>
        scheduler.run(
          { ownerId, kind: "tool", toolName: name, signal },
          {
            ...declaration,
            resourceScoped: scoped,
            parentLease: () => parentLease,
            readiness,
            ...(scoped
              ? {
                  prepare: async () => {
                    return access.operation(
                      entry.definition.name,
                      submittedInput as import("./catalog.js").JsonValue,
                    );
                  },
                }
              : {}),
          },
          (operationSignal) => {
            const operation = manager
              .records(ownerId)
              .find((record) => record.state === "running" && record.signal === operationSignal);
            if (!operation) throw invalidResource("missing running operation");
            return active.run({ id: operation.id, claims: operation.resourceClaims ?? [] }, () =>
              metering.run(() => entry.definition.execute(submittedInput, operationSignal)),
            );
          },
        ),
      );
    },
    operations: () => manager.records(ownerId),
    abort,
    async drain() {
      abort();
      await manager.drain(ownerId, opts.abortGraceMs ?? 5000);
    },
    dispose() {
      disposed ??= (async () => {
        await runtime.drain();
        if (!opts.parentLease && parentLease) resources.release(parentLease);
        opts.signal?.removeEventListener("abort", onExternalAbort);
        controller.signal.removeEventListener("abort", onScopeAbort);
        if (!binding) await manager.dispose();
      })();
      return disposed;
    },
  };
  return runtime;
}

function unknownTool(name: string): CheaperError {
  return new CheaperError("UNKNOWN_TOOL", `unknown or unregistered tool: ${name}`, {
    details: { tool: name },
  });
}
