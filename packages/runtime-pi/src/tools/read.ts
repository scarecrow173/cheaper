import { createReadTool as canonical, type ReadToolOptions } from "@cheaper/tool-runtime";
import { toPiTool } from "./adapter.js";

export type { ReadToolDetails, ReadToolOptions } from "@cheaper/tool-runtime";
export { IMAGE_EXTENSIONS } from "@cheaper/tool-runtime";
export function createReadTool(opts: ReadToolOptions) {
  return toPiTool(canonical(opts));
}
