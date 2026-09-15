import { readFile } from "node:fs/promises";
import path from "node:path";
import { Worker } from "node:worker_threads";
import { defineTool } from "@earendil-works/pi-coding-agent";
import { Type } from "typebox";
import { rethrowAsDenied, type WalkOptions, walkFiles } from "./walker.js";

const DEFAULT_MAX_RESULTS = 100;
const MAX_MAX_RESULTS = 500;
const DEFAULT_EVAL_PER_FILE_MS = 5_000;
const DEFAULT_EVAL_TOTAL_MS = 60_000;
const EVAL_TIMEOUT_MESSAGE = "grep pattern evaluation timed out";

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
 * regex 評価だけを逃がす worker の自己完結 eval ソース。
 * `new Worker(code, { eval: true })` で動くためファイル配置不要（vitest/dist 両対応）。
 * `require("node:worker_threads").parentPort` のみ使用し、他 import は持たない。
 * 行ごとに `re.test` するだけの純粋処理。catastrophic backtracking でブロックされても
 * main thread の event loop は止まらず、`terminate()` で強制停止できる。
 */
const GREP_EVAL_WORKER_SOURCE = String.raw`
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
  if (msg.type === "file" && re !== null) {
    const matched = [];
    const lines = String(msg.text).split("\n");
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
  resolve: (matched: number[]) => void;
  reject: (err: Error) => void;
  timer: NodeJS.Timeout;
}

/**
 * eval worker の薄いラッパー。`init` で pattern を送り、`file` メッセージごとに
 * match 行 index 配列を受け取る。1 ファイル評価が `perFileMs` を超えたら
 * `worker.terminate()` し `EVAL_TIMEOUT_MESSAGE` の普通の Error で reject
 * （tool error 扱い・rejectedToolCalls には計上されない）。
 */
class GrepEvalWorker {
  private readonly worker: Worker;
  private nextId = 0;
  private readonly pending = new Map<number, PendingEval>();
  private dead = false;

  constructor(
    pattern: string,
    flags: string,
    private readonly perFileMs: number,
  ) {
    this.worker = new Worker(GREP_EVAL_WORKER_SOURCE, { eval: true });
    this.worker.on("message", (msg: GrepWorkerReply) => {
      if (msg === null || typeof msg !== "object") return;
      if (msg.type === "result" && typeof msg.id === "number") {
        const p = this.pending.get(msg.id);
        if (p !== undefined) {
          this.pending.delete(msg.id);
          clearTimeout(p.timer);
          p.resolve(Array.isArray(msg.matched) ? (msg.matched as number[]) : []);
        }
      } else if (msg.type === "error") {
        // init 失敗（main 側で事前検証済みのため到達しない想定だが fail closed）
        this.failAll(new Error(`invalid grep pattern: ${String(msg.message)}`));
      }
    });
    this.worker.on("error", (err) => {
      this.failAll(err instanceof Error ? err : new Error(String(err)));
    });
    this.worker.postMessage({ type: "init", pattern, flags });
  }

  evaluate(display: string, text: string): Promise<number[]> {
    if (this.dead) return Promise.reject(new Error(EVAL_TIMEOUT_MESSAGE));
    const id = this.nextId++;
    return new Promise<number[]>((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id);
        this.dead = true;
        void this.worker.terminate();
        reject(new Error(EVAL_TIMEOUT_MESSAGE));
      }, this.perFileMs);
      this.pending.set(id, {
        resolve: (matched) => {
          clearTimeout(timer);
          resolve(matched);
        },
        reject: (err) => {
          clearTimeout(timer);
          reject(err);
        },
        timer,
      });
      this.worker.postMessage({ type: "file", id, display, text });
    });
  }

  private failAll(err: Error): void {
    this.dead = true;
    for (const [id, p] of this.pending) {
      this.pending.delete(id);
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
  /** 1 ファイルの regex 評価 watchdog ms（既定 5000）。超過で tool error。 */
  evalPerFileMs?: number;
  /** 1 回の grep 呼び出しの累計 regex 評価上限 ms（既定 60000）。超過で truncated。 */
  evalTotalMs?: number;
}

/**
 * 自前 sandboxed grep（DESIGN-0002）。外部プロセス（rg 等）を spawn せず、
 * 共有 walker で `.git`/`node_modules`/バイナリ/1MB 超を skip する。
 * 正規表現評価は eval worker で行い、event loop ブロック（ReDoS）を防ぐ。
 * 出力: match は `path:line: text`、context 行は `path-line- text`、
 * 非連続ブロック間に `--`。
 */
export function createGrepTool(opts: GrepToolOptions) {
  return defineTool({
    name: "grep",
    label: "grep",
    description:
      "Search file contents with a JavaScript regular expression. Returns matching lines as path:line: text. Skips .git, node_modules, binary files, and files over 1MB.",
    parameters: grepSchema,
    async execute(_toolCallId, params) {
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
      const perFileMs = opts.evalPerFileMs ?? DEFAULT_EVAL_PER_FILE_MS;
      const totalMs = opts.evalTotalMs ?? DEFAULT_EVAL_TOTAL_MS;

      const evaluator = new GrepEvalWorker(re.source, re.flags, perFileMs);
      const out: string[] = [];
      let matches = 0;
      let truncated = false;
      let evalTruncated = false;
      let evalTotal = 0;
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
          const t0 = performance.now();
          const matchedAt = await evaluator.evaluate(file.display, text);
          evalTotal += performance.now() - t0;
          if (matchedAt.length === 0) {
            if (evalTotal >= totalMs) {
              evalTruncated = true;
              break;
            }
            continue;
          }
          const lines = text.split("\n");
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
          if (evalTotal >= totalMs) {
            evalTruncated = true;
            break;
          }
        }
      } catch (e) {
        rethrowAsDenied(e);
      } finally {
        await evaluator.dispose();
      }
      if (truncated) out.push(`[truncated: more matches]`);
      if (evalTruncated) out.push(`[truncated: evaluation time limit reached]`);
      const body = out.length === 0 ? "(no matches)" : out.join("\n");
      return { content: [{ type: "text" as const, text: body }], details: undefined };
    },
  });
}
