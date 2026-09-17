import { type FileHandle, open, stat } from "node:fs/promises";
import path from "node:path";
import { StringDecoder } from "node:string_decoder";
import { type PathDenylist, resolveWorkspacePath, type SymlinkPolicy } from "@cheaper/core";
import { defineTool } from "@earendil-works/pi-coding-agent";
import { Type } from "typebox";
import {
  IO_CHUNK_BYTES,
  READ_DEFAULT_LIMIT,
  READ_MAX_BYTES,
  READ_MAX_LIMIT,
  SNIFF_BYTES,
} from "./limits.js";
import { POLICY_DENIED_MESSAGE, rethrowAsDenied } from "./walker.js";

const LF = 0x0a;
const EMPTY = Buffer.alloc(0);

/** read が明示 error で拒否する画像拡張子（大小文字非依存、DESIGN-0019）。 */
export const IMAGE_EXTENSIONS = new Set([".png", ".jpg", ".jpeg", ".gif", ".webp", ".bmp"]);

export interface ReadToolOptions {
  /** canonical 化済み workspace root。 */
  root: string;
  symlinkPolicy: SymlinkPolicy;
  denylist?: PathDenylist | undefined;
  /** 本文 byte 上限の縮退注入（既定 `READ_MAX_BYTES`）。テスト用。 */
  maxBytes?: number;
  /** `FileHandle.read` chunk byte 数の縮退注入（既定 `IO_CHUNK_BYTES`）。テスト用。 */
  chunkBytes?: number;
}

export interface ReadToolDetails {
  /** 返した先頭行（1-indexed）。 */
  startLine: number;
  /** 返した末尾行（inclusive）。1 行も返せなかった場合は startLine - 1。 */
  endLine: number;
  /** この呼び出しで EOF（最終行）まで emit したか。 */
  eof: boolean;
  truncatedBy: "limit" | "bytes" | null;
  /** 継続読み取り位置。truncated 時のみ。 */
  continuation?: { offset: number; byteInLine?: number };
  /** 総行数。eof === true のときのみ返す（前方走査は行わない）。 */
  totalLines?: number;
}

const readSchema = Type.Object({
  path: Type.String({ description: "Workspace-relative file path" }),
  offset: Type.Optional(
    Type.Integer({ minimum: 1, description: "1-indexed start line (default 1)" }),
  ),
  limit: Type.Optional(
    Type.Integer({
      minimum: 1,
      maximum: READ_MAX_LIMIT,
      default: READ_DEFAULT_LIMIT,
    }),
  ),
  byteInLine: Type.Optional(
    Type.Integer({
      minimum: 0,
      description: "Byte offset within the `offset` line, for continuing an oversized line",
    }),
  ),
});

function formatLimit(maxBytes: number): string {
  return maxBytes % 1024 === 0 ? `${maxBytes / 1024}KB` : `${maxBytes}B`;
}

interface ScanParams {
  offset: number;
  limit: number;
  byteInLine: number | undefined;
  maxBytes: number;
  chunkBytes: number;
  signal: AbortSignal | undefined;
}

interface ReadResult {
  content: [{ type: "text"; text: string }];
  details: ReadToolDetails;
}

/**
 * `fh` を `chunkBytes` 単位で前方走査し、`offset` 行から `limit` 行・
 * `maxBytes` byte までを emit する（DESIGN-0019）。保持量は O(C + R) で、
 * 行全体・file 全体を蓄積しない。行分割は raw Buffer 上の `0x0A` 走査で行い
 * （UTF-8 multibyte sequence に `0x0A` は現れないため安全）、decode は行単位で
 * `StringDecoder("utf8")` を使い不正 byte は U+FFFD に置換する。
 * `byteInLine` は常にファイル byte 基準。
 */
