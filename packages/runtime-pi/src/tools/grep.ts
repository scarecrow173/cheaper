import { readFile } from "node:fs/promises";
import path from "node:path";
import { defineTool } from "@earendil-works/pi-coding-agent";
import { Type } from "typebox";
import { rethrowAsDenied, type WalkOptions, walkFiles } from "./walker.js";

const DEFAULT_MAX_RESULTS = 100;
const MAX_MAX_RESULTS = 500;

const grepSchema = Type.Object({
  pattern: Type.String({ description: "JavaScript regular expression" }),
  path: Type.Optional(Type.String({ description: "Search root, workspace-relative (default: .)" })),
  glob: Type.Optional(Type.String({ description: "Glob filter on workspace-relative paths" })),
  ignoreCase: Type.Optional(Type.Boolean()),
  maxResults: Type.Optional(
    Type.Integer({ minimum: 1, maximum: MAX_MAX_RESULTS, default: DEFAULT_MAX_RESULTS }),
  ),
  context: Type.Optional(Type.Integer({ minimum: 0, maximum: 3, default: 0 })),
});

/**
 * 自前 sandboxed grep（DESIGN-0002）。外部プロセス（rg 等）を spawn せず、
 * 共有 walker で `.git`/`node_modules`/バイナリ/1MB 超を skip する。
 * 出力: match は `path:line: text`、context 行は `path-line- text`、
 * 非連続ブロック間に `--`。
 */
export function createGrepTool(opts: WalkOptions) {
  return defineTool({
    name: "grep",
    label: "grep",
    description:
      "Search file contents with a JavaScript regular expression. Returns matching lines as path:line: text. Skips .git, node_modules, binary files, and files over 1MB.",
    parameters: grepSchema,
    async execute(_toolCallId, params) {
      let re: RegExp;
      try {
        re = new RegExp(params.pattern, params.ignoreCase === true ? "i" : "");
      } catch (e) {
        throw new Error(`invalid grep pattern: ${e instanceof Error ? e.message : String(e)}`);
      }
      const startAbs = params.path ?? ".";
      const maxResults = Math.min(params.maxResults ?? DEFAULT_MAX_RESULTS, MAX_MAX_RESULTS);
      const context = params.context ?? 0;

      const out: string[] = [];
      let matches = 0;
      let truncated = false;
      try {
        outer: for await (const file of walkFiles(opts, startAbs)) {
          if (params.glob !== undefined && !path.posix.matchesGlob(file.display, params.glob)) {
            continue;
          }
          let text: string;
          try {
            text = await readFile(file.real, "utf8");
          } catch {
            continue;
          }
          const lines = text.split("\n");
          const matchedAt: number[] = [];
          for (let i = 0; i < lines.length; i++) {
            if (re.test(lines[i] as string)) matchedAt.push(i);
          }
          if (matchedAt.length === 0) continue;
          // context 込みの出力ブロックを構築（重なりはマージ）
          const blocks: Array<{ from: number; to: number }> = [];
          for (const i of matchedAt) {
            const from = Math.max(0, i - context);
            const to = Math.min(lines.length - 1, i + context);
            const last = blocks[blocks.length - 1];
            if (last !== undefined && from <= last.to + 1) last.to = Math.max(last.to, to);
            else blocks.push({ from, to });
          }
          let firstBlock = true;
          for (const block of blocks) {
            if (context > 0 && !firstBlock) out.push("--");
            firstBlock = false;
            for (let i = block.from; i <= block.to; i++) {
              const isMatch = matchedAt.includes(i);
              const line = lines[i] as string;
              out.push(
                isMatch ? `${file.display}:${i + 1}: ${line}` : `${file.display}-${i + 1}- ${line}`,
              );
              if (isMatch) {
                matches++;
                if (matches >= maxResults) {
                  truncated = true;
                  break outer;
                }
              }
            }
          }
        }
      } catch (e) {
        rethrowAsDenied(e);
      }
      if (truncated) out.push(`[truncated: more matches]`);
      const body = out.length === 0 ? "(no matches)" : out.join("\n");
      return { content: [{ type: "text" as const, text: body }], details: undefined };
    },
  });
}
