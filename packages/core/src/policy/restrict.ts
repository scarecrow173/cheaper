import { CheaperError } from "../errors.js";
import type { RuntimePolicy } from "./schema.js";

/**
 * CLI による policy restriction（引継ぎ §30）。V1 で狭められるのは timeout のみ。
 * `--timeout` が policy の timeoutMs を超える場合は POLICY_VIOLATION（silent clamp しない）。
 */
export function restrictPolicy(
  policy: RuntimePolicy,
  restrictions: { timeoutMs?: number },
): RuntimePolicy {
  const requested = restrictions.timeoutMs;
  if (requested === undefined) return policy;
  if (requested > policy.execution.timeoutMs) {
    throw new CheaperError(
      "POLICY_VIOLATION",
      `--timeout ${requested}ms exceeds policy timeout ${policy.execution.timeoutMs}ms`,
      { details: { requested, policy: policy.execution.timeoutMs } },
    );
  }
  return { ...policy, execution: { ...policy.execution, timeoutMs: requested } };
}
