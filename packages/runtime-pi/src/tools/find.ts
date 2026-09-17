import path from "node:path";
import { defineTool } from "@earendil-works/pi-coding-agent";
import { Type } from "typebox";
import { SKIP_FOOTER_MAX_BYTES, SKIP_FOOTER_MAX_ENTRIES, SKIP_PATH_MAX_CHARS } from "./limits.js";
import { rethrowAsDenied, type WalkIssue, type WalkOptions, walkFiles } from "./walker.js";

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
 * 有界 skip footer（DESIGN-0019）。代表 path は `SKIP_FOOTER_MAX_ENTRIES` 件まで、
 * 残りは `and N more` の集計。1 path は `SKIP_PATH_MAX_CHARS` 文字で切り詰め、
 * footer 全体が `SKIP_FOOTER_MAX_BYTES` を超えたら `[skipped-report: truncated]` で締める。
 */
function skippedFooter(reason: string, displays: string[]): string {
  const shown = displays
    .slice(0, SKIP_FOOTER_MAX_ENTRIES)
    .map((d) => (d.length > SKIP_PATH_MAX_CHARS ? d.slice(0, SKIP_PATH_MAX_CHARS) : d));
  const rest = displays.length - shown.length;
  const list = rest > 0 ? `${shown.join(", ")}, and ${rest} more` : shown.join(", ");
  const line = `[skipped: ${reason} — ${list}]`;
  return Buffer.byteLength(line, "utf8") > SKIP_FOOTER_MAX_BYTES
    ? "[skipped-report: truncated]"
    : line;
}

/**
 * 自前 sandboxed find（DESIGN-0002 / DESIGN-0019）。`path.posix.matchesGlob` で
 * workspace 相対 display path に glob を適用し、外部プロセスを spawn しない。
 * path 列挙のみで内容の open/read/sniff は行わず、列挙失敗は有界 footer で報告する。
 */
export function createFindTool(opts: WalkOptions) {
  return defineTool({
    name: "find",
    label: "find",
    description:
      "List files whose workspace-relative path matches a glob (e.g. **/*.ts). Lists all allowed file paths regardless of size or content. Skips .git and node_modules.",
    parameters: findSchema,
    async execute(_toolCallId, params) {
      const startAbs = params.path ?? ".";
      const maxResults = Math.min(params.maxResults ?? DEFAULT_MAX_RESULTS, MAX_MAX_RESULTS);
      const out: string[] = [];
      const issues: WalkIssue[] = [];
      let truncated = false;
      try {
        for await (const file of walkFiles(opts, startAbs, (issue) => issues.push(issue))) {
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
      let body = out.length === 0 ? "(no matches)" : out.join("\n");
      if (issues.length > 0) {
        body += `\n${skippedFooter(
          "unreadable",
          issues.map((i) => i.display),
        )}`;
      }
      return { content: [{ type: "text" as const, text: body }], details: undefined };
    },
  });
}
