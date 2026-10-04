import { CheaperError } from "../errors.js";
import { type ConcurrencyPolicy, type RuntimePolicy, resolveConcurrencyPolicy } from "./schema.js";

/**
 * CLI-style policy restriction（引継ぎ §30 / 共有 ADR D2）。
 * `--timeout` が policy の timeoutMs を超える場合は POLICY_VIOLATION（silent clamp しない）。
 */
export function restrictPolicy(
  policy: RuntimePolicy,
  restrictions: { timeoutMs?: number; concurrency?: Partial<ConcurrencyPolicy> },
): RuntimePolicy {
  const requested = restrictions.timeoutMs;
  if (requested === undefined && restrictions.concurrency === undefined) return policy;
  if (requested !== undefined && requested > policy.execution.timeoutMs) {
    throw new CheaperError(
      "POLICY_VIOLATION",
      `--timeout ${requested}ms exceeds policy timeout ${policy.execution.timeoutMs}ms`,
      { details: { requested, policy: policy.execution.timeoutMs } },
    );
  }
  const execution = {
    ...policy.execution,
    ...(requested === undefined ? {} : { timeoutMs: requested }),
  };
  if (restrictions.concurrency !== undefined) {
    const current = resolveConcurrencyPolicy(policy.execution.concurrency);
    const next = resolveConcurrencyPolicy({ ...current, ...restrictions.concurrency });
    if (
      next.maxOperations > current.maxOperations ||
      (current.mutations === "serialized" && next.mutations !== "serialized")
    ) {
      throw new CheaperError("POLICY_VIOLATION", "concurrency restriction exceeds runtime policy", {
        details: { requested: next, policy: current },
      });
    }
    execution.concurrency = next;
  }
  return { ...policy, execution };
}
