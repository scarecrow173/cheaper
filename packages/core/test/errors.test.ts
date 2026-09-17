import { describe, expect, it } from "vitest";
import { CheaperError, ERROR_TABLE, isCheaperError } from "../src/errors.js";

const EXPECTED: Record<string, { exitCode: number; retryable: boolean }> = {
  INVALID_REQUEST: { exitCode: 2, retryable: false },
  INVALID_INPUT: { exitCode: 2, retryable: false },
  FILE_TOO_LARGE: { exitCode: 2, retryable: false },
  WORKER_NOT_FOUND: { exitCode: 3, retryable: false },
  INVALID_WORKER_DEFINITION: { exitCode: 3, retryable: false },
  INVALID_WORKER_NAME: { exitCode: 3, retryable: false },
  WORKER_NAME_MISMATCH: { exitCode: 3, retryable: false },
  UNKNOWN_TOOL: { exitCode: 3, retryable: false },
  INVALID_RUNTIME_POLICY: { exitCode: 3, retryable: false },
  OUTPUT_SCHEMA_NOT_FOUND: { exitCode: 3, retryable: false },
  INVALID_OUTPUT_SCHEMA: { exitCode: 3, retryable: false },
  INPUT_SCHEMA_NOT_FOUND: { exitCode: 3, retryable: false },
  INVALID_INPUT_SCHEMA: { exitCode: 3, retryable: false },
  INVALID_MODEL_CONFIG: { exitCode: 3, retryable: false },
  POLICY_VIOLATION: { exitCode: 4, retryable: false },
  MODEL_TIER_UNAVAILABLE: { exitCode: 5, retryable: true },
  PROVIDER_FAILURE: { exitCode: 5, retryable: true },
  TOOL_EXECUTION_FAILURE: { exitCode: 6, retryable: false },
  INVALID_OUTPUT: { exitCode: 6, retryable: false },
  MAX_TURNS_EXCEEDED: { exitCode: 6, retryable: false },
  MAX_TOOL_CALLS_EXCEEDED: { exitCode: 6, retryable: false },
  TIMEOUT: { exitCode: 7, retryable: true },
  ABORTED: { exitCode: 8, retryable: false },
  PARTIAL_MUTATION: { exitCode: 9, retryable: false },
  INTERNAL_ERROR: { exitCode: 70, retryable: false },
};

describe("ERROR_TABLE", () => {
  it("covers every expected code exactly once", () => {
    expect(Object.keys(ERROR_TABLE).sort()).toEqual(Object.keys(EXPECTED).sort());
  });
  it.each(Object.entries(EXPECTED))("%s maps to %o", (code, expected) => {
    expect(ERROR_TABLE[code as keyof typeof ERROR_TABLE]).toEqual(expected);
  });
});

describe("CheaperError", () => {
  it("sets code, exitCode, retryable from the table", () => {
    const e = new CheaperError("TIMEOUT", "timed out");
    expect(e.code).toBe("TIMEOUT");
    expect(e.exitCode).toBe(7);
    expect(e.retryable).toBe(true);
    expect(e.message).toBe("timed out");
    expect(e).toBeInstanceOf(Error);
  });
  it("carries details and cause", () => {
    const cause = new Error("root");
    const e = new CheaperError("INTERNAL_ERROR", "x", {
      details: { a: 1 },
      cause,
    });
    expect(e.details).toEqual({ a: 1 });
    expect(e.cause).toBe(cause);
  });
  it("omits details/cause when not given", () => {
    const e = new CheaperError("ABORTED", "x");
    expect(e.details).toBeUndefined();
    expect(e.cause).toBeUndefined();
  });
});

describe("isCheaperError", () => {
  it("returns true for CheaperError", () => {
    expect(isCheaperError(new CheaperError("ABORTED", "x"))).toBe(true);
  });
  it("returns false for plain errors and non-errors", () => {
    expect(isCheaperError(new Error("x"))).toBe(false);
    expect(isCheaperError("x")).toBe(false);
    expect(isCheaperError(null)).toBe(false);
  });
});
