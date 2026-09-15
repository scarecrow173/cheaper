import { describe, expect, it } from "vitest";
import { CheaperError } from "../../src/errors.js";
import { normalizeToolNames, TOOL_ALIASES, TOOL_NAMES } from "../../src/tools/vocabulary.js";

describe("normalizeToolNames", () => {
  it("passes canonical names through", () => {
    expect(normalizeToolNames(["read", "grep"])).toEqual(["read", "grep"]);
  });
  it("normalizes allowed aliases to canonical names", () => {
    expect(normalizeToolNames(["Read", "Grep", "Glob", "Write", "Edit"])).toEqual([
      "read",
      "grep",
      "find",
      "write",
      "edit",
    ]);
  });
  it("exposes exactly the allowed aliases", () => {
    expect(Object.keys(TOOL_ALIASES).sort()).toEqual(["Edit", "Glob", "Grep", "Read", "Write"]);
  });
  it.each([undefined, null, "read", 5, {}])(
    "rejects non-array %j as INVALID_WORKER_DEFINITION",
    (raw) => {
      expect(() => normalizeToolNames(raw)).toThrowError(CheaperError);
      try {
        normalizeToolNames(raw);
      } catch (e) {
        expect((e as CheaperError).code).toBe("INVALID_WORKER_DEFINITION");
      }
    },
  );
  it("rejects empty array", () => {
    expect(() => normalizeToolNames([])).toThrowError(
      expect.objectContaining({ code: "INVALID_WORKER_DEFINITION" }),
    );
  });
  it("rejects non-string entries", () => {
    expect(() => normalizeToolNames(["read", 3])).toThrowError(
      expect.objectContaining({ code: "INVALID_WORKER_DEFINITION" }),
    );
  });
  it("rejects unknown tool with details", () => {
    try {
      normalizeToolNames(["bash"]);
      expect.unreachable();
    } catch (e) {
      const err = e as CheaperError;
      expect(err.code).toBe("UNKNOWN_TOOL");
      expect(err.details?.tool).toBe("bash");
      expect(err.details?.known).toEqual(TOOL_NAMES);
    }
  });
  it("rejects case variants not in the alias table", () => {
    expect(() => normalizeToolNames(["READ"])).toThrowError(
      expect.objectContaining({ code: "UNKNOWN_TOOL" }),
    );
  });
  it("rejects duplicates after normalization", () => {
    expect(() => normalizeToolNames(["read", "Read"])).toThrowError(
      expect.objectContaining({ code: "INVALID_WORKER_DEFINITION" }),
    );
  });
});
