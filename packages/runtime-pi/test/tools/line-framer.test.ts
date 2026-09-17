import { describe, expect, it } from "vitest";
import { createLineFramer } from "../../src/tools/line-framer.js";

interface Emitted {
  line: string;
  lineNo: number;
  rawBytes: number;
}

function collect(maxLineBytes: number) {
  const lines: Emitted[] = [];
  const oversized: number[] = [];
  const framer = createLineFramer({
    maxLineBytes,
    onLine: (line, lineNo, rawBytes) => lines.push({ line, lineNo, rawBytes }),
    onOversized: (lineNo) => oversized.push(lineNo),
  });
  return { framer, lines, oversized };
}

describe("createLineFramer", () => {
  it("emits lines split on raw 0x0A bytes with 1-indexed line numbers", () => {
    const { framer, lines, oversized } = collect(1024);
    framer.feed(Buffer.from("a\nbb\nccc\n"));
    framer.flush();
    expect(lines).toEqual([
      { line: "a", lineNo: 1, rawBytes: 1 },
      { line: "bb", lineNo: 2, rawBytes: 2 },
      { line: "ccc", lineNo: 3, rawBytes: 3 },
      { line: "", lineNo: 4, rawBytes: 0 }, // LF 終端の末尾空行（split parity）
    ]);
    expect(oversized).toEqual([]);
  });

  it("reassembles a line split across chunk boundaries", () => {
    const { framer, lines } = collect(1024);
    framer.feed(Buffer.from("hel"));
    framer.feed(Buffer.from("lo w"));
    framer.feed(Buffer.from("orld\nnex"));
    framer.feed(Buffer.from("t\n"));
    framer.flush();
    expect(lines.map((l) => l.line)).toEqual(["hello world", "next", ""]);
    expect(lines.map((l) => l.lineNo)).toEqual([1, 2, 3]);
  });

  it("keeps multibyte UTF-8 split across chunks intact (rawBytes stays byte-based)", () => {
    const { framer, lines } = collect(1024);
    const buf = Buffer.from("あいう\n"); // 9 byte + LF
    framer.feed(buf.subarray(0, 4)); // multibyte sequence の途中で分割
    framer.feed(buf.subarray(4));
    framer.flush();
    expect(lines).toEqual([
      { line: "あいう", lineNo: 1, rawBytes: 9 },
      { line: "", lineNo: 2, rawBytes: 0 }, // LF 終端の末尾空行（split parity）
    ]);
  });

  it("discards a line over maxLineBytes and emits onOversized once", () => {
    const { framer, lines, oversized } = collect(4);
    framer.feed(Buffer.from("ok\n"));
    framer.feed(Buffer.from("this line is way too long\n"));
    framer.feed(Buffer.from("tail\n"));
    framer.flush();
    expect(lines.map((l) => l.line)).toEqual(["ok", "tail", ""]);
    expect(oversized).toEqual([2]);
  });

  it("consumes the oversized line number so later lines keep their numbering", () => {
    const { framer, lines, oversized } = collect(4);
    framer.feed(Buffer.from("a\n"));
    framer.feed(Buffer.from("xxxxx\n")); // line 2 oversized
    framer.feed(Buffer.from("yy\n"));
    framer.flush();
    expect(lines.map((l) => l.lineNo)).toEqual([1, 3, 4]); // 末尾空行（split parity）
    expect(oversized).toEqual([2]);
  });

  it("detects overflow mid-chunk and resumes at the next newline", () => {
    const { framer, lines, oversized } = collect(4);
    framer.feed(Buffer.from("abc"));
    framer.feed(Buffer.from("defgh")); // pendingBytes 3 + 5 > 4 → oversized、\n まで破棄
    framer.feed(Buffer.from("ij\nkl\n"));
    framer.flush();
    expect(lines).toEqual([
      { line: "kl", lineNo: 2, rawBytes: 2 },
      { line: "", lineNo: 3, rawBytes: 0 }, // LF 終端の末尾空行（split parity）
    ]);
    expect(oversized).toEqual([1]);
  });

  it("emits the unterminated trailing line on flush", () => {
    const { framer, lines } = collect(1024);
    framer.feed(Buffer.from("a\nlast"));
    framer.flush();
    expect(lines.map((l) => l.line)).toEqual(["a", "last"]);
    expect(lines.map((l) => l.lineNo)).toEqual([1, 2]);
  });

  it('emits a trailing empty line on flush when input ends with a newline (split("\\n") parity)', () => {
    const { framer, lines } = collect(1024);
    framer.feed(Buffer.from("a\n"));
    framer.flush();
    // "a\n".split("\n") === ["a", ""]
    expect(lines.map((l) => l.line)).toEqual(["a", ""]);
    expect(lines.map((l) => l.lineNo)).toEqual([1, 2]);
  });

  it('emits a single empty line on flush for empty input (split("\\n") parity)', () => {
    const { framer, lines } = collect(1024);
    framer.flush();
    // "".split("\n") === [""]
    expect(lines).toEqual([{ line: "", lineNo: 1, rawBytes: 0 }]);
  });

  it('emits every trailing empty line for consecutive newlines (split("\\n") parity)', () => {
    const { framer, lines } = collect(1024);
    framer.feed(Buffer.from("a\n\n"));
    framer.flush();
    // "a\n\n".split("\n") === ["a", "", ""]
    expect(lines.map((l) => l.line)).toEqual(["a", "", ""]);
    expect(lines.map((l) => l.lineNo)).toEqual([1, 2, 3]);
  });

  it("emits no extra empty line when input ends without a newline", () => {
    const { framer, lines } = collect(1024);
    framer.feed(Buffer.from("a"));
    framer.flush();
    expect(lines.map((l) => l.line)).toEqual(["a"]);
  });

  it("does not emit a trailing empty line when an oversized line is discarded at EOF", () => {
    const { framer, lines, oversized } = collect(4);
    framer.feed(Buffer.from("a\n"));
    framer.feed(Buffer.from("discarded-long-line"));
    framer.flush();
    expect(lines.map((l) => l.line)).toEqual(["a"]);
    expect(oversized).toEqual([2]);
  });

  it("reports an oversized unterminated line via onOversized only at flush", () => {
    const { framer, lines, oversized } = collect(4);
    framer.feed(Buffer.from("a very long line without newline"));
    framer.flush();
    expect(lines).toEqual([]);
    expect(oversized).toEqual([1]);
  });

  it('keeps \\r in line content (split("\\n") parity)', () => {
    const { framer, lines } = collect(1024);
    framer.feed(Buffer.from("a\r\nb\r\n"));
    framer.flush();
    expect(lines.map((l) => l.line)).toEqual(["a\r", "b\r", ""]);
  });

  it("replaces invalid UTF-8 bytes with U+FFFD", () => {
    const { framer, lines } = collect(1024);
    framer.feed(Buffer.from([0x61, 0xff, 0x62, 0x0a]));
    framer.flush();
    expect(lines).toEqual([
      { line: "a�b", lineNo: 1, rawBytes: 3 },
      { line: "", lineNo: 2, rawBytes: 0 }, // LF 終端の末尾空行（split parity）
    ]);
  });
});
