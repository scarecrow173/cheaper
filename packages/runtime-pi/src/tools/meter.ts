import { createToolMeter, type ToolMeter } from "@cheaper/tool-runtime";
import type { ToolDefinition } from "@earendil-works/pi-coding-agent";

export type { ToolMeter } from "@cheaper/tool-runtime";
export interface MeteredTools {
  tools: ToolDefinition[];
  meter: ToolMeter;
}
/** 旧公開 wrapper の互換 adapter。registry は二重に wrap しない。 */
export function wrapToolMeter(tools: ToolDefinition[]): MeteredTools {
  const { meter, run } = createToolMeter();
  return {
    meter,
    tools: tools.map((tool) => ({
      ...tool,
      execute: (...args) => run(() => tool.execute(...args)),
    })),
  };
}
