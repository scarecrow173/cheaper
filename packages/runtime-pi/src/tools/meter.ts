import { countTextTokens } from "@cheaper/core";
import type { ToolDefinition } from "@earendil-works/pi-coding-agent";

export interface ToolMeter {
  /** 全 tool output の text に対する canonical token 和。 */
  readonly sourceTokens: number;
}

export interface MeteredTools {
  tools: ToolDefinition[];
  meter: ToolMeter;
}

/**
 * 全 ToolDefinition の `execute` を wrap し、戻り値の
 * `content[].text` を canonical tokenizer で encode して累積する
 * （ADR-034rzGnXqru47VgHNL16cS）。throw した call は計上しない。
 * これは「worker が親の代わりに ingest した内容量」の実測であり、
 * `meta.savings.sourceTokens` の供給元。
 */
export function wrapToolMeter(tools: ToolDefinition[]): MeteredTools {
  let sourceTokens = 0;

  const wrapped = tools.map((tool) => {
    const execute = tool.execute.bind(tool);
    const def: ToolDefinition = {
      ...tool,
      execute: async (toolCallId, params, signal, onUpdate, ctx) => {
        const result = await execute(toolCallId, params, signal, onUpdate, ctx);
        const content = (result as { content?: unknown } | undefined)?.content;
        if (Array.isArray(content)) {
          for (const part of content) {
            const text = (part as { text?: unknown } | undefined)?.text;
            if (typeof text === "string") sourceTokens += countTextTokens(text);
          }
        }
        return result;
      },
    };
    return def;
  });

  return {
    tools: wrapped,
    meter: {
      get sourceTokens() {
        return sourceTokens;
      },
    },
  };
}
