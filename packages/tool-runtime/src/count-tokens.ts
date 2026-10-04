import {
  countFileTokens,
  type PathDenylist,
  resolveWorkspacePath,
  type SymlinkPolicy,
} from "@cheaper/core";
import { Type } from "typebox";
import { defineCanonicalTool } from "./definition.js";
import { POLICY_DENIED_MESSAGE, rethrowAsDenied } from "./walker.js";

export interface CountTokensToolOptions {
  /** canonical 化済み workspace root。 */
  root: string;
  symlinkPolicy: SymlinkPolicy;
  denylist?: PathDenylist | undefined;
}

export const countTokensSchema = Type.Object({
  path: Type.String({ description: "Workspace-relative file path to count tokens for" }),
});

/**
 * canonical Token Counter の tool 入口（DESIGN-0012）。
 * core `countFileTokens` と同一実装を使い、CLI `count-tokens` と同じ
 * count を返す。file 本文は返さない。
 */
export function createCountTokensTool(opts: CountTokensToolOptions) {
  return defineCanonicalTool({
    name: "count_tokens",
    label: "count_tokens",
    description:
      "Count the canonical o200k_base tokens of a workspace file. Returns path, tokens, bytes, and tokenizer identity. Does not return file contents.",
    parameters: countTokensSchema,
    async execute(params) {
      try {
        if (opts.denylist !== undefined) {
          const resolved = await resolveWorkspacePath(opts.root, params.path, opts.symlinkPolicy);
          if (opts.denylist.isDenied(resolved.display)) {
            throw new Error(POLICY_DENIED_MESSAGE);
          }
        }
        const result = await countFileTokens({
          root: opts.root,
          path: params.path,
          symlinkPolicy: opts.symlinkPolicy,
        });
        return {
          content: [{ type: "text" as const, text: JSON.stringify(result) }],
          details: { ...result, tokenizer: { ...result.tokenizer } },
        };
      } catch (e) {
        rethrowAsDenied(e);
      }
    },
  });
}
