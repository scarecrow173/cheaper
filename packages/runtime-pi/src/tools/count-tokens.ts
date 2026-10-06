import {
  type CountTokensToolOptions,
  createCountTokensTool as canonical,
} from "@cheaper/tool-runtime";
import { toPiTool } from "./adapter.js";

export type { CountTokensToolOptions } from "@cheaper/tool-runtime";
export function createCountTokensTool(opts: CountTokensToolOptions) {
  return toPiTool(canonical(opts));
}
