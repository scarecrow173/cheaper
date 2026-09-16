import type { CheaperError } from "@cheaper/core";
import { describe, expect, it } from "vitest";
import { parseCliArgs } from "../src/args.js";

function codeOf(fn: () => unknown): string {
  try {
    fn();
  } catch (e) {
    return (e as CheaperError).code;
  }
  throw new Error("did not throw");
}

describe("parseCliArgs", () => {
  it("parses `run <worker>`", () => {
    const r = parseCliArgs(["run", "bulk-reader"]);
    expect(r).toMatchObject({ kind: "run", workerName: "bulk-reader", format: "json" });
  });

  it("maps bulk-read / code-write / doc-write aliases to fixed worker names", () => {
    expect(parseCliArgs(["bulk-read"])).toMatchObject({
      kind: "run",
      workerName: "bulk-reader",
    });
    expect(parseCliArgs(["code-write"])).toMatchObject({
      kind: "run",
      workerName: "code-writer",
    });
    expect(parseCliArgs(["doc-write"])).toMatchObject({
      kind: "run",
      workerName: "doc-writer",
    });
  });

  it("rejects the removed cheap-write command", () => {
    expect(codeOf(() => parseCliArgs(["cheap-write"]))).toBe("INVALID_REQUEST");
  });

  it("accepts --worker-file on run without a worker name", () => {
    const r = parseCliArgs(["run", "--worker-file", "./w.md"]);
    expect(r).toMatchObject({ kind: "run", workerFile: "./w.md", workerName: undefined });
  });

  it("rejects --worker-file combined with a worker name", () => {
    expect(codeOf(() => parseCliArgs(["run", "w", "--worker-file", "x.md"]))).toBe(
      "INVALID_REQUEST",
    );
  });

  it("rejects run with neither worker nor --worker-file", () => {
    expect(codeOf(() => parseCliArgs(["run"]))).toBe("INVALID_REQUEST");
  });

  it("rejects --worker-file on alias commands", () => {
    expect(codeOf(() => parseCliArgs(["bulk-read", "--worker-file", "x.md"]))).toBe(
      "INVALID_REQUEST",
    );
  });

  it("rejects path-like worker names as INVALID_WORKER_NAME", () => {
    expect(codeOf(() => parseCliArgs(["run", "./foo.md"]))).toBe("INVALID_WORKER_NAME");
    expect(codeOf(() => parseCliArgs(["run", "Foo"]))).toBe("INVALID_WORKER_NAME");
    expect(codeOf(() => parseCliArgs(["run", "a/b"]))).toBe("INVALID_WORKER_NAME");
  });

  it("rejects unknown options as INVALID_REQUEST", () => {
    expect(codeOf(() => parseCliArgs(["run", "w", "--bogus"]))).toBe("INVALID_REQUEST");
  });

  it("rejects unknown commands as INVALID_REQUEST", () => {
    expect(codeOf(() => parseCliArgs(["frobnicate"]))).toBe("INVALID_REQUEST");
  });

  it("rejects extra positionals", () => {
    expect(codeOf(() => parseCliArgs(["run", "w", "extra"]))).toBe("INVALID_REQUEST");
    expect(codeOf(() => parseCliArgs(["bulk-read", "extra"]))).toBe("INVALID_REQUEST");
  });

  it("rejects invalid --format values", () => {
    expect(codeOf(() => parseCliArgs(["run", "w", "--format", "xml"]))).toBe("INVALID_REQUEST");
  });

  it("requires an option value", () => {
    expect(codeOf(() => parseCliArgs(["run", "w", "--input"]))).toBe("INVALID_REQUEST");
  });

  it("defaults --input to stdin marker '-'", () => {
    const r = parseCliArgs(["run", "w"]);
    expect(r).toMatchObject({ input: "-" });
  });

  it("parses --input/--cwd/--timeout/--format/--quiet", () => {
    const r = parseCliArgs([
      "run",
      "w",
      "--input",
      "req.json",
      "--cwd",
      "/repo",
      "--timeout",
      "30s",
      "--format",
      "text",
      "--quiet",
    ]);
    expect(r).toMatchObject({
      input: "req.json",
      cwd: "/repo",
      timeout: "30s",
      format: "text",
      quiet: true,
    });
  });

  it("returns help/version kinds", () => {
    expect(parseCliArgs(["--help"])).toEqual({ kind: "help" });
    expect(parseCliArgs(["--version"])).toEqual({ kind: "version" });
    expect(parseCliArgs(["run", "w", "--help"])).toEqual({ kind: "help" });
  });

  it("rejects bare invocation without command", () => {
    expect(codeOf(() => parseCliArgs([]))).toBe("INVALID_REQUEST");
  });
});