async function readBounded(fh: FileHandle, size: number, p: ScanParams): Promise<ReadResult> {
  const buf = Buffer.alloc(p.chunkBytes);
  let chunkStart = 0; // buf[0] のファイル byte 位置
  let nextRead = 0; // 次に読むファイル byte 位置（単調増加）
  let fileEnd = size; // EOF 観測で確定する実効ファイル終端
  let sniffed = false;

  const checkAbort = () => {
    if (p.signal?.aborted) throw new Error("Operation aborted");
  };

  // buf が pos を含むまで順方向に読み進める。EOF で false。
  // 最初の chunk はそのまま NUL sniff（先頭 SNIFF_BYTES 分）を兼ね、追加 I/O はしない。
  const ensure = async (pos: number): Promise<boolean> => {
    checkAbort();
    while (pos >= nextRead) {
      const { bytesRead } = await fh.read(buf, 0, p.chunkBytes, nextRead);
      if (bytesRead === 0) {
        fileEnd = nextRead;
        return false;
      }
      if (!sniffed) {
        sniffed = true;
        if (buf.subarray(0, Math.min(SNIFF_BYTES, bytesRead)).includes(0)) {
          throw new Error("read: binary file (NUL byte within first 8KiB)");
        }
      }
      chunkStart = nextRead;
      nextRead += bytesRead;
      checkAbort();
    }
    return true;
  };

  // 現在の chunk 内で pos 以降の次の LF の絶対位置を返す（無ければ -1）。
  const lfInChunk = (pos: number): number => {
    const rel = buf.indexOf(LF, pos - chunkStart);
    return rel >= 0 && chunkStart + rel < nextRead ? chunkStart + rel : -1;
  };

  // [from, before) の次の LF の絶対位置。範囲走完了で -2、EOF で -1。
  const scanLfBefore = async (from: number, before: number): Promise<number> => {
    let cur = from;
    for (;;) {
      if (cur >= before) return -2;
      if (!(await ensure(cur))) return -1;
      const nl = lfInChunk(cur);
      if (nl >= 0) return nl < before ? nl : -2;
      cur = nextRead;
    }
  };

  // 1. offset 行の開始 byte 位置を特定する（prefix は chunk 単位で破棄、保持 O(C)）。
  let lineStart = 0;
  if (p.offset > 1) {
    let cur = 0;
    let nls = 0;
    for (;;) {
      if (!(await ensure(cur))) {
        // EOF 到達時点で総行数は判明済み（split("\n") 意味論: LF 数 + 1）。
        throw new Error(`Offset ${p.offset} is beyond end of file (${nls + 1} lines total)`);
      }
      const nl = lfInChunk(cur);
      if (nl < 0) {
        cur = nextRead;
        continue;
      }
      nls++;
      if (nls === p.offset - 1) {
        lineStart = nl + 1;
        break;
      }
      cur = nl + 1;
    }
  }

  // 2. byteInLine の検証と UTF-8 境界スナップ。
  let emitFrom = lineStart;
  if (p.byteInLine !== undefined) {
    const k = p.byteInLine;
    // [lineStart, lineStart+k+1) に行末（LF or EOF）が来るなら行 byte 長 ≤ k で error。
    const r = await scanLfBefore(lineStart, lineStart + k + 1);
    if (r !== -2) {
      const lineLen = (r === -1 ? fileEnd : r) - lineStart;
      throw new Error(`byteInLine ${k} is beyond end of line ${p.offset} (${lineLen} bytes)`);
    }
    emitFrom = lineStart + k;
    // UTF-8 文字境界へ前進スナップ（最大 3 byte、後退は継続の進行性を壊すため行わない）。
    for (let i = 0; i < 3; i++) {
      if (!(await ensure(emitFrom))) break;
      const b = buf[emitFrom - chunkStart];
      if (b === undefined || (b & 0xc0) !== 0x80) break;
      emitFrom++;
    }
  }

  // 3. emit ループ。行ごとに残り byte 予算を引き、limit 行または maxBytes で継続を確定する。
  const decoder = new StringDecoder("utf8");
  const outLines: string[] = [];
  let emittedBytes = 0; // join 後本文 byte 数（marker・行・`\n` 連結を含む）
  let emittedLines = 0;
  let wroteSomething = false;
  let endLine = p.offset - 1;
  let eof = false;
  let truncatedBy: "limit" | "bytes" | null = null;
  let continuation: { offset: number; byteInLine?: number } | undefined;
  let trailer: string | undefined;

  // byteInLine 再開時は先頭に partial marker 行を付ける（本文 byte 予算に計上）。
  if (emitFrom > lineStart) {
    const marker = `[partial line ${p.offset}: continuing from byte ${emitFrom - lineStart}]`;
    outLines.push(marker);
    emittedBytes += Buffer.byteLength(marker, "utf8");
    wroteSomething = true;
  }

  let lineNo = p.offset;
  let curStart = lineStart;
  let curEmit = emitFrom;

  for (;;) {
    const pieces: Buffer[] = [];
    const emitBeginInLine = curEmit - curStart;
    let lineEmittedBytes = 0;
    let lineDone = false;
    let terminated = false;
    let budgetStop = false;

    while (!lineDone && !budgetStop) {
      let regionEnd: number;
      let termHere = false;
      if (await ensure(curEmit)) {
        const nl = lfInChunk(curEmit);
        if (nl >= 0) {
          regionEnd = nl;
          termHere = true;
        } else {
          regionEnd = nextRead;
        }
      } else {
        regionEnd = fileEnd;
      }
      const want = regionEnd - curEmit;
      if (want > 0) {
        // join 用 `\n` は本文 byte 予算に計上するが、separator 自体は
        // outLines.join("\n") が供給するため pieces には入れない。
        const sep = pieces.length === 0 && wroteSomething ? 1 : 0;
        const take = Math.min(want, p.maxBytes - emittedBytes - sep);
        if (take <= 0) {
          budgetStop = true;
          break;
        }
        emittedBytes += sep;
        pieces.push(Buffer.from(buf.subarray(curEmit - chunkStart, curEmit - chunkStart + take)));
        emittedBytes += take;
        lineEmittedBytes += take;
        curEmit += take;
        if (curEmit < regionEnd) {
          budgetStop = true;
          break;
        }
        continue;
      }
      // want === 0: 行末（`\n` 直上 or EOF）。空行は join 用 `\n` 分だけ予算を見る。
      if (pieces.length === 0 && wroteSomething) {
        if (emittedBytes + 1 > p.maxBytes) {
          budgetStop = true;
          break;
        }
        emittedBytes += 1;
      }
      lineDone = true;
      terminated = termHere;
    }

    if (pieces.length > 0 || lineDone) {
      // 空行も join 要素として emit する（split("\n") 意味論の verbatim 連結）。
      outLines.push(decoder.end(pieces.length > 0 ? Buffer.concat(pieces) : EMPTY));
      endLine = lineNo;
      wroteSomething = true;
    }

    if (budgetStop) {
      truncatedBy = "bytes";
      if (lineEmittedBytes === 0) {
        // byte cap が行境界で到達。
        continuation = { offset: lineNo };
        trailer = `[Reached ${formatLimit(p.maxBytes)} limit. Use offset=${lineNo} to continue.]`;
      } else {
        // byte cap が行途中で到達。byteInLine 継続を返す。
        const c = emitBeginInLine + lineEmittedBytes;
        continuation = { offset: lineNo, byteInLine: c };
        trailer = `[Line ${lineNo} emitted bytes ${emitBeginInLine}-${c} (reached ${formatLimit(p.maxBytes)} limit). Use offset=${lineNo} byteInLine=${c} to continue.]`;
      }
      break;
    }

    emittedLines++;
    if (!terminated) {
      // EOF の未終端行 = 最終行。総行数はこの時点で確定。
      eof = true;
      break;
    }
    if (emittedLines >= p.limit) {
      truncatedBy = "limit";
      continuation = { offset: lineNo + 1 };
      trailer = `[Showing lines ${p.offset}-${lineNo}. Use offset=${lineNo + 1} to continue.]`;
      break;
    }
    lineNo++;
    curStart = curEmit + 1; // 行末 `\n` の次 byte から次行が始まる
    curEmit = curStart;
  }

  const details: ReadToolDetails = {
    startLine: p.offset,
    endLine,
    eof,
    truncatedBy,
    ...(continuation !== undefined ? { continuation } : {}),
    ...(eof ? { totalLines: endLine } : {}),
  };
  let text = outLines.join("\n");
  if (trailer !== undefined) text += `\n${trailer}`;
  return { content: [{ type: "text" as const, text }], details };
}

