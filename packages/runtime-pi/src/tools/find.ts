import { createFindTool as canonical, type WalkOptions } from "@cheaper/tool-runtime";
import { toPiTool } from "./adapter.js";

export type { WalkOptions } from "@cheaper/tool-runtime";
export function createFindTool(opts: WalkOptions) {
  return toPiTool(canonical(opts));
}
