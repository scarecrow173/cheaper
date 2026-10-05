import { CheaperError } from "../errors.js";
import type { WorkerDefinition } from "./validate.js";

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** literal path の抽出だけを所有し、canonical identity は ResourceAccess に委ねる。 */
export function extractWriteScope(
  worker: WorkerDefinition,
  input: unknown,
): Set<string> | undefined {
  const field = worker.cheaper.writeScope;
  if (field === undefined) return undefined;
  const value = isPlainObject(input) ? input[field] : undefined;
  const invalid = (reason: string) =>
    new CheaperError("INVALID_INPUT", `writeScope field invalid: ${reason}`, {
      details: { field, reason },
    });
  if (!Array.isArray(value)) throw invalid("missing-or-not-array");
  const allowed = new Set<string>();
  for (const item of value) {
    const target =
      typeof item === "string"
        ? item
        : isPlainObject(item) && typeof item.path === "string"
          ? item.path
          : undefined;
    if (target === undefined) throw invalid("item-lacks-path");
    allowed.add(target);
  }
  return allowed;
}
