import type { CompiledSchema } from "@cheaper/core";

export interface SchemaValidationError {
  instancePath: string;
  message: string;
}

export type ExtractedOutput =
  | { ok: true; result: Record<string, unknown>; rawOutput: string }
  | {
      ok: false;
      reason: "no-assistant-message" | "invalid-json" | "schema-mismatch";
      rawOutput: string;
      errors: SchemaValidationError[];
    };

/**
 * 最後の assistant message の text block を連結して返す。
 * AgentMessage を構造的に扱う（pi-agent-core への直接依存を避けるため）。
 */
export function extractLastAssistantText(messages: readonly unknown[]): string | undefined {
  for (let i = messages.length - 1; i >= 0; i--) {
    const m = messages[i];
    if (typeof m !== "object" || m === null) continue;
    const role = (m as { role?: unknown }).role;
    if (role !== "assistant") continue;
    const content = (m as { content?: unknown }).content;
    if (!Array.isArray(content)) return "";
    const texts = content
      .filter(
        (c): c is { type: "text"; text: string } =>
          typeof c === "object" &&
          c !== null &&
          (c as { type?: unknown }).type === "text" &&
          typeof (c as { text?: unknown }).text === "string",
      )
      .map((c) => c.text);
    return texts.join("");
  }
  return undefined;
}

/** 先頭末尾の ``` フェンスを 1 組だけ除去する。 */
export function stripCodeFence(text: string): string {
  let t = text.trim();
  const open = /^```[^\n]*\n/.exec(t);
  if (open) t = t.slice(open[0].length);
  if (/```\s*$/.test(t)) t = t.replace(/\n?```\s*$/, "");
  return t.trim();
}

function isPlainObject(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

/**
 * 最終 assistant 出力を抽出し output schema で検証する（DESIGN-0004）。
 * 失敗時は reason・rawOutput・errors を返す（repair が利用する）。
 */
export function extractOutput(
  messages: readonly unknown[],
  schema: CompiledSchema,
): ExtractedOutput {
  const raw = extractLastAssistantText(messages);
  if (raw === undefined) {
    return { ok: false, reason: "no-assistant-message", rawOutput: "", errors: [] };
  }
  const stripped = stripCodeFence(raw);
  let parsed: unknown;
  try {
    parsed = JSON.parse(stripped);
  } catch (e) {
    return {
      ok: false,
      reason: "invalid-json",
      rawOutput: raw,
      errors: [{ instancePath: "", message: e instanceof Error ? e.message : String(e) }],
    };
  }
  if (!isPlainObject(parsed)) {
    return {
      ok: false,
      reason: "schema-mismatch",
      rawOutput: raw,
      errors: [{ instancePath: "", message: "output is not a JSON object" }],
    };
  }
  const result = schema.validate(parsed);
  if (!result.ok) {
    return { ok: false, reason: "schema-mismatch", rawOutput: raw, errors: result.errors };
  }
  return { ok: true, result: parsed, rawOutput: raw };
}
