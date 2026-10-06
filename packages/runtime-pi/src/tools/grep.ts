import { createGrepTool as canonical, type GrepToolOptions } from "@cheaper/tool-runtime";
import { toPiTool } from "./adapter.js";

export type { GrepToolOptions } from "@cheaper/tool-runtime";
export function createGrepTool(opts: GrepToolOptions) {
  return toPiTool(canonical(opts));
}
