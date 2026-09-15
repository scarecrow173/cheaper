import path from "node:path";
import { defineTool } from "@earendil-works/pi-coding-agent";
import { Type } from "typebox";
import { rethrowAsDenied, type WalkOptions, walkFiles } from "./walker.js";

const DEFAULT_MAX_RESULTS = 200;
const MAX_MAX_RESULTS = 1000;

const findSchema = Type.Object({
  pattern: Type.String({ description: "Glob matched against workspace-relative paths" }),
  path: Type.Optional(Type.String({ description: "Search root, workspace-relative (default: .)" })),
  maxResults: Type.Optional(
    Type.Integer({ minimum: 1, maximum: MAX_MAX_RESULTS, default: DEFAULT_MAX_RESULTS }),
  ),
});

/**
 * 自前 sandboxed find（DESIGN-0002）。`path.posix.matchesGlob` で
 * workspace 相対 display path に glob を適用し、外部プロセスを spawn しない。
 */
export function createFindTool(opts: WalkOptions) {
  return defineTool({
    name: "find",
    label: "find",
    description:
      "List files whose workspace-relative path matches a glob (e.g. **/*.ts). Skips .git, node_modules, binary files, and files over 1MB.",
    parameters: findSchema,
    async execute(_toolCallId, params) {
      const startAbs = params.path ?? ".";
      const maxResults = Math.min(params.maxResults ?? DEFAULT_MAX_RESULTS, MAX_MAX_RESULTS);
      const out: string[] = [];
      let truncated = false;
      try {
        for await (const file of walkFiles(opts, startAbs)) {
          if (!path.posix.matchesGlob(file.display, params.pattern)) continue;
          if (out.length >= maxResults) {
            truncated = true;
            break;
          }
          out.push(file.display);
        }
      } catch (e) {
        rethrowAsDenied(e);
      }
      if (truncated) out.push(`[truncated: more results]`);
      const body = out.length === 0 ? "(no matches)" : out.join("\n");
      return { content: [{ type: "text" as const, text: body }], details: undefined };
    },
  });
}
