import { type FileHandle, open } from "node:fs/promises";
import path from "node:path";
import { Worker } from "node:worker_threads";
import { Type } from "typebox";
import { defineCanonicalTool } from "./definition.js";
import {
  GREP_BATCH_MAX_BYTES,
  GREP_BATCH_MAX_LINES,
  GREP_EVAL_PER_FILE_MS,
  GREP_EVAL_TOTAL_MS,
  GREP_MAX_LINE_BYTES,
  GREP_OUTPUT_LINE_MAX_CHARS,
  GREP_OUTPUT_MAX_BYTES,
  GREP_PATTERN_MAX_CHARS,
  GREP_SCAN_DEADLINE_MS,
  GREP_WORKER_MAX_OLD_SPACE_MB,
  IO_CHUNK_BYTES,
  SKIP_FOOTER_MAX_BYTES,
  SKIP_FOOTER_MAX_ENTRIES,
  SKIP_PATH_MAX_CHARS,
} from "./limits.js";
import { createLineFramer } from "./line-framer.js";
import { rethrowAsDenied, sniffTextFile, type WalkOptions, walkFiles } from "./walker.js";

const DEFAULT_MAX_RESULTS = 100;
const MAX_MAX_RESULTS = 500;
const EVAL_TIMEOUT_MESSAGE = "grep pattern evaluation timed out";
const ABORTED_MESSAGE = "Operation aborted";

