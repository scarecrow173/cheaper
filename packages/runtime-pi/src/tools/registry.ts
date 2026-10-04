import type { ConcurrencyPolicy, PathDenylist, SymlinkPolicy, ToolName } from "@cheaper/core";
import {
  type AccessObserver,
  type CanonicalToolResult,
  createToolRuntime,
  type InjectedToolHandlers,
  isAccessDeniedError,
  type JsonValue,
  type MutationJournal,
  ResourceAccess,
  type ResourceScopeOptions,
  type ToolBudget,
  type ToolMeter,
  type ToolRuntime,
} from "@cheaper/tool-runtime";
import {
  createEditToolDefinition,
  createLsToolDefinition,
  createWriteToolDefinition,
  defineTool,
  type ToolDefinition,
} from "@earendil-works/pi-coding-agent";
import { toPiTool } from "./adapter.js";
import { createEditOperations, createLsOperations, createWriteOperations } from "./operations.js";

export interface SandboxedToolsOptions extends ResourceScopeOptions {
  root: string;
  symlinkPolicy: SymlinkPolicy;
  journal: MutationJournal;
  names: ToolName[];
  maxToolCalls: number;
  onLimit: () => void;
  denylist?: PathDenylist | undefined;
  observeAccess?: AccessObserver | undefined;
  runtime?: ToolRuntime;
  concurrency?: Partial<ConcurrencyPolicy>;
}
export interface SandboxedTools {
  tools: ToolDefinition[];
  toolNames: string[];
  budget: ToolBudget;
  meter: ToolMeter;
  runtime: ToolRuntime;
}
function helperDefinitions(
  opts: SandboxedToolsOptions,
): Record<"write" | "edit" | "ls", ToolDefinition> {
  return {
    write: defineTool(
      createWriteToolDefinition(opts.root, { operations: createWriteOperations(opts) }),
    ),
    edit: defineTool(
      createEditToolDefinition(opts.root, { operations: createEditOperations(opts) }),
    ),
    ls: defineTool(createLsToolDefinition(opts.root, { operations: createLsOperations(opts) })),
  };
}
/** Pi helper の mutation queue と boundary operations を port に注入する。 */
export function createPiToolHandlers(options: SandboxedToolsOptions): InjectedToolHandlers {
  const opts = {
    ...options,
    writeScope: options.writeScope === undefined ? undefined : new Set(options.writeScope),
    resourceManifest:
      options.resourceManifest === undefined
        ? undefined
        : structuredClone(options.resourceManifest),
  };
  const preflight = opts.observeAccess ? undefined : new ResourceAccess(opts);
  const definitions = helperDefinitions(opts);
  let nextId = 0;
  return Object.fromEntries(
    Object.entries(definitions).map(([name, definition]) => [
      name,
      async (input: unknown, signal: AbortSignal): Promise<CanonicalToolResult> => {
        // raw injected helper でも target を mkdir より先に共通 controller で認可する。
        if (name === "write" || name === "edit")
          await preflight?.operation(name, input as JsonValue);
        let denied: unknown;
        let executable = definition;
        if (name === "edit") {
          // Pi は access error を別 Error に包む。call ごとの port で元の拒否を保存し、
          // canonical accounting が boundary 拒否を一回認識できるようにする。
          const operations = createEditOperations(opts);
          executable = defineTool(
            createEditToolDefinition(opts.root, {
              operations: {
                ...operations,
                async access(file) {
                  try {
                    await operations.access(file);
                  } catch (e) {
                    if (isAccessDeniedError(e)) denied = e;
                    throw e;
                  }
                },
              },
            }),
          );
        }
        try {
          return (await executable.execute(
            `canonical-${++nextId}`,
            input as never,
            signal,
            undefined,
            undefined as never,
          )) as CanonicalToolResult;
        } catch (e) {
          throw denied ?? e;
        }
      },
    ]),
  );
}
export function createSandboxedTools(opts: SandboxedToolsOptions): SandboxedTools {
  let runtime = opts.runtime;
  const observed = {
    ...opts,
    observeAccess: (...args: Parameters<AccessObserver>) => {
      if (!runtime) throw new Error("missing tool runtime");
      return runtime.observeAccess(...args);
    },
  };
  runtime ??= createToolRuntime({ ...opts, handlers: createPiToolHandlers(observed) });
  const helpers = helperDefinitions(opts);
  const tools: ToolDefinition[] = runtime.definitions.map((definition) => {
    const execute = (input: unknown, signal?: AbortSignal) =>
      runtime.execute(definition.name, input, { signal });
    if (definition.name === "write" || definition.name === "edit" || definition.name === "ls") {
      return {
        ...helpers[definition.name],
        ...toPiTool(definition, execute),
        description: helpers[definition.name].description,
      };
    }
    return toPiTool(definition, execute);
  });
  return {
    tools,
    toolNames: tools.map((t) => t.name),
    budget: runtime.budget,
    meter: runtime.meter,
    runtime,
  };
}
