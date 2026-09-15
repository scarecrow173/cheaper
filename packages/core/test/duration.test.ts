import { describe, expect, it } from "vitest";
import { DURATION_PATTERN, parseDuration } from "../src/duration.js";

describe("parseDuration", () => {
  it.each([
    ["500ms", 500],
    ["1ms", 1],
    ["30s", 30_000],
    ["5m", 300_000],
    ["2h", 7_200_000],
  ])("parses %s to %d ms", (input, ms) => {
    expect(parseDuration(input)).toBe(ms);
  });

  it.each(["0s", "0ms", "01s", "10", "s", "1.5s", "1d", "-5s", "", "ms", "10ss", " 5s", "5s "])(
    "rejects %j with RangeError",
    (input) => {
      expect(() => parseDuration(input)).toThrow(RangeError);
    },
  );
});

describe("DURATION_PATTERN", () => {
  it("matches only non-zero unsigned integer + suffix", () => {
    expect(DURATION_PATTERN.test("10m")).toBe(true);
    expect(DURATION_PATTERN.test("0m")).toBe(false);
    expect(DURATION_PATTERN.test("m")).toBe(false);
  });
});
