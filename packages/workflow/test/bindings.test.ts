import { describe, expect, it } from "vitest";
import { compileDag, materializeBindings, resolvePointer, scanBindings } from "../src/index.js";

describe("binding の単一解釈と result-root Pointer", () => {
  it("input の JSON 型と literal を保ち、挿入値を再 scan しない", () => {
    const inserted = { $from: "ghost", path: "" };
    const template = {
      input: { $input: "data" },
      literal: { $literal: { $input: "missing", extra: true } },
      array: [{ $input: "bool" }, { $input: "number" }, { $input: "nil" }],
    };
    expect(scanBindings(template)).toEqual({ inputs: ["bool", "data", "nil", "number"], from: [] });
    const resolved = materializeBindings(template, { data: inserted, bool: true, number: 7, nil: null });
    expect(resolved).toEqual({ input: inserted, literal: { $input: "missing", extra: true }, array: [true, 7, null] });
    expect((resolved as { input: unknown }).input).not.toBe(inserted);
  });

  it("literal を一度だけ剥がし、from 挿入値も再 scan しない", () => {
    const result = { $input: "ghost" };
    expect(materializeBindings({ $from: "a", path: "" }, {}, new Map([["a", result]]))).toEqual(result);
    expect(materializeBindings({ $literal: { $literal: 42 } }, {})).toEqual({ $literal: 42 });
  });

  it.each([
    { $input: "x", extra: 1 }, { $input: 7 }, { $from: "a" },
    { $from: "a", path: "result" }, { $from: "a", path: "/bad~2" },
    { $from: "a", path: "", extra: true }, { $literal: null, other: 1 },
    { $input: "x", $from: "a", path: "" },
  ])("不正 reserved wrapper を definition error にする: %j", (value) => {
    expect(() => scanBindings(value)).toThrow(expect.objectContaining({ code: "INVALID_REQUEST" }));
  });

  it("空文字/escape/array/null を result 自体から解決する", () => {
    const result = { details: { "a/b": { "~": [null, 9] } }, result: "nested" };
    expect(resolvePointer(result, "")).toEqual(result);
    expect(resolvePointer(result, "/details/a~1b/~0/0")).toBeNull();
    expect(resolvePointer(result, "/details/a~1b/~0/1")).toBe(9);
    expect(resolvePointer(result, "/result")).toBe("nested");
    expect(resolvePointer({ "": 5 }, "/")).toBe(5);
  });

  it.each(["/missing", "/details/a~1b/~0/2", "/details/a~1b/~0/01", "/details/a~1b/~0/-", "/toString"])("missing/不正 index/継承 key は拒否: %s", (pointer) => {
    expect(() => resolvePointer({ details: { "a/b": { "~": [null, 9] } } }, pointer)).toThrow();
  });

  it("欠落 input/result は明示的な INVALID_INPUT", () => {
    expect(() => materializeBindings({ $input: "absent" }, {})).toThrow(expect.objectContaining({ code: "INVALID_INPUT" }));
    expect(() => materializeBindings({ $from: "absent", path: "" }, {})).toThrow(expect.objectContaining({ code: "INVALID_INPUT" }));
  });
});

describe("DAG の正本 order/edges", () => {
  it("辺を dedupe し locale 非依存の順序にする", () => {
    expect(compileDag(["z", "2", "10", "a"], [{ from: "a", to: "z" }, { from: "a", to: "z" }])).toEqual({
      order: ["10", "2", "a", "z"], edges: [{ from: "a", to: "z" }],
    });
  });
  it.each([
    { edges: [{ from: "a", to: "a" }] }, { edges: [{ from: "unknown", to: "b" }] },
    { edges: [{ from: "a", to: "b" }, { from: "b", to: "a" }] },
  ])("self/unknown/cycle は definition error: %j", ({ edges }) => {
    expect(() => compileDag(["a", "b"], edges)).toThrow(expect.objectContaining({ code: "INVALID_REQUEST" }));
  });
});
