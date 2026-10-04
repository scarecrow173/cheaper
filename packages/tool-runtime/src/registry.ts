import {
  CheaperError,
  compileSchema,
  type PathDenylist,
  type SymlinkPolicy,
  type ToolName,
} from "@cheaper/core";
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
import { createReadTool } from "./read.js";

export type ToolHandler = (input: unknown, signal: AbortSignal) => Promise<CanonicalToolResult>;
export type InjectedToolHandlers = Partial<Record<"write" | "edit" | "ls", ToolHandler>>;
export interface ToolRuntimeOptions {
  root: string;
  symlinkPolicy: SymlinkPolicy;
  names: readonly ToolName[];
  maxToolCalls: number;
  onLimit: () => void;
  journal?: MutationJournal;
  handlers?: InjectedToolHandlers;
  denylist?: PathDenylist | undefined;
  writeScope?: ReadonlySet<string> | undefined;
  signal?: AbortSignal | undefined;
  abortGraceMs?: number | undefined;
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
  execute(name: string, input: unknown, context?: ToolCallContext): Promise<CanonicalToolResult>;
  operations(): readonly OperationRecord[];
  abort(reason?: unknown): void;
  drain(): Promise<void>;
  dispose(): Promise<void>;
}

interface OperationBinding {
  manager: OperationManager;
  ownerId: string;
}

export function createToolRuntime(
  options: ToolRuntimeOptions,
  binding?: OperationBinding,
): ToolRuntime {
  const opts = { ...options, names: [...options.names], handlers: { ...options.handlers } };
  if (
    opts.abortGraceMs !== undefined &&
    (!Number.isFinite(opts.abortGraceMs) || opts.abortGraceMs < 0)
  ) {
    throw new CheaperError("INVALID_REQUEST", "abort grace must be a finite nonnegative number");
  }
  const catalog = createToolCatalog();
  const journal = opts.journal ?? new MutationJournal();
  const accounting = createToolBudget(opts);
  const metering = createToolMeter();
  const controller = new AbortController();
  const manager =
    binding?.manager ?? new OperationManager({ abortGraceMs: opts.abortGraceMs ?? 5000 });
  const ownerId = binding?.ownerId ?? "tool-runtime";
  const abort = (reason?: unknown) => controller.abort(reason);
  const onScopeAbort = () => manager.cancelOwner(ownerId, controller.signal.reason);
  const onExternalAbort = () => abort(opts.signal?.reason);
  const definitions: CanonicalToolDefinition[] = opts.names.map((name) => {
    switch (name) {
      case "read":
        return createReadTool(opts);
      case "grep":
        return createGrepTool(opts);
      case "find":
        return createFindTool(opts);
      case "count_tokens":
        return createCountTokensTool(opts);
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
    definitions,
    async execute(name, input, context) {
      if (controller.signal.aborted) throw new Error("Operation aborted");
      const entry = registered.get(name as ToolName);
      if (
        !entry ||
        (["write", "edit", "ls"].includes(name) &&
          !opts.handlers?.[name as keyof InjectedToolHandlers])
      )
        throw unknownTool(name);
      validateToolInput(name, entry.schema, input);
      const signal = context?.signal
        ? AbortSignal.any([controller.signal, context.signal])
        : controller.signal;
      if (signal.aborted) throw new Error("Operation aborted");
      return accounting.run(() =>
        manager.run({ ownerId, kind: "tool", toolName: name, signal }, (operationSignal) =>
          metering.run(() => entry.definition.execute(input, operationSignal)),
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
