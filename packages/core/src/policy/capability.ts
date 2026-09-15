import { CheaperError } from "../errors.js";
import { compareTier } from "../model/tier.js";
import type { ToolName } from "../tools/vocabulary.js";
import type { WorkerDefinition } from "../worker/validate.js";
import type { RuntimePolicy } from "./schema.js";

/**
 * capability 交差判定（DESIGN-0001 / 引継ぎ §25, §27）。
 * requested tools ⊄ policy.tools.allow → POLICY_VIOLATION（missing を details に）。
 * worker tier > policy.model.maxTier → POLICY_VIOLATION。
 * 問題なければ effective tools（= worker.tools）を返す。部分実行はしない。
 */
export function checkCapabilities(worker: WorkerDefinition, policy: RuntimePolicy): ToolName[] {
  const allowed = new Set<string>(policy.tools.allow);
  const missing = worker.tools.filter((t) => !allowed.has(t));
  if (missing.length > 0) {
    throw new CheaperError(
      "POLICY_VIOLATION",
      `worker requires tools not allowed by policy: ${missing.join(", ")}`,
      { details: { missing } },
    );
  }
  const requestedTier = worker.cheaper.model.tier;
  if (compareTier(requestedTier, policy.model.maxTier) > 0) {
    throw new CheaperError(
      "POLICY_VIOLATION",
      `worker tier ${requestedTier} exceeds policy maxTier ${policy.model.maxTier}`,
      { details: { requestedTier, maxTier: policy.model.maxTier } },
    );
  }
  return worker.tools;
}
