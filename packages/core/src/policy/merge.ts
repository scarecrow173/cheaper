import { parseDuration } from "../duration.js";

function isPlainObject(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

/**
 * Runtime Policy の merge 規則（引継ぎ §22）:
 * object は deep merge、scalar は後段 replace、array は array 全体 replace。
 * 入力は変更せず、新しい object を返す。
 */
export function mergePolicyLayers(
  base: Record<string, unknown>,
  override: Record<string, unknown>,
  options?: { mode: "replace" | "restrict-concurrency" },
): Record<string, unknown> {
  return merge(base, override, options?.mode === "restrict-concurrency", []);
}

function merge(
  base: Record<string, unknown>,
  override: Record<string, unknown>,
  restrict: boolean,
  parent: string[],
): Record<string, unknown> {
  const out: Record<string, unknown> = { ...base };
  for (const [key, value] of Object.entries(override)) {
    const existing = out[key];
    const keys = [...parent, key];
    const field = keys.join(".");
    if (
      restrict &&
      [
        "execution.concurrency.maxOperations",
        "workflow.maxJobs",
        "process.maxConcurrent",
        "process.maxOutputBytes",
      ].includes(field) &&
      typeof existing === "number" &&
      typeof value === "number"
    ) {
      out[key] = Math.min(existing, value);
    } else if (
      restrict &&
      ["process.timeout", "process.killGrace", "process.heartbeat.interval"].includes(field) &&
      typeof existing === "string" &&
      typeof value === "string"
    ) {
      out[key] = parseDuration(existing) <= parseDuration(value) ? existing : value;
    } else if (
      restrict &&
      field === "execution.concurrency.mutations" &&
      (existing === "serialized" || value === "serialized")
    ) {
      out[key] = "serialized";
    } else {
      out[key] =
        isPlainObject(existing) && isPlainObject(value)
          ? merge(existing, value, restrict, keys)
          : value;
    }
  }
  return out;
}