export const grepSchema = Type.Object({
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
 * regex 評価だけを逃がす worker の自己完結 eval ソース。
 * `new Worker(code, { eval: true })` で動くためファイル配置不要（vitest/dist 両対応）。
 * `require("node:worker_threads").parentPort` のみ使用し、他 import は持たない。
 * `batch` メッセージの `lines` 各行に `re.test` するだけの純粋処理で、
 * batch 内 0-indexed の一致 index 配列を返す。catastrophic backtracking で
 * ブロックされても main thread の event loop は止まらず、`terminate()` で強制停止できる。
 */
const GREP_EVAL_WORKER_SOURCE = `
"use strict";
const { parentPort } = require("node:worker_threads");
let re = null;
parentPort.on("message", (msg) => {
  if (msg === null || typeof msg !== "object") return;
  if (msg.type === "init") {
    try {
      re = new RegExp(msg.pattern, msg.flags);
      parentPort.postMessage({ type: "ready" });
    } catch (e) {
      parentPort.postMessage({
        type: "error",
        message: e && e.message ? e.message : String(e),
      });
    }
    return;
  }
  if (msg.type === "batch" && re !== null) {
    const matched = [];
    const lines = msg.lines;
    for (let i = 0; i < lines.length; i++) {
      if (re.test(lines[i])) matched.push(i);
    }
    parentPort.postMessage({ type: "result", id: msg.id, matched });
  }
});
`;

interface GrepWorkerReply {
  type?: unknown;
  id?: unknown;
  matched?: unknown;
  message?: unknown;
}

interface PendingEval {
  id: number;
  resolve: (matched: number[]) => void;
  reject: (err: Error) => void;
  timer: NodeJS.Timeout;
}

/**
 * eval worker の薄いラッパー。`init` で pattern を送り、`batch` メッセージごとに
 * batch 内 0-indexed の一致 index 配列を受け取る。未応答 batch は常に最大 1 で、
 * `evaluate` は前の batch の result を受け取ってからしか呼ばれない。
 * `watchMs` は呼び出し側が per-file・total・scan deadline の残予算の最小値として
 * 計算して渡し、超過で `terminate()` して `EVAL_TIMEOUT_MESSAGE` で reject する。
 * `resourceLimits.maxOldGenerationSizeMb` で worker heap も上限化する（DESIGN-0019）。
 */
class GrepEvalWorker {
  private readonly worker: Worker;
  private nextId = 0;
  private pending: PendingEval | undefined;
  private dead = false;

  constructor(pattern: string, flags: string) {
    this.worker = new Worker(GREP_EVAL_WORKER_SOURCE, {
      eval: true,
      resourceLimits: { maxOldGenerationSizeMb: GREP_WORKER_MAX_OLD_SPACE_MB },
    });
    this.worker.on("message", (msg: GrepWorkerReply) => {
      if (msg === null || typeof msg !== "object") return;
      if (msg.type === "result" && typeof msg.id === "number") {
        const p = this.pending;
        if (p !== undefined && p.id === msg.id) {
          this.pending = undefined;
          clearTimeout(p.timer);
          p.resolve(Array.isArray(msg.matched) ? (msg.matched as number[]) : []);
        }
      } else if (msg.type === "error") {
        // init 失敗（main 側で事前検証済みのため到達しない想定だが fail closed）。
        // 後続の再生成に stale worker を残さないよう terminate も行う。
        this.failAll(new Error(`invalid grep pattern: ${String(msg.message)}`));
        void this.worker.terminate();
      }
    });
    this.worker.on("error", (err) => {
      this.failAll(err instanceof Error ? err : new Error(String(err)));
    });
    this.worker.postMessage({ type: "init", pattern, flags });
  }

  get isDead(): boolean {
    return this.dead;
  }

  evaluate(
    display: string,
    startLine: number,
    lines: string[],
    watchMs: number,
  ): Promise<number[]> {
    if (this.dead || this.pending !== undefined) {
      return Promise.reject(new Error(EVAL_TIMEOUT_MESSAGE));
    }
    const id = this.nextId++;
    return new Promise<number[]>((resolve, reject) => {
      const timer = setTimeout(
        () => {
          this.pending = undefined;
          this.dead = true;
          void this.worker.terminate();
          reject(new Error(EVAL_TIMEOUT_MESSAGE));
        },
        Math.max(1, watchMs),
      );
      this.pending = {
        id,
        resolve: (matched) => {
          clearTimeout(timer);
          this.pending = undefined;
          resolve(matched);
        },
        reject: (err) => {
          clearTimeout(timer);
          this.pending = undefined;
          reject(err);
        },
        timer,
      };
      this.worker.postMessage({ type: "batch", id, display, startLine, lines });
    });
  }

  /** per-file 予算超過などで worker を止める。後続 file 用に呼び出し側が再生成する。 */
  kill(): void {
    this.failAll(new Error(EVAL_TIMEOUT_MESSAGE));
    void this.worker.terminate();
  }

  private failAll(err: Error): void {
    this.dead = true;
    const p = this.pending;
    this.pending = undefined;
    if (p !== undefined) {
      clearTimeout(p.timer);
      p.reject(err);
    }
  }

  async dispose(): Promise<void> {
    this.failAll(new Error(EVAL_TIMEOUT_MESSAGE));
    await this.worker.terminate();
  }
}

export interface GrepToolOptions extends WalkOptions {
  /** 同一 file の全 batch に累積される regex 評価予算 ms（既定 `GREP_EVAL_PER_FILE_MS`）。超過で file を timeout skip。 */
  evalPerFileMs?: number;
  /** 1 回の grep 呼び出しの累計 regex 評価上限 ms（既定 `GREP_EVAL_TOTAL_MS`）。超過で走査停止。 */
  evalTotalMs?: number;
  /** walk+I/O+eval 待機を含む走査全体の wall-clock deadline ms（既定 `GREP_SCAN_DEADLINE_MS`）。 */
  scanDeadlineMs?: number;
  /** `FileHandle.read` chunk byte 数の縮退注入（既定 `IO_CHUNK_BYTES`）。テスト用。 */
  chunkBytes?: number;
  /**
   * 評価対象にする 1 行の raw byte 上限の縮退注入（既定 `GREP_MAX_LINE_BYTES`）。テスト用。
   * raw byte の行は UTF-8 decode で最大 3 倍へ展開しうる（invalid byte →
   * U+FFFD 3 byte）ため、decoded payload は最大 `3 * maxLineBytes`。
   */
  maxLineBytes?: number;
  /** 1 batch の行数上限の縮退注入（既定 `GREP_BATCH_MAX_LINES`）。テスト用。 */
  batchMaxLines?: number;
  /**
   * 1 batch の行テキスト合計 byte 上限の縮退注入（既定 `GREP_BATCH_MAX_BYTES`）。テスト用。
   * invariant: `batchMaxBytes >= 3 * maxLineBytes`（1 行の最大 decoded
   * payload が空 batch に収まり、「1 batch ≤ B」が厳密に保たれる）。
   * 違反する組み合わせは `createGrepTool` 構築時に reject される。
   */
  batchMaxBytes?: number;
  /** 出力本文の総 byte 上限の縮退注入（既定 `GREP_OUTPUT_MAX_BYTES`）。テスト用。 */
  outputMaxBytes?: number;
  /** 出力 1 行の表示文字数上限の縮退注入（既定 `GREP_OUTPUT_LINE_MAX_CHARS`）。テスト用。 */
  outputLineMaxChars?: number;
  /** skip 理由ごとの代表 path 件数の縮退注入（既定 `SKIP_FOOTER_MAX_ENTRIES`）。テスト用。 */
  skipFooterMaxEntries?: number;
  /** footer 内 1 path の表示文字数の縮退注入（既定 `SKIP_PATH_MAX_CHARS`）。テスト用。 */
  skipPathMaxChars?: number;
  /** skip footer 全体の byte 上限の縮退注入（既定 `SKIP_FOOTER_MAX_BYTES`）。テスト用。 */
  skipFooterMaxBytes?: number;
  /** eval 予算・deadline の時間源の差し替え（既定 `performance.now`）。テスト用。 */
  now?: () => number;
}

interface PendingLine {
  lineNo: number;
  text: string;
  /** worker へ送る decoded payload の UTF-8 byte 長（batch byte 計上用）。 */
  payloadBytes: number;
}

interface SkipBucket {
  shown: string[];
  total: number;
}

/**
 * 自前 sandboxed grep（DESIGN-0002 / DESIGN-0019）。外部プロセス（rg 等）を
 * spawn せず、path-only `walkFiles` → `sniffTextFile` → `IO_CHUNK_BYTES` chunk
 * 読み取り → `createLineFramer` → batch protocol（K 行 / B byte 締め・未応答 1）
 * で eval worker に流す streaming 構成。全文 `readFile` はしない。
 * eval 予算は per-file（batch 間累積・超過で timeout skip + worker 再生成）と
 * total（超過で走査停止）を持ち、scan deadline は部分結果 + footer で返す。
 * 出力: match は `path:line: text`、context 行は `path-line- text`、
 * 非連続ブロック間に `--`。binary/unreadable/oversized-line/timeout は有界
 * footer で報告する（deny 対象は walker filter 段で消えるため一切出ない）。
 */
export function createGrepTool(opts: GrepToolOptions) {
  // invariant: B >= 3 * L。1 行の最大 decoded payload（invalid UTF-8 byte が
  // U+FFFD 3 byte へ展開しうるため 3 * maxLineBytes）が空 batch に収まる
  // ことを要求し、「1 batch ≤ B」を厳密に保つ。違反は縮退注入でのみ
  // 発生しうるため、構築時に reject する。
  const effMaxLineBytes = opts.maxLineBytes ?? GREP_MAX_LINE_BYTES;
  const effBatchMaxBytes = opts.batchMaxBytes ?? GREP_BATCH_MAX_BYTES;
  if (effBatchMaxBytes < 3 * effMaxLineBytes) {
    throw new Error(
      `grep: batchMaxBytes (${effBatchMaxBytes}) must be >= 3 * maxLineBytes ` +
        `(${3 * effMaxLineBytes}); a single decoded line payload must fit in one batch`,
    );
  }
  return defineCanonicalTool({
    name: "grep",
    label: "grep",
    description:
      "Search file contents with a JavaScript regular expression. Returns matching lines as path:line: text. Searches allowed files of any size; binary files, unreadable files, oversized lines, and per-file evaluation timeouts are skipped and reported in a bounded footer. Skips .git and node_modules.",
    parameters: grepSchema,
    async execute(params, signal) {
      if (params.pattern.length > GREP_PATTERN_MAX_CHARS) {
        throw new Error(
          `invalid grep pattern: pattern exceeds ${GREP_PATTERN_MAX_CHARS} characters`,
        );
      }
      const flags = params.ignoreCase === true ? "i" : "";
      let re: RegExp;
      try {
        re = new RegExp(params.pattern, flags);
      } catch (e) {
        throw new Error(`invalid grep pattern: ${e instanceof Error ? e.message : String(e)}`);
      }
      const startAbs = params.path ?? ".";
      const maxResults = Math.min(params.maxResults ?? DEFAULT_MAX_RESULTS, MAX_MAX_RESULTS);
      const context = params.context ?? 0;
      const perFileMs = opts.evalPerFileMs ?? GREP_EVAL_PER_FILE_MS;
      const totalMs = opts.evalTotalMs ?? GREP_EVAL_TOTAL_MS;
      const now = opts.now ?? (() => performance.now());
      const deadlineAt = now() + (opts.scanDeadlineMs ?? GREP_SCAN_DEADLINE_MS);
      const chunkBytes = opts.chunkBytes ?? IO_CHUNK_BYTES;
      const maxLineBytes = opts.maxLineBytes ?? GREP_MAX_LINE_BYTES;
      const batchMaxLines = opts.batchMaxLines ?? GREP_BATCH_MAX_LINES;
      const batchMaxBytes = opts.batchMaxBytes ?? GREP_BATCH_MAX_BYTES;
      const outputMaxBytes = opts.outputMaxBytes ?? GREP_OUTPUT_MAX_BYTES;
      const outputLineMaxChars = opts.outputLineMaxChars ?? GREP_OUTPUT_LINE_MAX_CHARS;
      const footerMaxEntries = opts.skipFooterMaxEntries ?? SKIP_FOOTER_MAX_ENTRIES;
      const pathMaxChars = opts.skipPathMaxChars ?? SKIP_PATH_MAX_CHARS;
      const footerMaxBytes = opts.skipFooterMaxBytes ?? SKIP_FOOTER_MAX_BYTES;

      let evaluator = new GrepEvalWorker(re.source, re.flags);
      let onAbort: (() => void) | undefined;
      const abortPromise =
        signal === undefined
          ? undefined
          : new Promise<never>((_resolve, reject) => {
              onAbort = () => reject(new Error(ABORTED_MESSAGE));
              if (signal.aborted) onAbort();
              else signal.addEventListener("abort", onAbort, { once: true });
            });
      // race で負けた側の reject が未処理にならないよう握りつぶす
      void abortPromise?.catch(() => {});

      const outLines: string[] = [];
      let outBytes = 0;
      let matches = 0;
      let evalTotalMs = 0;
      // 走査停止理由。先に立ったものを記録し、部分結果 + marker として返す（error にしない）。
      let stopReason: "result-cap" | "output" | "eval" | "deadline" | undefined;
      // 各 reason の代表 entry は footerMaxEntries 件まで保持し、残りは total で
      // `and N more` に集計する（oversized 行が大量にあっても蓄積は有界）。
      const skipped: Record<"binary" | "unreadable" | "oversized" | "timeout", SkipBucket> = {
        binary: { shown: [], total: 0 },
        unreadable: { shown: [], total: 0 },
        oversized: { shown: [], total: 0 },
        timeout: { shown: [], total: 0 },
      };
      const recordSkip = (bucket: SkipBucket, entry: string): void => {
        bucket.total++;
        if (bucket.shown.length < footerMaxEntries) {
          bucket.shown.push(entry.length > pathMaxChars ? entry.slice(0, pathMaxChars) : entry);
        }
      };

      // abort は cleanup 後に reject、deadline は部分結果 + marker で走査を止める。
      const checkScan = (): void => {
        if (signal?.aborted) throw new Error(ABORTED_MESSAGE);
        if (now() >= deadlineAt) stopReason ??= "deadline";
      };

      const emitRaw = (s: string): boolean => {
        const need = Buffer.byteLength(s, "utf8") + (outLines.length > 0 ? 1 : 0);
        if (outBytes + need > outputMaxBytes) {
          stopReason ??= "output";
          return false;
        }
        outLines.push(s);
        outBytes += need;
        return true;
      };

      try {
        for await (const file of walkFiles(opts, startAbs, (issue) => {
          recordSkip(skipped.unreadable, issue.display);
        })) {
          checkScan();
          if (stopReason !== undefined) break;
          if (params.glob !== undefined && !path.posix.matchesGlob(file.display, params.glob)) {
            continue;
          }
          const sniff = await sniffTextFile(file.real, opts.observeAccess);
          if (sniff === "binary") {
            recordSkip(skipped.binary, file.display);
            continue;
          }
          if (sniff === "unreadable") {
            recordSkip(skipped.unreadable, file.display);
            continue;
          }
          await opts.observeAccess?.(file.real, "shared");
          const fh: FileHandle | undefined = await open(file.real, "r").catch(() => undefined);
          if (fh === undefined) {
            recordSkip(skipped.unreadable, file.display);
            continue;
          }

          // per-file 状態。batch は常に連続行番号を保つ（oversized 行は行番号を
          // 消費するためそこで batch を締め、0-indexed ↔ 絶対行番号の写像を維持する）。
          const pendingLines: PendingLine[] = [];
          const ring: { lineNo: number; text: string }[] = [];
          let batchLines: string[] = [];
          let batchStart = 0;
          let batchBytes = 0;
          let fileEvalMs = 0;
          let openBlockTo = 0;
          let lastEmitted = 0;
          let needSep = false;
          let fileAbandoned = false;
          let fileBatches = 0;

          const emitText = (isMatch: boolean, lineNo: number, text: string): boolean => {
            const t =
              text.length > outputLineMaxChars
                ? `${text.slice(0, outputLineMaxChars)}... [truncated]`
                : text;
            return emitRaw(
              isMatch ? `${file.display}:${lineNo}: ${t}` : `${file.display}-${lineNo}- ${t}`,
            );
          };

          // batch 送信 → result 待ち → context 付き出力。eval 待ちの間は read が
          // 進まない（未応答 batch 最大 1 の backpressure）。
          const sendBatch = async (): Promise<void> => {
            const lines = batchLines;
            const start = batchStart;
            batchLines = [];
            batchBytes = 0;
            fileBatches++;
            if (evaluator.isDead) evaluator = new GrepEvalWorker(re.source, re.flags);
            const watchMs = Math.min(
              perFileMs - fileEvalMs,
              totalMs - evalTotalMs,
              deadlineAt - now(),
            );
            const t0 = now();
            let matched: number[];
            try {
              const evalP = evaluator.evaluate(file.display, start, lines, watchMs);
              matched =
                abortPromise === undefined
                  ? await evalP
                  : await Promise.race([evalP, abortPromise]);
            } catch (e) {
              if (signal?.aborted) throw new Error(ABORTED_MESSAGE);
              if (e instanceof Error && e.message === EVAL_TIMEOUT_MESSAGE) {
                evalTotalMs += now() - t0;
                recordSkip(skipped.timeout, file.display);
                if (now() >= deadlineAt) stopReason ??= "deadline";
                else if (evalTotalMs >= totalMs) stopReason ??= "eval";
                else fileAbandoned = true; // per-file 予算超過 → 当該 file を捨てて次へ
                return;
              }
              throw e;
            }
            const dt = now() - t0;
            fileEvalMs += dt;
            evalTotalMs += dt;

            // batch 内 0-indexed matched を startLine で絶対行番号へ写像し、
            // ring（前 context）と pending countdown（後 context）で block merge しながら
            // 出力へ流す。全行配列・全一致配列は作らない。
            const lookup = (lineNo: number): string | undefined =>
              lineNo >= start ? lines[lineNo - start] : ring.find((r) => r.lineNo === lineNo)?.text;
            let mi = 0;
            for (let n = start; n < start + lines.length; n++) {
              const isMatch = mi < matched.length && matched[mi] === n - start;
              if (isMatch) {
                mi++;
                const from = Math.max(1, n - context);
                if (from > openBlockTo + 1 && needSep && context > 0 && !emitRaw("--")) {
                  break;
                }
                for (let i = Math.max(from, lastEmitted + 1); i < n; i++) {
                  const t = lookup(i);
                  if (t !== undefined && !emitText(false, i, t)) break;
                }
                if (stopReason !== undefined) break;
                if (!emitText(true, n, lines[n - start] as string)) break;
                needSep = true;
                openBlockTo = Math.max(openBlockTo, n + context);
                lastEmitted = n;
                matches++;
                if (matches >= maxResults) {
                  stopReason ??= "result-cap";
                  break;
                }
              } else if (n <= openBlockTo) {
                if (!emitText(false, n, lines[n - start] as string)) break;
                lastEmitted = n;
              }
            }

            // ring を次 batch の pre-context 用に更新（直近 context 行のみ保持）
            for (let i = 0; i < lines.length; i++) {
              ring.push({ lineNo: start + i, text: lines[i] as string });
            }
            while (ring.length > context) ring.shift();
          };

          // pendingLines から batch を組み立て送信する。batch は
          // batchMaxLines 行・batchMaxBytes byte・oversized 行を挟む行番号の
          // 不連続のいずれかで締める。byte 計上は worker に実際に送る
          // decoded payload の UTF-8 byte 長で行い（rawBytes だと U+FFFD
          // 展開で実 payload が B を超え得る）、B は hard cap として
          // 超える行を積む前に締める。force 時は partial batch も送る。
          const drain = async (force: boolean): Promise<void> => {
            while (!fileAbandoned && stopReason === undefined) {
              let closedByBytes = false;
              while (batchLines.length < batchMaxLines && pendingLines.length > 0) {
                const head = pendingLines[0] as PendingLine;
                if (batchLines.length > 0 && head.lineNo !== batchStart + batchLines.length) {
                  break; // oversized 行を挟む不連続 → 現在 batch を締める
                }
                // B は hard cap: 次行を足すと超えるなら現在 batch を締める。
                // 空 batch には必ず積むため、1 行が B を超える場合（縮退した
                // test 注入 limit でのみ到達可能。実 limit では L が decoded
                // payload を B 未満に抑える）も 1 行 batch として送られ、
                // 進行は保証される。
                if (batchLines.length > 0 && batchBytes + head.payloadBytes > batchMaxBytes) {
                  closedByBytes = true;
                  break;
                }
                pendingLines.shift();
                if (batchLines.length === 0) batchStart = head.lineNo;
                batchLines.push(head.text);
                batchBytes += head.payloadBytes;
              }
              if (batchLines.length === 0) return;
              const closedByGap =
                pendingLines.length > 0 &&
                (pendingLines[0] as PendingLine).lineNo !== batchStart + batchLines.length;
              if (batchLines.length < batchMaxLines && !closedByBytes && !closedByGap && !force) {
                return; // partial batch は次の feed/EOF まで保留
              }
              // 送信前に残予算を検査（超過なら送らずに打ち切る）
              if (fileEvalMs >= perFileMs) {
                recordSkip(skipped.timeout, file.display);
                evaluator.kill();
                fileAbandoned = true;
                return;
              }
              if (evalTotalMs >= totalMs) {
                // file 未着手なら名指ししない（scan-level の停止は marker が担う）
                if (fileBatches > 0) recordSkip(skipped.timeout, file.display);
                stopReason ??= "eval";
                return;
              }
              if (now() >= deadlineAt) {
                if (fileBatches > 0) recordSkip(skipped.timeout, file.display);
                stopReason ??= "deadline";
                return;
              }
              await sendBatch();
            }
          };

          const framer = createLineFramer({
            maxLineBytes,
            onLine: (line, lineNo) => {
              pendingLines.push({
                lineNo,
                text: line,
                payloadBytes: Buffer.byteLength(line, "utf8"),
              });
            },
            onOversized: (lineNo) => {
              recordSkip(skipped.oversized, `${file.display}:${lineNo}`);
            },
          });

          try {
            const buf = Buffer.alloc(chunkBytes);
            let pos = 0;
            for (;;) {
              checkScan();
              if (stopReason !== undefined || fileAbandoned) break;
              let bytesRead: number;
              try {
                ({ bytesRead } = await fh.read(buf, 0, chunkBytes, pos));
              } catch {
                // 途中 I/O error → file は不完全。unreadable skip として記録し次へ。
                recordSkip(skipped.unreadable, file.display);
                fileAbandoned = true;
                break;
              }
              if (bytesRead === 0) break;
              pos += bytesRead;
              // framer は pending を subarray view で保持するため、buf 再利用前に
              // copy を渡す（chunk 境界をまたぐ行の破壊を防ぐ）。
              framer.feed(Buffer.from(buf.subarray(0, bytesRead)));
              await drain(false);
            }
            if (!fileAbandoned && stopReason === undefined) {
              framer.flush();
              await drain(true);
            }
          } finally {
            await fh.close();
          }
          if (stopReason !== undefined) break;
        }
      } catch (e) {
        rethrowAsDenied(e);
      } finally {
        if (signal !== undefined && onAbort !== undefined) {
          signal.removeEventListener("abort", onAbort);
        }
        await evaluator.dispose();
      }

      const footer: string[] = [];
      // [skipped: ...] 行。1 entry ≤ pathMaxChars、skipped 部全体は
      // footerMaxBytes byte で締め、超過は [skipped-report: truncated] で示す。
      let skippedBytes = 0;
      let skippedReportTruncated = false;
      for (const [reason, bucket] of [
        ["binary", skipped.binary],
        ["unreadable", skipped.unreadable],
        ["oversized-line", skipped.oversized],
        ["timeout", skipped.timeout],
      ] as const) {
        if (bucket.total === 0) continue;
        const more = bucket.total - bucket.shown.length;
        const list =
          more > 0
            ? `${bucket.shown.length > 0 ? `${bucket.shown.join(", ")}, ` : ""}and ${more} more`
            : bucket.shown.join(", ");
        const line = `[skipped: ${reason} — ${list}]`;
        const need = Buffer.byteLength(line, "utf8") + (footer.length > 0 ? 1 : 0);
        if (skippedBytes + need > footerMaxBytes) {
          skippedReportTruncated = true;
          break;
        }
        footer.push(line);
        skippedBytes += need;
      }
      if (stopReason === "eval") footer.push("[truncated: evaluation time limit reached]");
      if (stopReason === "deadline") footer.push("[truncated: scan deadline reached]");
      if (stopReason === "result-cap") footer.push("[truncated: more matches]");
      if (stopReason === "output") footer.push("[truncated: output limit reached]");
      if (skippedReportTruncated) footer.push("[skipped-report: truncated]");

      let body = outLines.length === 0 ? "(no matches)" : outLines.join("\n");
      if (footer.length > 0) body += `\n${footer.join("\n")}`;
      return { content: [{ type: "text" as const, text: body }] };
    },
  });
}
