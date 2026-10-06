import path from "node:path";
import { Type } from "typebox";
import { defineCanonicalTool } from "./definition.js";
import { SKIP_FOOTER_MAX_BYTES, SKIP_FOOTER_MAX_ENTRIES, SKIP_PATH_MAX_CHARS } from "./limits.js";
import { rethrowAsDenied, type WalkOptions, walkFiles } from "./walker.js";

const DEFAULT_MAX_RESULTS = 200;
const MAX_MAX_RESULTS = 1000;
const ABORTED_MESSAGE = "Operation aborted";

interface SkipBucket {
  shown: string[];
  total: number;
}

export const findSchema = Type.Object({
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
function skippedFooter(reason: string, bucket: SkipBucket): string {
  const more = bucket.total - bucket.shown.length;
  const list =
    more > 0
      ? `${bucket.shown.length > 0 ? `${bucket.shown.join(", ")}, ` : ""}and ${more} more`
      : bucket.shown.join(", ");
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
  return defineCanonicalTool({
    name: "find",
    label: "find",
    description:
      "List files whose workspace-relative path matches a glob (e.g. **/*.ts). Lists all allowed file paths regardless of size or content. Skips .git and node_modules.",
    parameters: findSchema,
    async execute(params, signal) {
      const startAbs = params.path ?? ".";
      const maxResults = Math.min(params.maxResults ?? DEFAULT_MAX_RESULTS, MAX_MAX_RESULTS);
      const out: string[] = [];
      // grep と同じ有界 skip 集計（DESIGN-0019）。代表 path は
      // SKIP_FOOTER_MAX_ENTRIES 件まで保持し、残りは total で `and N more` に
      // 集計する（列挙失敗が大量にあっても蓄積は有界）。
      const issues: SkipBucket = { shown: [], total: 0 };
      const recordIssue = (display: string): void => {
        issues.total++;
        if (issues.shown.length < SKIP_FOOTER_MAX_ENTRIES) {
          issues.shown.push(
            display.length > SKIP_PATH_MAX_CHARS ? display.slice(0, SKIP_PATH_MAX_CHARS) : display,
          );
        }
      };
      const checkAbort = (): void => {
        if (signal?.aborted) throw new Error(ABORTED_MESSAGE);
      };
      let truncated = false;
      try {
        checkAbort();
        for await (const file of walkFiles(opts, startAbs, (issue) => recordIssue(issue.display))) {
          checkAbort();
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
      if (issues.total > 0) {
        body += `\n${skippedFooter("unreadable", issues)}`;
      }
      return { content: [{ type: "text" as const, text: body }] };
    },
  });
}
