import Ajv2020Ns, { type ValidateFunction } from "ajv/dist/2020.js";
import { DURATION_PATTERN, parseDuration } from "../duration.js";
import { CheaperError } from "../errors.js";
import { TIERS, type Tier } from "../model/tier.js";
import { TOOL_ALIASES, TOOL_NAMES, type ToolName } from "../tools/vocabulary.js";
import { PROCESS_POLICY_PROPERTIES, type ProcessPolicy, resolveProcessPolicy } from "./process.js";

export interface ConcurrencyPolicy {
  maxOperations: number;
  mutations: "serialized" | "per-resource";
}

const DEFAULT_CONCURRENCY: Readonly<ConcurrencyPolicy> = Object.freeze({
  maxOperations: 8,
  mutations: "serialized",
});

/** legacy policy と runtime options の既定・値検証を一箇所で所有する。 */
export function resolveConcurrencyPolicy(input?: Partial<ConcurrencyPolicy>): ConcurrencyPolicy {
  const maxOperations = input?.maxOperations ?? DEFAULT_CONCURRENCY.maxOperations;
  const mutations = input?.mutations ?? DEFAULT_CONCURRENCY.mutations;
  if (
    !Number.isSafeInteger(maxOperations) ||
    maxOperations < 1 ||
    (mutations !== "serialized" && mutations !== "per-resource")
  ) {
    throw new CheaperError("INVALID_RUNTIME_POLICY", "invalid execution.concurrency");
  }
  return { maxOperations, mutations };
}

export interface RuntimePolicy {
  /** legacy typed policy の互換。normalize/load は常に補完する。 */
  process?: ProcessPolicy;
  execution: {
    timeoutMs: number;
    maxTurns: number;
    maxToolCalls: number;
    /** legacy typed policy の互換。normalizePolicy は常に補完する。 */
    concurrency?: ConcurrencyPolicy;
  };
  tools: { allow: ToolName[] };
  filesystem: { symlinks: "deny" | "within-workspace"; denyPaths: string[] };
  model: { maxTier: Tier; tierFallback: "none" | "higher" };
  output: { maxTokens: number; repairAttempts: number };
  retry: { transientAttempts: number };
}

const Ajv2020 = Ajv2020Ns.default;

function objectSchema(required: string[], properties: Record<string, unknown>) {
  return { type: "object", additionalProperties: false, required, properties };
}

const CONCURRENCY_PROPERTIES = {
  maxOperations: { type: "integer", minimum: 1, maximum: Number.MAX_SAFE_INTEGER },
  mutations: { type: "string", enum: ["serialized", "per-resource"] },
};
const EXECUTION_PROPERTIES = {
  timeout: { type: "string", pattern: DURATION_PATTERN.source },
  maxTurns: { type: "integer", minimum: 1 },
  maxToolCalls: { type: "integer", minimum: 1 },
  concurrency: objectSchema(["maxOperations", "mutations"], CONCURRENCY_PROPERTIES),
};

const TOOLS_PROPERTIES = {
  allow: { type: "array", items: { type: "string" } },
};

const FILESYSTEM_PROPERTIES = {
  symlinks: { type: "string", enum: ["deny", "within-workspace"] },
  denyPaths: { type: "array", items: { type: "string" } },
};

const MODEL_PROPERTIES = {
  maxTier: { type: "string", enum: [...TIERS] },
  tierFallback: { type: "string", enum: ["none", "higher"] },
};

const OUTPUT_PROPERTIES = {
  maxTokens: { type: "integer", minimum: 1 },
  repairAttempts: { type: "integer", minimum: 0 },
};

const RETRY_PROPERTIES = {
  transientAttempts: { type: "integer", minimum: 0 },
};

const VERSION_PROPERTY = { type: "integer", const: 1 };

export const POLICY_COMPLETE_SCHEMA = objectSchema(
  ["version", "execution", "tools", "filesystem", "model", "output", "retry"],
  {
    version: VERSION_PROPERTY,
    process: objectSchema(["maxConcurrent", "tasks"], PROCESS_POLICY_PROPERTIES),
    execution: objectSchema(["timeout", "maxTurns", "maxToolCalls"], EXECUTION_PROPERTIES),
    tools: objectSchema(["allow"], TOOLS_PROPERTIES),
    filesystem: objectSchema(["symlinks"], FILESYSTEM_PROPERTIES),
    model: objectSchema(["maxTier", "tierFallback"], MODEL_PROPERTIES),
    output: objectSchema(["maxTokens", "repairAttempts"], OUTPUT_PROPERTIES),
    retry: objectSchema(["transientAttempts"], RETRY_PROPERTIES),
  },
);

