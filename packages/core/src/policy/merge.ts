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
): Record<string, unknown> {
  const out: Record<string, unknown> = { ...base };
  for (const [key, value] of Object.entries(override)) {
    const existing = out[key];
    out[key] =
      isPlainObject(existing) && isPlainObject(value) ? mergePolicyLayers(existing, value) : value;
  }
  return out;
}
