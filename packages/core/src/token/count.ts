import { readFile, stat } from "node:fs/promises";
import { getEncoding } from "js-tiktoken";
import {
  type ResolvedWorkspacePath,
  resolveWorkspacePath,
  type SymlinkPolicy,
} from "../workspace/paths.js";

/** count 対象 file の byte 上限（ADR-0015）。超過は read/encode せず失敗させる。 */
export const DEFAULT_COUNT_MAX_BYTES = 16 * 1024 * 1024;

/**
 * byte cap 超過を表す専用 Error（ADR-0015 / DESIGN-0017）。
 * CheaperError ではなく plain Error とし、呼び出し側（CLI/tool）が
 * 専用 code や tool error へ変換する。
 */
export class FileTooLargeError extends Error {
  readonly path: string;
  readonly bytes: number;
  readonly maxBytes: number;

  constructor(path: string, bytes: number, maxBytes: number) {
    super(`file exceeds byte cap for token counting: ${path} (${bytes} bytes > ${maxBytes})`);
    this.name = "FileTooLargeError";
    this.path = path;
    this.bytes = bytes;
    this.maxBytes = maxBytes;
  }
}

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
  /** byte cap。省略時は DEFAULT_COUNT_MAX_BYTES。 */
  maxBytes?: number;
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
  const maxBytes = opts.maxBytes ?? DEFAULT_COUNT_MAX_BYTES;
  const info = await stat(resolved.real);
  if (info.size > maxBytes) {
    throw new FileTooLargeError(resolved.display, info.size, maxBytes);
  }
  const buf = await readFile(resolved.real);
  const tokens = encoding.encode(buf.toString("utf8")).length;
  return {
    path: resolved.display,
    tokens,
    bytes: buf.length,
    tokenizer: TOKENIZER_IDENTITY,
  };
}
