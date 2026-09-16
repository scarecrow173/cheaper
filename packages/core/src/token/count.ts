import { readFile } from "node:fs/promises";
import { getEncoding } from "js-tiktoken";
import {
  type ResolvedWorkspacePath,
  resolveWorkspacePath,
  type SymlinkPolicy,
} from "../workspace/paths.js";

export const TOKENIZER_NAME = "o200k_base" as const;
export const TOKENIZER_IMPLEMENTATION = "js-tiktoken" as const;
/** 導入 js-tiktoken の version。package.json の pin と一致させる（test で保証）。 */
export const TOKENIZER_VERSION = "1.0.21";

export interface TokenizerIdentity {
  name: typeof TOKENIZER_NAME;
  implementation: typeof TOKENIZER_IMPLEMENTATION;
  version: string;
}

export interface FileTokenCount {
  /** workspace 相対 POSIX 区切り path。 */
  path: string;
  tokens: number;
  bytes: number;
  tokenizer: TokenizerIdentity;
}

export const TOKENIZER_IDENTITY: TokenizerIdentity = {
  name: TOKENIZER_NAME,
  implementation: TOKENIZER_IMPLEMENTATION,
  version: TOKENIZER_VERSION,
};

// BPE rank table は module scope で 1 度だけロードする。
const encoding = getEncoding(TOKENIZER_NAME);

export interface CountFileTokensOptions {
  /** canonical 化済み workspace root。 */
  root: string;
  /** workspace 相対または絶対 path。boundary 解決する。 */
  path: string;
  symlinkPolicy: SymlinkPolicy;
}

/**
 * workspace 内 file の canonical token count（DESIGN-0012 / ADR-0011）。
 * boundary + symlink policy を `resolveWorkspacePath` で適用し、
 * utf8 lossy decode → o200k_base encode で決定的に数える。
 * file 本文は返さない。
 */
export async function countFileTokens(opts: CountFileTokensOptions): Promise<FileTokenCount> {
  const resolved: ResolvedWorkspacePath = await resolveWorkspacePath(
    opts.root,
    opts.path,
    opts.symlinkPolicy,
  );
  const buf = await readFile(resolved.real);
  const tokens = encoding.encode(buf.toString("utf8")).length;
  return {
    path: resolved.display,
    tokens,
    bytes: buf.length,
    tokenizer: TOKENIZER_IDENTITY,
  };
}
