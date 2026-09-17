import * as fsp from "node:fs/promises";
import { mkdir, mkdtemp, realpath, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { createPathDenylist } from "@cheaper/core";
import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { IO_CHUNK_BYTES, READ_MAX_BYTES } from "../../src/tools/limits.js";
import { createReadTool, type ReadToolDetails } from "../../src/tools/read.js";
import { ACCESS_DENIED_MESSAGE, POLICY_DENIED_MESSAGE } from "../../src/tools/walker.js";

// fs/promises を mock して readFile 不呼出・open 経由の fh.read 呼出を計測する。
// open/readFile 以外は importOriginal のまま（writeFile/mkdir 等の fixture 生成を妨げない）。
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
openMock.mockImplementation(async (...args: Parameters<typeof fsp.open>) => {
  const fh = await realOpen(...args);
  const origRead = fh.read.bind(fh);
  fh.read = ((buffer: Buffer, offset: number, length: number, position: number | null) => {
    readCalls.push({ length, position });
    return origRead(buffer, offset, length, position);
  }) as typeof fh.read;
  return fh;
});

let dir: string;
let root: string;

const BIG_LINES = 200_000;
let bigTotalLines = 0;

beforeAll(async () => {
  dir = await mkdtemp(path.join(tmpdir(), "cheaper-read-"));
  root = await realpath(dir);
  // 16MB 超の複数行 fixture（offset 深さ・有界性の検証用）。
  const lines: string[] = [];
  for (let i = 1; i <= BIG_LINES; i++) {
    lines.push(`line-${String(i).padStart(7, "0")} ${"x".repeat(80)}`);
  }
  const big = `${lines.join("\n")}\n`;
  bigTotalLines = lines.length + 1; // 末尾 \n の後の空行を含む split("\n") 意味論
  expect(Buffer.byteLength(big, "utf8")).toBeGreaterThan(16 * 1024 * 1024);
  await writeFile(path.join(root, "big.txt"), big, "utf8");
  // 100MB の改行なし単一行 fixture（byteInLine 継続・有界性の検証用）。
  await writeFile(
    path.join(root, "single-line.bin.txt"),
    "0123456789".repeat(10 * 1024 * 1024),
    "utf8",
  );
}, 120_000);

afterAll(async () => {
  await rm(dir, { recursive: true, force: true });
});

beforeEach(() => {
  readCalls.length = 0;
  vi.mocked(fsp.readFile).mockClear();
});
afterEach(() => {
  vi.mocked(fsp.readFile).mockClear();
});

function tool(opts?: { maxBytes?: number; chunkBytes?: number; denylist?: string[] }) {
  return createReadTool({
    root,
    symlinkPolicy: "deny",
    ...(opts?.denylist !== undefined ? { denylist: createPathDenylist(opts.denylist) } : {}),
    ...(opts?.maxBytes !== undefined ? { maxBytes: opts.maxBytes } : {}),
    ...(opts?.chunkBytes !== undefined ? { chunkBytes: opts.chunkBytes } : {}),
  });
}

async function execute(
  t: ReturnType<typeof createReadTool>,
  p: { path: string; offset?: number; limit?: number; byteInLine?: number },
  signal?: AbortSignal,
) {
  const result = await t.execute("c1", p as never, signal, undefined, undefined as never);
  const texts = result.content.filter(
    (c): c is { type: "text"; text: string } => c.type === "text",
  );
  expect(texts).toHaveLength(1); // 応答はテキスト 1 ブロック
  return { text: texts[0]?.text ?? "", details: result.details as ReadToolDetails };
}

describe("bounded read: 有界性", () => {
  it("never calls fs.readFile and keeps every fh.read <= IO_CHUNK_BYTES", async () => {
    const t = tool();
    await execute(t, { path: "big.txt" });
    expect(fsp.readFile).not.toHaveBeenCalled();
    expect(readCalls.length).toBeGreaterThan(0);
    for (const c of readCalls) {
      expect(c.length).toBeLessThanOrEqual(IO_CHUNK_BYTES);
    }
  });

  it("returns at most READ_MAX_BYTES of content on a 16MB+ file", async () => {
    const t = tool();
    const { text, details } = await execute(t, { path: "big.txt" });
    // 本文（trailer 以前）は R 以下。trailer は protocol metadata で別枠。
    const body = text.split("\n[").at(0) ?? "";
    expect(Buffer.byteLength(body, "utf8")).toBeLessThanOrEqual(READ_MAX_BYTES);
    expect(details.truncatedBy).not.toBeNull();
    expect(details.eof).toBe(false);
    expect(details.continuation).toBeDefined();
  });

  it("returns at most READ_MAX_BYTES on a 100MB single-line file with byteInLine continuation", async () => {
    const t = tool();
    const { text, details } = await execute(t, { path: "single-line.bin.txt" });
    const body = text.split("\n[").at(0) ?? "";
    expect(Buffer.byteLength(body, "utf8")).toBeLessThanOrEqual(READ_MAX_BYTES);
    expect(details.truncatedBy).toBe("bytes");
    expect(details.continuation).toEqual({ offset: 1, byteInLine: READ_MAX_BYTES });
    for (const c of readCalls) {
      expect(c.length).toBeLessThanOrEqual(IO_CHUNK_BYTES);
    }
  });
});

describe("bounded read: offset/limit", () => {
  it("reads from the head by default", async () => {
    const t = tool({ maxBytes: 4096 });
    const { text, details } = await execute(t, { path: "big.txt" });
    expect(text.startsWith("line-0000001 ")).toBe(true);
    expect(details.startLine).toBe(1);
  });

  it("reads from a middle offset", async () => {
    const t = tool({ maxBytes: 4096 });
    const { text, details } = await execute(t, { path: "big.txt", offset: 100_000 });
    expect(text.startsWith("line-0100000 ")).toBe(true);
    expect(details.startLine).toBe(100_000);
    expect(details.eof).toBe(false);
  });

  it("reads the last line with eof and totalLines", async () => {
    const t = tool();
    const { details } = await execute(t, { path: "big.txt", offset: bigTotalLines });
    expect(details.eof).toBe(true);
    expect(details.truncatedBy).toBeNull();
    expect(details.totalLines).toBe(bigTotalLines);
    expect(details.endLine).toBe(bigTotalLines);
  });

  it("emits exactly `limit` lines then reports offset continuation", async () => {
    await writeFile(path.join(root, "five.txt"), "l1\nl2\nl3\nl4\nl5\n", "utf8");
    const t = tool();
    const { text, details } = await execute(t, { path: "five.txt", limit: 3 });
    expect(text).toBe("l1\nl2\nl3\n[Showing lines 1-3. Use offset=4 to continue.]");
    expect(details).toMatchObject({
      startLine: 1,
      endLine: 3,
      eof: false,
      truncatedBy: "limit",
      continuation: { offset: 4 },
    });
  });

  it("reaches eof without a trailer when the limit covers the file", async () => {
    await writeFile(path.join(root, "two.txt"), "a\nb", "utf8");
    const t = tool();
    const { text, details } = await execute(t, { path: "two.txt", limit: 2 });
    expect(text).toBe("a\nb");
    expect(details.eof).toBe(true);
    expect(details.truncatedBy).toBeNull();
    expect(details.totalLines).toBe(2);
  });
});

describe("bounded read: 境界ケース", () => {
  it("returns empty content with eof for an empty file", async () => {
    await writeFile(path.join(root, "empty.txt"), "", "utf8");
    const t = tool();
    const { text, details } = await execute(t, { path: "empty.txt" });
    expect(text).toBe("");
    expect(details.eof).toBe(true);
    expect(details.totalLines).toBe(1);
  });

  it("keeps CR verbatim for CRLF files", async () => {
    await writeFile(path.join(root, "crlf.txt"), "a\r\nb\r\n", "utf8");
    const t = tool();
    const { text, details } = await execute(t, { path: "crlf.txt" });
    expect(text).toBe("a\r\nb\r\n");
    expect(details.eof).toBe(true);
    expect(details.totalLines).toBe(3);
  });

  it("decodes UTF-8 characters split across chunk boundaries", async () => {
    // 3 byte の € が小さな chunk を跨ぐ配置。byte 走査で行を切るため正しく decode される。
    await writeFile(path.join(root, "utf8.txt"), "ab€cd\nxy€z\n", "utf8");
    const t = tool({ chunkBytes: 4 });
    const { text } = await execute(t, { path: "utf8.txt" });
    expect(text).toBe("ab€cd\nxy€z\n");
  });

  it("replaces invalid UTF-8 bytes with U+FFFD", async () => {
    await writeFile(path.join(root, "invalid.bin.txt"), Buffer.from([0x61, 0xff, 0x62]), "utf8");
    const t = tool();
    const { text } = await execute(t, { path: "invalid.bin.txt" });
    expect(text).toBe("a�b");
  });
});

describe("bounded read: byteInLine 継続", () => {
  it("continues an oversized line without loss or duplication", async () => {
    // 確定的な 1000 byte 単一行。maxBytes 縮退で継続を繰り返す。
    const src = "0123456789".repeat(100); // 1000 bytes, no newline
    await writeFile(path.join(root, "long-line.txt"), src, "utf8");
    const t = tool({ maxBytes: 100 });
    let offset = 1;
    let byteInLine: number | undefined;
    let emitted = "";
    for (let i = 0; i < 30; i++) {
      const { text, details } = await execute(t, {
        path: "long-line.txt",
        offset,
        ...(byteInLine !== undefined ? { byteInLine } : {}),
      });
      let body = text;
      const trailerIdx = body.indexOf("\n[");
      if (trailerIdx >= 0) body = body.slice(0, trailerIdx);
      if (byteInLine !== undefined) {
        // partial marker 行を外して本文を連結する
        const nl = body.indexOf("\n");
        body = nl >= 0 ? body.slice(nl + 1) : "";
      }
      emitted += body;
      if (details.eof) break;
      const cont = details.continuation;
      expect(cont).toBeDefined();
      offset = cont?.offset ?? 0;
      byteInLine = cont?.byteInLine;
      if (byteInLine === undefined) throw new Error("expected byteInLine continuation");
    }
    expect(emitted).toBe(src);
  });

  it("emits a partial-line marker when resuming with byteInLine", async () => {
    const src = "0123456789".repeat(100);
    await writeFile(path.join(root, "long-line2.txt"), src, "utf8");
    const t = tool({ maxBytes: 100 });
    const { text } = await execute(t, { path: "long-line2.txt", offset: 1, byteInLine: 100 });
    expect(text.startsWith("[partial line 1: continuing from byte 100]\n")).toBe(true);
  });

  it("snaps byteInLine forward to the next UTF-8 boundary", async () => {
    // "€rest\n": € は 3 byte。byteInLine=1（€ 途中）→ 次の境界 byte 3 へ前進。
    await writeFile(path.join(root, "snap.txt"), "€rest\n", "utf8");
    const t = tool();
    const { text } = await execute(t, { path: "snap.txt", offset: 1, byteInLine: 1 });
    expect(text.startsWith("[partial line 1: continuing from byte 3]\nrest")).toBe(true);
  });

  it("reports mid-line byte cap with byteInLine continuation", async () => {
    await writeFile(path.join(root, "cap.txt"), "aaaa\nbbbb\n", "utf8");
    const t = tool({ maxBytes: 6 });
    const { text, details } = await execute(t, { path: "cap.txt" });
    expect(text).toBe(
      "aaaa\nb\n[Line 2 emitted bytes 0-1 (reached 6B limit). Use offset=2 byteInLine=1 to continue.]",
    );
    expect(details.truncatedBy).toBe("bytes");
    expect(details.continuation).toEqual({ offset: 2, byteInLine: 1 });
  });

  it("preserves byteInLine in the continuation when the cap hits at the resume point", async () => {
    // partial marker (42B) が予算を使い切り、行 byte を 1 byte も emit せず
    // cap に到達するケース。再開時に [0, emitBeginInLine) を再 emit しないよう
    // continuation は byteInLine を保持しなければならない。
    const src = "0123456789".repeat(100); // 1000 bytes, no newline
    await writeFile(path.join(root, "cap-resume.txt"), src, "utf8");
    const t = tool({ maxBytes: 42 });
    const { text, details } = await execute(t, {
      path: "cap-resume.txt",
      offset: 1,
      byteInLine: 100,
    });
    expect(text).toBe(
      "[partial line 1: continuing from byte 100]\n" +
        "[Line 1 emitted bytes 100-100 (reached 42B limit). Use offset=1 byteInLine=100 to continue.]",
    );
    expect(details.truncatedBy).toBe("bytes");
    expect(details.continuation).toEqual({ offset: 1, byteInLine: 100 });
  });

  it("reports line-boundary byte cap with offset continuation", async () => {
    await writeFile(path.join(root, "cap2.txt"), "aaaa\nbbbb\n", "utf8");
    const t = tool({ maxBytes: 5 });
    const { text, details } = await execute(t, { path: "cap2.txt" });
    expect(text).toBe("aaaa\n[Reached 5B limit. Use offset=2 to continue.]");
    expect(details.truncatedBy).toBe("bytes");
    expect(details.continuation).toEqual({ offset: 2 });
  });

  it("uses the documented 50KB trailer text at the default cap", async () => {
    const t = tool();
    const { text } = await execute(t, { path: "single-line.bin.txt" });
    expect(text).toContain(
      `[Line 1 emitted bytes 0-${READ_MAX_BYTES} (reached 50KB limit). Use offset=1 byteInLine=${READ_MAX_BYTES} to continue.]`,
    );
  });
});

describe("bounded read: 明示 error", () => {
  it("rejects byteInLine without offset", async () => {
    const t = tool();
    await expect(execute(t, { path: "big.txt", byteInLine: 5 })).rejects.toThrow(
      "byteInLine requires offset",
    );
  });

  it("rejects an offset beyond EOF with the total line count", async () => {
    await writeFile(path.join(root, "three.txt"), "l1\nl2\nl3\n", "utf8");
    const t = tool();
    await expect(execute(t, { path: "three.txt", offset: 99 })).rejects.toThrow(
      "Offset 99 is beyond end of file (4 lines total)",
    );
  });

  it("rejects byteInLine beyond the line byte length", async () => {
    await writeFile(path.join(root, "short.txt"), "abc\ndef\n", "utf8");
    const t = tool();
    await expect(execute(t, { path: "short.txt", offset: 1, byteInLine: 100 })).rejects.toThrow(
      "byteInLine 100 is beyond end of line 1 (3 bytes)",
    );
    await expect(execute(t, { path: "short.txt", offset: 1, byteInLine: 3 })).rejects.toThrow(
      "byteInLine 3 is beyond end of line 1 (3 bytes)",
    );
  });

  it("rejects binary files on NUL sniff", async () => {
    await writeFile(path.join(root, "bin.dat"), Buffer.from([0x41, 0x00, 0x42]), "utf8");
    const t = tool();
    await expect(execute(t, { path: "bin.dat" })).rejects.toThrow(
      "read: binary file (NUL byte within first 8KiB)",
    );
  });

  it("rejects image extensions before scanning", async () => {
    await writeFile(path.join(root, "img.png"), "not a real png", "utf8");
    await writeFile(path.join(root, "img.JPG"), "x", "utf8");
    const t = tool();
    await expect(execute(t, { path: "img.png" })).rejects.toThrow(
      "read does not support image files",
    );
    await expect(execute(t, { path: "img.JPG" })).rejects.toThrow(
      "read does not support image files",
    );
  });

  it("rejects non-regular files", async () => {
    await mkdir(path.join(root, "adir"), { recursive: true });
    const t = tool();
    await expect(execute(t, { path: "adir" })).rejects.toThrow("read: not a regular file");
    await expect(execute(t, { path: "." })).rejects.toThrow("read: not a regular file");
  });
});

describe("bounded read: boundary/deny", () => {
  it("denies workspace escapes", async () => {
    const t = tool();
    await expect(execute(t, { path: "../outside.txt" })).rejects.toThrow(ACCESS_DENIED_MESSAGE);
    await expect(execute(t, { path: path.join(tmpdir(), "abs-outside.txt") })).rejects.toThrow(
      ACCESS_DENIED_MESSAGE,
    );
  });

  it("denies symlink escapes under deny policy", async (ctx) => {
    const outside = await mkdtemp(path.join(tmpdir(), "cheaper-read-out-"));
    try {
      await writeFile(path.join(outside, "secret.txt"), "s", "utf8");
      const link = path.join(root, "link-out.txt");
      try {
        await symlink(path.join(outside, "secret.txt"), link);
      } catch (e) {
        console.warn(`symlink unavailable, skipping: ${String(e)}`);
        ctx.skip();
      }
      const t = tool();
      await expect(execute(t, { path: "link-out.txt" })).rejects.toThrow(ACCESS_DENIED_MESSAGE);
    } finally {
      await rm(outside, { recursive: true, force: true });
    }
  });

  it("denies built-in and custom deny paths", async () => {
    await writeFile(path.join(root, ".env"), "SECRET=1\n", "utf8");
    await writeFile(path.join(root, "normal.txt"), "hi", "utf8");
    const t = tool({ denylist: ["normal.txt"] });
    await expect(execute(t, { path: ".env" })).rejects.toThrow(POLICY_DENIED_MESSAGE);
    await expect(execute(t, { path: "normal.txt" })).rejects.toThrow(POLICY_DENIED_MESSAGE);
  });
});

describe("bounded read: abort", () => {
  it("rejects with Operation aborted when the signal is already aborted", async () => {
    const t = tool();
    const ctrl = new AbortController();
    ctrl.abort();
    await expect(execute(t, { path: "big.txt" }, ctrl.signal)).rejects.toThrow("Operation aborted");
  });
});
