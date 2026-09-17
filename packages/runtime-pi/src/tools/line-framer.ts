/**
 * 上限付き行フレーマー（DESIGN-0019）。raw Buffer 上で `0x0A` を走査して
 * 行を切り出す。`0x0A` は UTF-8 multibyte sequence に現れないため byte 走査は
 * UTF-8 安全であり、行 byte 長は decode 前の raw byte で計測する。
 * `maxLineBytes` を超えた行は蓄積を止めて次の `\n` まで破棄モードに入り、
 * `onOversized` を発行して行番号を消費する（保持量は chunk + maxLineBytes
 * 以下で、入力総 byte 数 N に依存しない）。`\r` は行内容に残す
 * （既存 `split("\n")` との parity）。
 */

const LF = 0x0a;

export interface LineFramerOptions {
  /** 評価対象にする 1 行の raw byte 上限（`GREP_MAX_LINE_BYTES`）。 */
  maxLineBytes: number;
  /** 行確定時の callback。`lineNo` は 1-indexed、`rawBytes` は decode 前 byte 長。 */
  onLine: (line: string, lineNo: number, rawBytes: number) => void;
  /** `maxLineBytes` 超過で破棄した行の callback。行番号は消費される。 */
  onOversized: (lineNo: number) => void;
}

export interface LineFramer {
  feed(buf: Buffer): void;
  /** EOF の未終端行を 1 行として emit する。 */
  flush(): void;
}

export function createLineFramer(opts: LineFramerOptions): LineFramer {
  let pending: Buffer[] = [];
  let pendingBytes = 0;
  let discarding = false;
  let lineNo = 1;

  const takePending = (): Buffer => {
    const raw = pending.length === 1 ? (pending[0] as Buffer) : Buffer.concat(pending);
    pending = [];
    pendingBytes = 0;
    return raw;
  };

  return {
    feed(buf: Buffer): void {
      let i = 0;
      while (i < buf.length) {
        const nl = buf.indexOf(LF, i);
        if (discarding) {
          // oversized 行の残りを次の \n まで捨てる（行番号は超過時点で消費済み）
          if (nl === -1) return;
          discarding = false;
          i = nl + 1;
          continue;
        }
        const end = nl === -1 ? buf.length : nl;
        if (pendingBytes + (end - i) > opts.maxLineBytes) {
          pending = [];
          pendingBytes = 0;
          opts.onOversized(lineNo);
          lineNo++;
          if (nl === -1) {
            discarding = true;
            return;
          }
          i = nl + 1;
          continue;
        }
        if (end > i) {
          pending.push(buf.subarray(i, end));
          pendingBytes += end - i;
        }
        if (nl === -1) return;
        const raw = takePending();
        opts.onLine(raw.toString("utf8"), lineNo, raw.length);
        lineNo++;
        i = nl + 1;
      }
    },
    flush(): void {
      if (discarding || pendingBytes === 0) return;
      const raw = takePending();
      opts.onLine(raw.toString("utf8"), lineNo, raw.length);
      lineNo++;
    },
  };
}
