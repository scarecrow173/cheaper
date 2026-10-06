import { describe, expect, it } from "vitest";
import type { WorkerDefinition } from "../src/index.js";
import * as core from "../src/index.js";

const worker = (field?: string) => ({ cheaper: { writeScope: field } }) as WorkerDefinition;
describe("literal writeScope extraction", () => {
  it("未指定は undefined、明示空配列は empty set", () => {
    expect(core.extractWriteScope(worker(), {})).toBeUndefined();
    expect(core.extractWriteScope(worker("targets"), { targets: [] })).toEqual(new Set());
  });
  it("literal strings/path objects を deduplicate し canonicalization をしない", () => {
    const input = { targets: ["./a", { path: "b\\c" }, "./a"] };
    expect(core.extractWriteScope(worker("targets"), input)).toEqual(new Set(["./a", "b\\c"]));
    expect(input.targets).toHaveLength(3);
  });
  it.each([{}, null, [], { targets: "a" }, { targets: [1] }, { targets: [{ path: 3 }] }])(
    "invalid declaration input は fail closed: %j",
    (input) => {
      expect(() => core.extractWriteScope(worker("targets"), input)).toThrow(
        expect.objectContaining({ code: "INVALID_INPUT" }),
      );
    },
  );
});
