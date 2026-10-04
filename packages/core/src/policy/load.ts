import { readFile } from "node:fs/promises";
import path from "node:path";
import { parse as parseYaml } from "yaml";
import { CheaperError } from "../errors.js";
import { mergePolicyLayers } from "./merge.js";
import {
  type ConcurrencyPolicy,
  normalizePolicy,
  type RuntimePolicy,
  resolveConcurrencyPolicy,
  validatePolicySchema,
} from "./schema.js";

function isPlainObject(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

async function readPolicyFile(
  filePath: string,
  required: boolean,
): Promise<Record<string, unknown> | undefined> {
  let text: string;
  try {
    text = await readFile(filePath, "utf8");
  } catch (e) {
    const errno = (e as NodeJS.ErrnoException).code;
    if (errno === "ENOENT" || errno === "ENOTDIR") {
      // 未配置のみ skip。それ以外の読み込み失敗（EACCES・EISDIR・壊れた
      // symlink 等）は silent skip せずエラーにする。
      if (!required) return undefined;
      throw new CheaperError("INVALID_RUNTIME_POLICY", `policy file not found: ${filePath}`, {
        details: { path: filePath },
        cause: e,
      });
    }
    throw new CheaperError("INVALID_RUNTIME_POLICY", `policy file read failed: ${filePath}`, {
      details: { path: filePath, reason: "read-failed", errno },
      cause: e,
    });
  }
  let parsed: unknown;
  try {
    parsed = parseYaml(text);
  } catch (e) {
    throw new CheaperError("INVALID_RUNTIME_POLICY", `policy yaml parse failed: ${filePath}`, {
      details: { path: filePath, reason: "yaml-parse" },
      cause: e,
    });
  }
  if (!isPlainObject(parsed)) {
    throw new CheaperError("INVALID_RUNTIME_POLICY", `policy is not an object: ${filePath}`, {
      details: { path: filePath, reason: "not-object" },
    });
  }
  return parsed;
}

/**
 * 引継ぎ §23 のロード順:
 * built-in(complete) → user(partial) → merge → project(partial) → merge → effective(complete)。
 * invalid な policy は silent ignore しない。
 */
export async function loadPolicy(opts: {
  builtinPolicyPath: string;
  userConfigDir: string;
  projectDir: string;
}): Promise<RuntimePolicy> {
  const builtinRaw = await readPolicyFile(opts.builtinPolicyPath, true);
  let effective = validatePolicySchema(builtinRaw, "complete");

  const user = await readPolicyFile(path.join(opts.userConfigDir, "policy.yaml"), false);
  if (user !== undefined) {
    effective = mergePolicyLayers(effective, validatePolicySchema(user, "partial"));
  }
  // trusted merge 後、project restriction 前に legacy/partial omission を補完する。
  const execution = effective.execution as Record<string, unknown>;
  effective = {
    ...effective,
    execution: {
      ...execution,
      concurrency: resolveConcurrencyPolicy(
        execution.concurrency as Partial<ConcurrencyPolicy> | undefined,
      ),
    },
  };
  const project = await readPolicyFile(path.join(opts.projectDir, "policy.yaml"), false);
  if (project !== undefined) {
    effective = mergePolicyLayers(effective, validatePolicySchema(project, "partial"), {
      mode: "restrict-concurrency",
    });
  }

  return normalizePolicy(validatePolicySchema(effective, "complete"));
}
