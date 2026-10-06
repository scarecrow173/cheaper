import { createToolBudget, type ToolBudget, type ToolBudgetOptions } from "@cheaper/tool-runtime";
import type { ToolDefinition } from "@earendil-works/pi-coding-agent";

export type { ToolBudget, ToolBudgetOptions } from "@cheaper/tool-runtime";
export interface BudgetedTools {
  tools: ToolDefinition[];
  budget: ToolBudget;
}
/** 旧公開 wrapper の互換 adapter。registry は canonical accounting を直接使用する。 */
export function wrapToolBudget(tools: ToolDefinition[], opts: ToolBudgetOptions): BudgetedTools {
  const { budget, run } = createToolBudget(opts);
  return {
    budget,
    tools: tools.map((tool) => ({
      ...tool,
      execute: (...args) => run(() => tool.execute(...args)),
    })),
  };
}
