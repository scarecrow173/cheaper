import childProcess from "node:child_process";
import * as fsp from "node:fs/promises";
import { mkdir, mkdtemp, realpath, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { createPathDenylist } from "@cheaper/core";
import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { createGrepTool, type GrepToolOptions } from "../../src/tools/grep.js";
import {
  GREP_BATCH_MAX_LINES,
  GREP_PATTERN_MAX_CHARS,
  IO_CHUNK_BYTES,
} from "../../src/tools/limits.js";

// fs/promises を mock して readFile 不呼出・open 経由の fh.read 呼出量を計測する。
// open/readFile 以外は importOriginal のまま（fixture 生成を妨げない）。
vi.mock("node:fs/promises", async (importOriginal) => {
  const orig = await importOriginal<typeof import("node:fs/promises")>();
  return {
    ...orig,
    readFile: vi.fn(orig.readFile),
    open: vi.fn(orig.open),
  };
});

const readCalls: Array<{ length: number; position: number | null }> = [];
const openMock = vi.mocked(fsp.open);
const realOpen = openMock.getMockImplementation();
if (realOpen === undefined) throw new Error("open mock missing implementation");
const passthroughOpen = realOpen;
const wrapOpen = async (...args: Parameters<typeof fsp.open>) => {
  const fh = await passthroughOpen(...args);
  const origRead = fh.read.bind(fh);
  fh.read = ((buffer: Buffer, offset: number, length: number, position: number | null) => {
    readCalls.push({ length, position });
    return origRead(buffer, offset, length, position);
  }) as typeof fh.read;
  return fh;
};
openMock.mockImplementation(wrapOpen);

// worker_threads を mock して batch 送信・in-flight 数・terminate を計測する。
// batch protocol（in-flight ≤1・batch 内 0-indexed matched）の構造検証用。
const batchLog: Array<{ display: string; startLine: number; lines: number; bytes: number }> = [];
let inFlight = 0;
let maxInFlight = 0;
let terminateCount = 0;
vi.mock("node:worker_threads", async (importOriginal) => {
  const orig = await importOriginal<typeof import("node:worker_threads")>();
  class SpyWorker extends orig.Worker {
    constructor(...args: ConstructorParameters<typeof orig.Worker>) {
      super(...args);
      this.on("message", (msg: { type?: unknown }) => {
        if (msg !== null && typeof msg === "object" && msg.type === "result") inFlight--;
      });
    }
    override postMessage(value: unknown): void {
      const v = value as {
        type?: string;
        display?: string;
        startLine?: number;
        lines?: string[];
      };
      if (v !== null && typeof v === "object" && v.type === "batch") {
        inFlight++;
        maxInFlight = Math.max(maxInFlight, inFlight);
        const lines = v.lines ?? [];
        batchLog.push({
          display: String(v.display),
          startLine: Number(v.startLine),
          lines: lines.length,
          bytes: lines.reduce((a, l) => a + Buffer.byteLength(l, "utf8"), 0),
        });
      }
      super.postMessage(value);
    }
    override terminate(): Promise<number> {
      terminateCount++;
      return super.terminate();
    }
  }
  return { ...orig, Worker: SpyWorker };
});

// dir/root: テストごとの小さな fixture 用（beforeEach で作り直す）。
// bigDir/bigRoot: 巨大 fixture 用（beforeAll で 1 回だけ生成）。
let dir: string;
let root: string;
let bigDir: string;
let bigRoot: string;

const BIG_LINES = 200_000;
let bigNeedleLine = 0;
let hugeNeedleLine = 0;
let hugeNeedleText = "";

beforeAll(async () => {
  bigDir = await mkdtemp(path.join(tmpdir(), "cheaper-grep-big-"));
  bigRoot = await realpath(bigDir);
  // 16MB 超の複数行 fixture。後半に既知一致を置く。
  const lines: string[] = [];
  for (let i = 1; i <= BIG_LINES; i++) {
    lines.push(`line-${String(i).padStart(7, "0")} ${"x".repeat(80)}`);
  }
  bigNeedleLine = BIG_LINES - 10;
  lines[bigNeedleLine - 1] = `line-${String(bigNeedleLine).padStart(7, "0")} UNIQUE_NEEDLE_16MB`;
  const big = `${lines.join("\n")}\n`;
  expect(Buffer.byteLength(big, "utf8")).toBeGreaterThan(16 * 1024 * 1024);
  await writeFile(path.join(bigRoot, "big.txt"), big, "utf8");
  // 100MB 超の fixture（64 byte 行を敷き詰め）。後半に既知一致を置く。
  const hugeLines = 1_700_000;
  const pad = `${"y".repeat(63)}\n`;
  expect(Buffer.byteLength(pad, "utf8")).toBe(64);
  const hugeBuf = Buffer.alloc(64 * hugeLines);
  for (let i = 0; i < hugeLines; i++) hugeBuf.write(pad, i * 64, "utf8");
  hugeNeedleLine = hugeLines - 100;
  hugeNeedleText = `UNIQUE_NEEDLE_100MB${"y".repeat(64 - 18 - 1)}`;
  hugeBuf.write(`${hugeNeedleText}\n`, (hugeNeedleLine - 1) * 64, "utf8");
  await writeFile(path.join(bigRoot, "huge.txt"), hugeBuf);
  expect(hugeBuf.length).toBeGreaterThan(100 * 1024 * 1024);
}, 180_000);

afterAll(async () => {
  await rm(bigDir, { recursive: true, force: true });
});

beforeEach(async () => {
  dir = await mkdtemp(path.join(tmpdir(), "cheaper-grep-"));
  root = await realpath(dir);
  readCalls.length = 0;
  batchLog.length = 0;
  inFlight = 0;
  maxInFlight = 0;
  terminateCount = 0;
  vi.mocked(fsp.readFile).mockClear();
});
afterEach(async () => {
  vi.restoreAllMocks();
  // module mock の実装は restore されないため明示的に戻す
  openMock.mockImplementation(wrapOpen);
  await rm(dir, { recursive: true, force: true });
});

function tool(opts?: GrepToolOptions) {
  return createGrepTool({ root, symlinkPolicy: "deny", ...opts });
}

function bigTool(opts?: GrepToolOptions) {
  return createGrepTool({ root: bigRoot, symlinkPolicy: "deny", ...opts });
}

async function run(
  t: ReturnType<typeof tool>,
  params: Record<string, unknown>,
  signal?: AbortSignal,
): Promise<string> {
  const result = await t.execute("call-1", params as never, signal, undefined, undefined as never);
  return result.content
    .filter((c): c is { type: "text"; text: string } => c.type === "text")
    .map((c) => c.text)
    .join("\n");
}

describe("grep tool: parity", () => {
  it("finds matches as display:line: text", async () => {
    await writeFile(path.join(root, "a.txt"), "foo\nbar\nfoo bar\n");
    const out = await run(tool(), { pattern: "foo" });
    expect(out).toBe("a.txt:1: foo\na.txt:3: foo bar");
  });

  it("honors ignoreCase", async () => {
    await writeFile(path.join(root, "a.txt"), "Foo\nfOO\n");
    const out = await run(tool(), { pattern: "foo", ignoreCase: true });
    expect(out).toBe("a.txt:1: Foo\na.txt:2: fOO");
  });

  it("filters by glob", async () => {
    await writeFile(path.join(root, "a.txt"), "hit\n");
    await writeFile(path.join(root, "a.md"), "hit\n");
    const out = await run(tool(), { pattern: "hit", glob: "*.txt" });
    expect(out).toBe("a.txt:1: hit");
  });

  it("searches a subdirectory via path param", async () => {
    await mkdir(path.join(root, "sub"));
    await writeFile(path.join(root, "sub", "s.txt"), "needle\n");
    await writeFile(path.join(root, "top.txt"), "needle\n");
    const out = await run(tool(), { pattern: "needle", path: "sub" });
    expect(out).toBe("sub/s.txt:1: needle");
  });

  it("caps results at maxResults", async () => {
    const lines = Array.from({ length: 10 }, (_, i) => `m${i}`).join("\n");
    await writeFile(path.join(root, "many.txt"), `${lines}\n`);
    const out = await run(tool(), { pattern: "m\\d", maxResults: 3 });
    expect(out).toBe("many.txt:1: m0\nmany.txt:2: m1\nmany.txt:3: m2\n[truncated: more matches]");
  });

  it("emits context lines around matches", async () => {
    await writeFile(path.join(root, "c.txt"), "l1\nl2\nhit\nl4\nl5\n");
    const out = await run(tool(), { pattern: "hit", context: 1 });
    expect(out).toBe("c.txt-2- l2\nc.txt:3: hit\nc.txt-4- l4");
  });

  it("emits -- between disjoint context blocks", async () => {
    const body = Array.from({ length: 12 }, (_, i) => (i === 2 || i === 9 ? "hit" : `l${i}`)).join(
      "\n",
    );
    await writeFile(path.join(root, "sep.txt"), `${body}\n`);
    const out = await run(tool(), { pattern: "hit", context: 1 });
    expect(out).toBe(
      "sep.txt-2- l1\nsep.txt:3: hit\nsep.txt-4- l3\n--\nsep.txt-9- l8\nsep.txt:10: hit\nsep.txt-11- l10",
    );
  });

  it("keeps CR verbatim for CRLF files", async () => {
    await writeFile(path.join(root, "crlf.txt"), "a\r\nhit\r\n", "utf8");
    const out = await run(tool(), { pattern: "hit" });
    expect(out).toBe("crlf.txt:2: hit\r");
  });

  it("skips node_modules", async () => {
    await mkdir(path.join(root, "node_modules"));
    await writeFile(path.join(root, "node_modules", "n.txt"), "hit\n");
    await writeFile(path.join(root, "t.txt"), "hit\n");
    const out = await run(tool(), { pattern: "hit" });
    expect(out).toBe("t.txt:1: hit");
  });

  it('matches ^$ on line 1 of an empty file (split("\\n") parity)', async () => {
    await writeFile(path.join(root, "empty.txt"), "");
    const out = await run(tool(), { pattern: "^$" });
    expect(out).toBe("empty.txt:1: ");
  });

  it('matches ^$ on the trailing empty line of a newline-terminated file (split("\\n") parity)', async () => {
    await writeFile(path.join(root, "a.txt"), "a\n");
    const out = await run(tool(), { pattern: "^$" });
    expect(out).toBe("a.txt:2: ");
  });

  it("does not produce a trailing empty line for a file without a final newline", async () => {
    await writeFile(path.join(root, "a.txt"), "a");
    const out = await run(tool(), { pattern: "^$" });
    expect(out).toBe("(no matches)");
  });

  it("returns a no-matches marker when nothing matched", async () => {
    await writeFile(path.join(root, "a.txt"), "zzz\n");
    const out = await run(tool(), { pattern: "qqq" });
    expect(out).toBe("(no matches)");
  });

  it("rejects invalid regex patterns", async () => {
    await expect(run(tool(), { pattern: "(((" })).rejects.toThrow(/invalid grep pattern/i);
  });

  it("rejects a pattern longer than GREP_PATTERN_MAX_CHARS", async () => {
    await expect(run(tool(), { pattern: "a".repeat(GREP_PATTERN_MAX_CHARS + 1) })).rejects.toThrow(
      /invalid grep pattern/,
    );
  });

  it("denies path outside the workspace", async () => {
    await expect(run(tool(), { pattern: "x", path: ".." })).rejects.toThrow(
      "Access denied: path is outside the workspace",
    );
  });

  it("never spawns an external process", async () => {
    const spy = vi.spyOn(childProcess, "spawn");
    await writeFile(path.join(root, "a.txt"), "hit\n");
    await run(tool(), { pattern: "hit" });
    expect(spy).not.toHaveBeenCalled();
  });
});

describe("grep tool: streaming / batch protocol", () => {
  it("finds a known match in the tail of a >1MB file", async () => {
    const out = await run(bigTool(), { pattern: "UNIQUE_NEEDLE_16MB", glob: "big.txt" });
    expect(out).toBe(
      `big.txt:${bigNeedleLine}: line-${String(bigNeedleLine).padStart(7, "0")} UNIQUE_NEEDLE_16MB`,
    );
  });

  it("finds a known match in the tail of a >100MB file", async () => {
    const out = await run(bigTool(), { pattern: "UNIQUE_NEEDLE_100MB", glob: "huge.txt" });
    expect(out).toBe(`huge.txt:${hugeNeedleLine}: ${hugeNeedleText}`);
  }, 120_000);

  it("matches lines spanning chunk and batch boundaries", async () => {
    // chunkBytes=5 で行をまたいで分割し、batchMaxLines=1 で全行を別 batch にする。
    await writeFile(path.join(root, "x.txt"), "aaaaaaaa\nhit-target\nzz\n", "utf8");
    const out = await run(tool({ chunkBytes: 5, batchMaxLines: 1 }), {
      pattern: "hit-target",
      glob: "x.txt",
    });
    expect(out).toBe("x.txt:2: hit-target");
    expect(batchLog.length).toBeGreaterThanOrEqual(3);
  });

  it("merges context blocks across batch boundaries", async () => {
    // batchMaxLines=3: batches [1-3],[4-6],[7]。match 2/4/6 の context が batch を
    // またいで 1 block に merge される。
    await writeFile(path.join(root, "m.txt"), "l1\nhit\nl3\nhit\nl5\nhit\nl7\n", "utf8");
    const out = await run(tool({ batchMaxLines: 3 }), {
      pattern: "hit",
      context: 1,
      glob: "m.txt",
    });
    expect(out).toBe(
      "m.txt-1- l1\nm.txt:2: hit\nm.txt-3- l3\nm.txt:4: hit\nm.txt-5- l5\nm.txt:6: hit\nm.txt-7- l7",
    );
  });

  it("keeps at most one unacknowledged batch in flight", async () => {
    const lines = Array.from({ length: 40 }, (_, i) => `l${i}`).join("\n");
    await writeFile(path.join(root, "if.txt"), `${lines}\n`);
    const out = await run(tool({ batchMaxLines: 2 }), { pattern: "l3", glob: "if.txt" });
    expect(out).toContain("if.txt:");
    expect(batchLog.length).toBeGreaterThanOrEqual(10);
    expect(maxInFlight).toBeLessThanOrEqual(1);
    expect(inFlight).toBe(0);
  });

  it("closes a batch at an oversized line so batch indexes map to absolute line numbers", async () => {
    // 行 5 が oversized → batch [1-4] と batch [6-] に分かれ、startLine=6 で送られる。
    await writeFile(
      path.join(root, "o.txt"),
      "l1\nl2\nl3\nl4\nOVERSIZED-LINE-CONTENT\nhit6\n",
      "utf8",
    );
    const out = await run(tool({ maxLineBytes: 10, batchMaxLines: 100 }), {
      pattern: "hit6",
      glob: "o.txt",
    });
    expect(out).toBe("o.txt:6: hit6\n[skipped: oversized-line — o.txt:5]");
    const starts = batchLog.map((b) => b.startLine);
    expect(starts).toEqual([1, 6]);
  });

  it("closes a batch before adding a line that would exceed batchMaxBytes", async () => {
    // batchMaxBytes=10: "aaa"+"bbb"+"cccc" = ちょうど 10 byte で 1 batch。
    // "ddd" を足すと 13 > 10 なので次 batch に締める（B は hard cap）。
    await writeFile(path.join(root, "b.txt"), "aaa\nbbb\ncccc\nddd\n", "utf8");
    const out = await run(tool({ batchMaxBytes: 10, batchMaxLines: 100 }), {
      pattern: "ddd",
      glob: "b.txt",
    });
    expect(out).toBe("b.txt:4: ddd");
    expect(batchLog.length).toBeGreaterThanOrEqual(2);
    for (const b of batchLog) {
      expect(b.bytes).toBeLessThanOrEqual(10);
    }
    expect(batchLog.map((b) => b.startLine)).toEqual([1, 4]);
  });

  it("still sends a single line whose payload exceeds batchMaxBytes as a one-line batch", async () => {
    // 縮退注入でのみ到達する経路: 1 行 payload > B でも 1 行 batch で送り、
    // 進行が保証される（実 limit では L が decoded payload を B 未満に抑える）。
    await writeFile(path.join(root, "big.txt"), "0123456789\nhit\n", "utf8");
    const out = await run(tool({ batchMaxBytes: 4, maxLineBytes: 100 }), {
      pattern: "hit",
      glob: "big.txt",
    });
    expect(out).toBe("big.txt:2: hit");
    expect(batchLog[0]?.lines).toBe(1);
    expect(batchLog[0]?.bytes).toBe(10); // B=4 超過だが単独 batch で送られる
  });
});

describe("grep tool: skip footer", () => {
  it("reports binary files in the footer", async () => {
    const bin = Buffer.from([0x68, 0x69, 0x00, 0x74]);
    await writeFile(path.join(root, "b.dat"), bin);
    await writeFile(path.join(root, "t.txt"), "hit\n");
    const out = await run(tool(), { pattern: "hit" });
    expect(out).toBe("t.txt:1: hit\n[skipped: binary — b.dat]");
  });

  it("reports oversized lines as path:line in the footer", async () => {
    await writeFile(path.join(root, "o.txt"), "ok\nthis-is-a-very-long-line\nhit\n", "utf8");
    const out = await run(tool({ maxLineBytes: 10 }), { pattern: "hit", glob: "o.txt" });
    // 行 2 は破棄されるが行番号は消費されるため hit は行 3 のまま。
    expect(out).toBe("o.txt:3: hit\n[skipped: oversized-line — o.txt:2]");
  });

  it("distinguishes oversized-line skips from (no matches)", async () => {
    await writeFile(path.join(root, "o.txt"), "a-very-very-long-line\n", "utf8");
    const out = await run(tool({ maxLineBytes: 10 }), { pattern: "hit", glob: "o.txt" });
    expect(out).toBe("(no matches)\n[skipped: oversized-line — o.txt:1]");
  });

  it("reports enumeration failures as unreadable in the footer", async (ctx) => {
    try {
      await symlink(path.join(root, "no-such-target"), path.join(root, "dangling"), "junction");
    } catch (e) {
      console.warn(`symlink unavailable, skipping: ${String(e)}`);
      ctx.skip();
    }
    await writeFile(path.join(root, "ok.txt"), "hit\n");
    const t = createGrepTool({ root, symlinkPolicy: "within-workspace" });
    const out = await run(t, { pattern: "hit" });
    expect(out).toBe("ok.txt:1: hit\n[skipped: unreadable — dangling]");
  });

  it("reports files that fail to open as unreadable", async () => {
    await writeFile(path.join(root, "locked.txt"), "hit\n");
    await writeFile(path.join(root, "ok.txt"), "hit\n");
    openMock.mockImplementation(async (...args: Parameters<typeof fsp.open>) => {
      if (String(args[0]).endsWith("locked.txt")) throw new Error("EACCES");
      return wrapOpen(...args);
    });
    const out = await run(tool(), { pattern: "hit" });
    expect(out).toBe("ok.txt:1: hit\n[skipped: unreadable — locked.txt]");
  });

  it("bounds representative entries and counts the rest as 'and N more'", async () => {
    for (let i = 0; i < 25; i++) {
      const bin = Buffer.from([0x61, 0x00, 0x62]);
      await writeFile(path.join(root, `b${String(i).padStart(2, "0")}.dat`), bin);
    }
    const out = await run(tool(), { pattern: "hit" });
    expect(out).toContain("[skipped: binary — ");
    expect(out).toContain("and 5 more");
    // 代表 entry は 20 件まで
    const shown = out.match(/b\d\d\.dat/g) ?? [];
    expect(shown.length).toBeLessThanOrEqual(20);
  });

  it("truncates long paths at skipPathMaxChars", async () => {
    const longName = `${"d".repeat(200)}.dat`;
    await writeFile(path.join(root, longName), Buffer.from([0x61, 0x00]));
    const out = await run(tool({ skipPathMaxChars: 20 }), { pattern: "hit" });
    expect(out).toBe(`(no matches)\n[skipped: binary — ${"d".repeat(20)}]`);
  });

  it("emits [skipped-report: truncated] when the footer exceeds the byte cap", async () => {
    for (let i = 0; i < 4; i++) {
      await writeFile(path.join(root, `long-name-${"n".repeat(40)}-${i}.dat`), Buffer.from([0x00]));
    }
    const out = await run(tool({ skipFooterMaxBytes: 40 }), { pattern: "hit" });
    expect(out).toContain("[skipped-report: truncated]");
    expect(out).not.toContain("[skipped: binary");
  });
});

describe("grep tool: eval budgets and deadline", () => {
  // 決定的にするため時間源を注入する。now() 呼出ごとに 10ms 進めると
  // 1 batch の eval 計測（t0→dt で 4 呼出）は 40ms と計上される。
  function fakeClock() {
    let t = 0;
    return () => (t += 10);
  }

  it("accumulates the per-file eval budget across batches, skips the file as timeout, and recreates the worker for the next file", async () => {
    // batchMaxLines=3 × 300 行 → ~100 batch、1 batch あたり 40ms 計上。
    // perFileMs=50 は 2 batch で累積超過する（batch ごとのリセットなら超えない）。
    const lines = Array.from({ length: 300 }, (_, i) => `l${i}`).join("\n");
    await writeFile(path.join(root, "a.txt"), `${lines}\n`);
    await writeFile(path.join(root, "b.txt"), "hit\n");
    const t = tool({ batchMaxLines: 3, evalPerFileMs: 50, now: fakeClock() });
    const out = await run(t, { pattern: "hit" });
    expect(out).toContain("b.txt:1: hit");
    expect(out).toContain("[skipped: timeout — a.txt]");
    // worker が terminate されて後続 file 用に再生成されている
    expect(terminateCount).toBeGreaterThanOrEqual(2);
  });

  it("accumulates the total eval budget and stops the scan", async () => {
    // opendir の列挙順は FS 依存なので、両 file を同形にして列挙順に依存しない
    // assertion にする。先に走査された file が total 予算を使い切り、走査全体が
    // 止まるため、もう一方の file は一切報告されない。
    const lines = Array.from({ length: 300 }, (_, i) => `l${i}`).join("\n");
    await writeFile(path.join(root, "a.txt"), `hit\n${lines}\n`);
    await writeFile(path.join(root, "b.txt"), `hit\n${lines}\n`);
    const t = tool({ batchMaxLines: 3, evalTotalMs: 80, now: fakeClock() });
    const out = await run(t, { pattern: "hit" });
    expect(out).toContain("[truncated: evaluation time limit reached]");
    // 先に走査された 1 file だけが match と timeout skip に現れる
    const scanned = ["a.txt", "b.txt"].filter((name) => out.includes(name));
    expect(scanned).toHaveLength(1);
    expect(out).toContain(`${scanned[0]}:1: hit`);
    expect(out).toContain(`[skipped: timeout — ${scanned[0]}]`);
  });

  it("stops at the scan deadline and returns partial results with a footer", async () => {
    await writeFile(path.join(root, "a.txt"), "hit\n");
    const t = tool({ scanDeadlineMs: 0 });
    const out = await run(t, { pattern: "hit" });
    expect(out).toContain("[truncated: scan deadline reached]");
    expect(out).not.toContain("a.txt:1: hit");
  });

  it("rejects with Operation aborted on signal abort and cleans up the worker", async () => {
    await writeFile(path.join(root, "a.txt"), "hit\n");
    const ctrl = new AbortController();
    ctrl.abort();
    await expect(run(tool(), { pattern: "hit" }, ctrl.signal)).rejects.toThrow("Operation aborted");
    expect(terminateCount).toBeGreaterThanOrEqual(1);
  });
});

describe("grep tool: output bounds", () => {
  it("truncates a long displayed line at outputLineMaxChars", async () => {
    await writeFile(path.join(root, "long.txt"), `hit${"a".repeat(100)}\n`, "utf8");
    const out = await run(tool({ outputLineMaxChars: 10 }), { pattern: "hit", glob: "long.txt" });
    expect(out).toBe(`long.txt:1: hit${"a".repeat(7)}... [truncated]`);
  });

  it("stops output at outputMaxBytes with a truncation marker", async () => {
    const lines = Array.from({ length: 30 }, (_, i) => `match-line-${i}-xxxxxxxx`).join("\n");
    await writeFile(path.join(root, "o.txt"), `${lines}\n`);
    const out = await run(tool({ outputMaxBytes: 100 }), { pattern: "match", glob: "o.txt" });
    expect(out).toContain("[truncated: output limit reached]");
    const body = out.split("\n[truncated")[0] ?? "";
    expect(Buffer.byteLength(body, "utf8")).toBeLessThanOrEqual(100);
  });
});

describe("grep tool: deny and bounded memory", () => {
  it("hides denied files from results and footer", async () => {
    await writeFile(path.join(root, ".env"), "SECRET=1\n");
    await writeFile(path.join(root, "normal.txt"), "hit\n");
    const t = createGrepTool({
      root,
      symlinkPolicy: "deny",
      denylist: createPathDenylist([]),
    });
    const out = await run(t, { pattern: "SECRET" });
    expect(out).not.toContain(".env");
    expect(out).not.toContain("SECRET=1");
    const out2 = await run(t, { pattern: "hit" });
    expect(out2).toBe("normal.txt:1: hit");
  });

  it("keeps every fh.read <= IO_CHUNK_BYTES and never calls fs.readFile", async () => {
    const out = await run(bigTool(), { pattern: "UNIQUE_NEEDLE_16MB", glob: "big.txt" });
    expect(out).toContain("UNIQUE_NEEDLE_16MB");
    expect(fsp.readFile).not.toHaveBeenCalled();
    expect(readCalls.length).toBeGreaterThan(0);
    for (const c of readCalls) {
      expect(c.length).toBeLessThanOrEqual(IO_CHUNK_BYTES);
    }
  });

  it("keeps batches within limits, in-flight <= 1, and bounded heap on a 100MB file", async () => {
    // 構造的証拠: batch ≤ K 行・in-flight ≤ 1・出力 ≤ O。heapUsed delta は
    // 補助指標として記録する（RSS 一回観測は証拠にしない）。
    const heapBefore = process.memoryUsage().heapUsed;
    const out = await run(bigTool({ batchMaxLines: 500 }), {
      pattern: "UNIQUE_NEEDLE_100MB",
      glob: "huge.txt",
    });
    const heapDelta = process.memoryUsage().heapUsed - heapBefore;
    console.info(`[grep memory] heapUsed delta on 100MB input: ${heapDelta} bytes`);
    expect(out).toContain(`huge.txt:${hugeNeedleLine}`);
    for (const b of batchLog) {
      expect(b.lines).toBeLessThanOrEqual(500);
      expect(b.lines).toBeLessThanOrEqual(GREP_BATCH_MAX_LINES);
    }
    expect(maxInFlight).toBeLessThanOrEqual(1);
    expect(inFlight).toBe(0);
    // 全文保持の回帰検出: 100MB 入力に対し delta は入力に比例しない
    expect(heapDelta).toBeLessThan(150 * 1024 * 1024);
  }, 120_000);
});