export const POLICY_PARTIAL_SCHEMA = objectSchema(["version"], {
  version: VERSION_PROPERTY,
  process: objectSchema([], PROCESS_POLICY_PROPERTIES),
  execution: objectSchema([], {
    ...EXECUTION_PROPERTIES,
    concurrency: objectSchema([], CONCURRENCY_PROPERTIES),
  }),
  tools: objectSchema([], TOOLS_PROPERTIES),
  filesystem: objectSchema([], FILESYSTEM_PROPERTIES),
  model: objectSchema([], MODEL_PROPERTIES),
  output: objectSchema([], OUTPUT_PROPERTIES),
  retry: objectSchema([], RETRY_PROPERTIES),
});

const ajv = new Ajv2020({ strict: true, allErrors: true });
const COMPLETE_VALIDATE: ValidateFunction = ajv.compile(POLICY_COMPLETE_SCHEMA);
const PARTIAL_VALIDATE: ValidateFunction = ajv.compile(POLICY_PARTIAL_SCHEMA);

function isPlainObject(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

export function validatePolicySchema(
  raw: unknown,
  mode: "complete" | "partial",
): Record<string, unknown> {
  const validate = mode === "complete" ? COMPLETE_VALIDATE : PARTIAL_VALIDATE;
  if (!validate(raw)) {
    throw new CheaperError("INVALID_RUNTIME_POLICY", `invalid ${mode} runtime policy`, {
      details: {
        errors: (validate.errors ?? []).map((e) => ({
          instancePath: e.instancePath,
          message: e.message ?? "validation error",
        })),
      },
    });
  }
  return raw as Record<string, unknown>;
}

function normalizePolicyTools(raw: unknown): ToolName[] {
  const list = Array.isArray(raw) ? raw : [];
  const names = list.map((v) => {
    if (typeof v === "string") {
      if ((TOOL_NAMES as readonly string[]).includes(v)) return v as ToolName;
      const aliased = TOOL_ALIASES[v];
      if (aliased !== undefined) return aliased;
    }
    throw new CheaperError("INVALID_RUNTIME_POLICY", `unknown tool in policy: ${String(v)}`, {
      details: { tool: v },
    });
  });
  if (new Set(names).size !== names.length) {
    throw new CheaperError("INVALID_RUNTIME_POLICY", "policy tools.allow contains duplicates", {
      details: { reason: "tools-duplicate" },
    });
  }
  return names;
}

export function normalizePolicy(raw: Record<string, unknown>): RuntimePolicy {
  const execution = isPlainObject(raw.execution) ? raw.execution : {};
  let timeoutMs: number;
  try {
    timeoutMs = parseDuration(String(execution.timeout));
  } catch (e) {
    throw new CheaperError(
      "INVALID_RUNTIME_POLICY",
      `invalid execution.timeout: ${String(execution.timeout)}`,
      { details: { timeout: execution.timeout }, cause: e },
    );
  }
  const tools = isPlainObject(raw.tools) ? raw.tools : {};
  const filesystem = isPlainObject(raw.filesystem) ? raw.filesystem : {};
  const model = isPlainObject(raw.model) ? raw.model : {};
  const output = isPlainObject(raw.output) ? raw.output : {};
  const retry = isPlainObject(raw.retry) ? raw.retry : {};
  return {
    process: resolveProcessPolicy(raw.process as Partial<ProcessPolicy> | undefined),
    execution: {
      timeoutMs,
      maxTurns: execution.maxTurns as number,
      maxToolCalls: execution.maxToolCalls as number,
      concurrency: resolveConcurrencyPolicy(
        execution.concurrency as Partial<ConcurrencyPolicy> | undefined,
      ),
    },
    tools: { allow: normalizePolicyTools(tools.allow) },
    filesystem: {
      symlinks: filesystem.symlinks as "deny" | "within-workspace",
      denyPaths: Array.isArray(filesystem.denyPaths) ? (filesystem.denyPaths as string[]) : [],
    },
    model: {
      maxTier: model.maxTier as Tier,
      tierFallback: model.tierFallback as "none" | "higher",
    },
    output: {
      maxTokens: output.maxTokens as number,
      repairAttempts: output.repairAttempts as number,
    },
    retry: { transientAttempts: retry.transientAttempts as number },
  };
}
