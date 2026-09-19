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

  it("parses --model <provider>/<id> on run and aliases", () => {
    expect(parseCliArgs(["run", "w", "--model", "anthropic/claude-haiku-4-5"])).toMatchObject({
      kind: "run",
      model: "anthropic/claude-haiku-4-5",
    });
    expect(parseCliArgs(["bulk-read", "--model", "openai-codex/gpt-5.1"])).toMatchObject({
      kind: "run",
      workerName: "bulk-reader",
      model: "openai-codex/gpt-5.1",
    });
  });

  it("rejects malformed --model references", () => {
    expect(codeOf(() => parseCliArgs(["bulk-read", "--model", "no-slash"]))).toBe(
      "INVALID_REQUEST",
    );
    expect(codeOf(() => parseCliArgs(["bulk-read", "--model", "p/"]))).toBe("INVALID_REQUEST");
    expect(codeOf(() => parseCliArgs(["bulk-read", "--model", "/m"]))).toBe("INVALID_REQUEST");
  });

  it("rejects --model on count-tokens, auth, and models", () => {
    expect(codeOf(() => parseCliArgs(["count-tokens", "f.txt", "--model", "p/m"]))).toBe(
      "INVALID_REQUEST",
    );
    expect(codeOf(() => parseCliArgs(["auth", "status", "--model", "p/m"]))).toBe(
      "INVALID_REQUEST",
    );
    expect(codeOf(() => parseCliArgs(["models", "--model", "p/m"]))).toBe("INVALID_REQUEST");
  });

  it("parses `models` with --provider and --tier filters", () => {
    expect(parseCliArgs(["models"])).toMatchObject({ kind: "models", provider: undefined });
    expect(parseCliArgs(["models", "--provider", "anthropic", "--tier", "low"])).toMatchObject({
      kind: "models",
      provider: "anthropic",
      tier: "low",
    });
  });

  it("rejects invalid --tier and run-only options on models", () => {
    expect(codeOf(() => parseCliArgs(["models", "--tier", "extreme"]))).toBe("INVALID_REQUEST");
    expect(codeOf(() => parseCliArgs(["models", "--input", "f.json"]))).toBe("INVALID_REQUEST");
    expect(codeOf(() => parseCliArgs(["models", "extra"]))).toBe("INVALID_REQUEST");
  });

  it("rejects --provider/--tier on run", () => {
    expect(codeOf(() => parseCliArgs(["bulk-read", "--provider", "anthropic"]))).toBe(
      "INVALID_REQUEST",
    );
    expect(codeOf(() => parseCliArgs(["bulk-read", "--tier", "low"]))).toBe("INVALID_REQUEST");
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

describe("count-tokens", () => {
  it("parses count-tokens <path> with --cwd/--format/--quiet", () => {
    const r = parseCliArgs([
      "count-tokens",
      "src/a.ts",
      "--cwd",
      "/repo",
      "--format",
      "text",
      "--quiet",
    ]);
    expect(r).toEqual({
      kind: "count-tokens",
      path: "src/a.ts",
      cwd: "/repo",
      format: "text",
      quiet: true,
    });
  });

  it("requires a path", () => {
    expect(codeOf(() => parseCliArgs(["count-tokens"]))).toBe("INVALID_REQUEST");
  });

  it("rejects extra positionals", () => {
    expect(codeOf(() => parseCliArgs(["count-tokens", "a.ts", "b.ts"]))).toBe("INVALID_REQUEST");
  });

  it("rejects worker-run options", () => {
    expect(codeOf(() => parseCliArgs(["count-tokens", "a.ts", "--worker-file", "w.md"]))).toBe(
      "INVALID_REQUEST",
    );
    expect(codeOf(() => parseCliArgs(["count-tokens", "a.ts", "--timeout", "30s"]))).toBe(
      "INVALID_REQUEST",
    );
    expect(codeOf(() => parseCliArgs(["count-tokens", "a.ts", "--input", "x.json"]))).toBe(
      "INVALID_REQUEST",
    );
  });
});

describe("usage", () => {
  it("parses bare usage with defaults", () => {
    expect(parseCliArgs(["usage"])).toEqual({
      kind: "usage",
      since: undefined,
      worker: undefined,
      provider: undefined,
      model: undefined,
      compareModel: undefined,
      format: "json",
      quiet: false,
    });
  });

  it("parses all usage filters", () => {
    const r = parseCliArgs([
      "usage",
      "--since",
      "24h",
      "--worker",
      "bulk-reader",
      "--provider",
      "ollama",
      "--model",
      "ollama/qwen3.5:9b",
      "--compare-model",
      "openai/gpt-5",
      "--format",
      "text",
    ]);
    expect(r).toEqual({
      kind: "usage",
      since: "24h",
      worker: "bulk-reader",
      provider: "ollama",
      model: "ollama/qwen3.5:9b",
      compareModel: "openai/gpt-5",
      format: "text",
      quiet: false,
    });
  });

  it("rejects extra positionals and worker-run/auth options", () => {
    expect(codeOf(() => parseCliArgs(["usage", "extra"]))).toBe("INVALID_REQUEST");
    expect(codeOf(() => parseCliArgs(["usage", "--input", "x.json"]))).toBe("INVALID_REQUEST");
    expect(codeOf(() => parseCliArgs(["usage", "--timeout", "30s"]))).toBe("INVALID_REQUEST");
    expect(codeOf(() => parseCliArgs(["usage", "--worker-file", "w.md"]))).toBe("INVALID_REQUEST");
    expect(codeOf(() => parseCliArgs(["usage", "--method", "device"]))).toBe("INVALID_REQUEST");
  });

  it("rejects usage filters on other commands", () => {
    expect(codeOf(() => parseCliArgs(["run", "bulk-reader", "--since", "1h"]))).toBe(
      "INVALID_REQUEST",
    );
    expect(codeOf(() => parseCliArgs(["count-tokens", "a.ts", "--worker", "w"]))).toBe(
      "INVALID_REQUEST",
    );
    expect(codeOf(() => parseCliArgs(["auth", "status", "--provider", "openai"]))).toBe(
      "INVALID_REQUEST",
    );
  });
});
