import type { ModelConfig } from "./config.js";
import type { Tier } from "./tier.js";

/** Pi 型に依存しない core 側の最小 model shape。 */
export interface ModelInfo {
  provider: string;
  id: string;
  cost?: { input: number; output: number };
  maxTokens?: number;
}

/**
 * DESIGN-0003 の built-in 分類表。model id を小文字化して評価し、
 * high → low → medium の順で最初に一致した tier を採用する。
 */
const BUILTIN_TABLE: ReadonlyArray<{ tier: Tier; patterns: RegExp[] }> = [
  {
    tier: "high",
    patterns: [/opus/, /gpt-5(\.\d+)?-pro/, /\bo1(-pro)?\b/, /o3-pro/, /heavy/, /ultra/],
  },
  {
    tier: "low",
    patterns: [
      /haiku/,
      /\bmini\b/, // "gemini" 内の部分文字列を誤 match させない（gemini-.*-pro は medium）
      /nano/,
      /flash/,
      /lite/,
      /small/,
      /codex-mini/,
      /\b(1|3|7|8)b\b/,
    ],
  },
  {
    tier: "medium",
    patterns: [
      /sonnet/,
      /gpt-4o/,
      /gpt-4\.1/,
      /gpt-5(\.\d+)?$/,
      /gemini-.*-pro/,
      /\bo3\b/,
      /codex/,
    ],
  },
];

/**
 * tier 分類 priority: overrides > built-in 表 > cost heuristic > medium。
 * cost heuristic は cost.output（USD/1M token）: <2 → low、≤20 → medium、>20 → high。
 * cost 欠落・0 → medium。
 */
export function classifyModel(model: ModelInfo, config: ModelConfig): Tier {
  const override = config.overrides[`${model.provider}/${model.id}`];
  if (override !== undefined) return override;

  const id = model.id.toLowerCase();
  for (const { tier, patterns } of BUILTIN_TABLE) {
    if (patterns.some((p) => p.test(id))) return tier;
  }

  const output = model.cost?.output;
  if (output === undefined || output === 0) return "medium";
  if (output < 2) return "low";
  if (output <= 20) return "medium";
  return "high";
}