/**
 * bounded read tool（DESIGN-0019）。Pi `createReadToolDefinition` + 全文
 * `readFile` operations を置き換える自前実装。境界検証は count-tokens と同一
 * 経路（`resolveWorkspacePath` + `rethrowAsDenied` + `denylist.isDenied` →
 * `POLICY_DENIED_MESSAGE`）で、迂回経路は作らない。
 * `IO_CHUNK_BYTES` 単位の `FileHandle.read` で prefix を破棄しながら `offset`
 * 行まで走査し、最大 `limit` 行・`READ_MAX_BYTES` までを返す。巨大 file は
 * `offset`/`byteInLine` で継続読み取りする。FileHandle は呼び出しごとに open
 * して cache せず、`finally` で必ず close する。
 */
export function createReadTool(opts: ReadToolOptions) {
  return defineTool({
    name: "read",
    label: "read",
    description:
      "Read a UTF-8 text file from the workspace. Returns up to `limit` lines (default 2000, max 2000) and at most 50KB per call. Use `offset` (1-indexed line number) to page through large files and `byteInLine` (byte offset within the offset line) to continue inside an oversized line. Binary and image files are rejected with an explicit error; invalid bytes are replaced with U+FFFD. Positions refer to the file as read; changed files may need re-reading from an earlier offset.",
    parameters: readSchema,
    async execute(_toolCallId, params, signal) {
      if (params.byteInLine !== undefined && params.offset === undefined) {
        throw new Error("byteInLine requires offset");
      }
      const scan: ScanParams = {
        offset: params.offset ?? 1,
        limit: Math.min(params.limit ?? READ_DEFAULT_LIMIT, READ_MAX_LIMIT),
        byteInLine: params.byteInLine,
        maxBytes: opts.maxBytes ?? READ_MAX_BYTES,
        chunkBytes: opts.chunkBytes ?? IO_CHUNK_BYTES,
        signal,
      };
      try {
        const resolved = await resolveWorkspacePath(opts.root, params.path, opts.symlinkPolicy);
        if (opts.denylist?.isDenied(resolved.display)) {
          throw new Error(POLICY_DENIED_MESSAGE);
        }
        // stat を open 前に行い directory 等を platform 非依存の明示 error にする。
        const st = await stat(resolved.real);
        if (!st.isFile()) throw new Error("read: not a regular file");
        // 画像は走査前に明示 error。画像経路・全文 fallback は設けない。
        if (IMAGE_EXTENSIONS.has(path.extname(resolved.real).toLowerCase())) {
          throw new Error("read does not support image files");
        }
        const fh = await open(resolved.real, "r");
        try {
          return await readBounded(fh, st.size, scan);
        } finally {
          await fh.close();
        }
      } catch (e) {
        rethrowAsDenied(e);
      }
    },
  });
}
