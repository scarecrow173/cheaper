import type { ToolDefinition } from "@earendil-works/pi-coding-agent";
import { isAccessDeniedError } from "./walker.js";

export interface ToolBudgetOptions {
  /** 許容する tool call 数の上限（policy.execution.maxToolCalls）。 */
  maxToolCalls: number;
  /**
   * 上限到達時に呼ばれるフック（run 側で `session.abort()` を起動する）。
   * abort の完了は待たない（execute 内で await するとデッドロックし得るため）。
   */
  onLimit: () => void;
}

export interface ToolBudget {
  /** 委譲された tool call 数（meta.toolCalls）。 */
  readonly toolCalls: number;
  /** boundary 拒否・budget 超過で error result になった回数（meta.rejectedToolCalls）。 */
  readonly rejectedToolCalls: number;
  /** 到達した制限（未到達は null）。 */
  readonly limitHit: "maxToolCalls" | null;
}

export interface BudgetedTools {
  tools: ToolDefinition[];
  budget: ToolBudget;
}

/**
 * 全 ToolDefinition の `execute` を wrap して maxToolCalls を強制する（DESIGN-0004）。
 * `count >= maxToolCalls` なら `limitHit` を記録し `onLimit()`（= abort 起動）のうえ
 * error として throw する。boundary 拒否（Access denied）の throw は rejectedToolCalls に計上。
 */
export function wrapToolBudget(tools: ToolDefinition[], opts: ToolBudgetOptions): BudgetedTools {
  let toolCalls = 0;
  let rejectedToolCalls = 0;
  let limitHit: "maxToolCalls" | null = null;

  const wrapped = tools.map((tool) => {
    const execute = tool.execute.bind(tool);
    const def: ToolDefinition = {
      ...tool,
      execute: async (toolCallId, params, signal, onUpdate, ctx) => {
        if (toolCalls >= opts.maxToolCalls) {
          limitHit = "maxToolCalls";
          rejectedToolCalls++;
          opts.onLimit();
          throw new Error(`tool call rejected: maxToolCalls limit reached (${opts.maxToolCalls})`);
        }
        toolCalls++;
        try {
          return await execute(toolCallId, params, signal, onUpdate, ctx);
        } catch (e) {
          if (isAccessDeniedError(e)) rejectedToolCalls++;
          throw e;
        }
      },
    };
    return def;
  });

  return {
    tools: wrapped,
    budget: {
      get toolCalls() {
        return toolCalls;
      },
      get rejectedToolCalls() {
        return rejectedToolCalls;
      },
      get limitHit() {
        return limitHit;
      },
    },
  };
}
