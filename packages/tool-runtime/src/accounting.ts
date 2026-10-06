import { countTextTokens } from "@cheaper/core";
import { isAccessDeniedError } from "./walker.js";

export interface ToolBudgetOptions {
  maxToolCalls: number;
  onLimit: () => void;
}
export interface ToolBudget {
  readonly toolCalls: number;
  readonly rejectedToolCalls: number;
  readonly limitHit: "maxToolCalls" | null;
}
export interface ToolMeter {
  readonly sourceTokens: number;
}

/** 同期 admission が sibling call の await より先に上限を予約する。 */
export function createToolBudget(opts: ToolBudgetOptions) {
  let toolCalls = 0;
  let rejectedToolCalls = 0;
  let limitHit: "maxToolCalls" | null = null;
  const budget: ToolBudget = {
    get toolCalls() {
      return toolCalls;
    },
    get rejectedToolCalls() {
      return rejectedToolCalls;
    },
    get limitHit() {
      return limitHit;
    },
  };
  return {
    budget,
    async run<T>(execute: () => Promise<T>): Promise<T> {
      if (toolCalls >= opts.maxToolCalls) {
        limitHit = "maxToolCalls";
        rejectedToolCalls++;
        opts.onLimit();
        throw new Error(`tool call rejected: maxToolCalls limit reached (${opts.maxToolCalls})`);
      }
      toolCalls++;
      try {
        return await execute();
      } catch (e) {
        if (isAccessDeniedError(e)) rejectedToolCalls++;
        throw e;
      }
    },
  };
}

export function createToolMeter() {
  let sourceTokens = 0;
  const meter: ToolMeter = {
    get sourceTokens() {
      return sourceTokens;
    },
  };
  return {
    meter,
    async run<T>(execute: () => Promise<T>): Promise<T> {
      const result = await execute();
      const content = (result as { content?: unknown } | undefined)?.content;
      if (Array.isArray(content))
        for (const part of content) {
          const text = (part as { text?: unknown } | undefined)?.text;
          if (typeof text === "string") sourceTokens += countTextTokens(text);
        }
      return result;
    },
  };
}
