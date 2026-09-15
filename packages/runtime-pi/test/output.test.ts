import { compileSchema } from "@cheaper/core";
import { describe, expect, it } from "vitest";
import { extractLastAssistantText, extractOutput, stripCodeFence } from "../src/output.js";

const schema = compileSchema(
  {
    type: "object",
    required: ["ok"],
    properties: { ok: { type: "boolean" } },
    additionalProperties: false,
  },
  "output",
);

function assistantMsg(text: string | string[]) {
  return {
    role: "assistant" as const,
    content: (Array.isArray(text) ? text : [text]).map((t) => ({
      type: "text" as const,
      text: t,
    })),
  };
}

describe("stripCodeFence", () => {
  it("strips a ```json fenced block", () => {
    expect(stripCodeFence('```json\n{"ok":true}\n```')).toBe('{"ok":true}');
    expect(stripCodeFence('```\n{"ok":true}\n```\n')).toBe('{"ok":true}');
  });
  it("leaves unfenced text untouched", () => {
    expect(stripCodeFence('{"ok":true}')).toBe('{"ok":true}');
  });
});

describe("extractLastAssistantText", () => {
  it("concatenates text blocks of the last assistant message", () => {
    const messages = [
      { role: "user", content: "u" },
      assistantMsg(["part1 ", "part2"]),
      { role: "assistant", content: [] },
      assistantMsg("final"),
    ];
    expect(extractLastAssistantText(messages)).toBe("final");
  });
  it("ignores non-text blocks and non-assistant roles", () => {
    const messages = [
      assistantMsg("x"),
      { role: "toolResult", content: [{ type: "text", text: "tool" }] },
      {
        role: "assistant",
        content: [
          { type: "toolCall", id: "1", name: "read", arguments: {} },
          { type: "text", text: "tail" },
        ],
      },
    ];
    expect(extractLastAssistantText(messages)).toBe("tail");
  });
  it("returns undefined when no assistant message exists", () => {
    expect(extractLastAssistantText([{ role: "user", content: "u" }])).toBeUndefined();
  });
});

describe("extractOutput", () => {
  it("parses a bare JSON object and validates", () => {
    const r = extractOutput([assistantMsg('{"ok":true}')], schema);
    expect(r).toEqual({ ok: true, result: { ok: true }, rawOutput: '{"ok":true}' });
  });
  it("parses fenced JSON", () => {
    const r = extractOutput([assistantMsg('```json\n{"ok":true}\n```')], schema);
    expect(r.ok && r.result).toEqual({ ok: true });
  });
  it("fails with no-assistant-message", () => {
    const r = extractOutput([{ role: "user", content: "u" }], schema);
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.reason).toBe("no-assistant-message");
  });
  it("fails with invalid-json and preserves raw output", () => {
    const r = extractOutput([assistantMsg("not json at all")], schema);
    expect(r.ok).toBe(false);
    if (!r.ok) {
      expect(r.reason).toBe("invalid-json");
      expect(r.rawOutput).toBe("not json at all");
    }
  });
  it("fails with schema-mismatch and Ajv errors", () => {
    const r = extractOutput([assistantMsg('{"ok":"yes"}')], schema);
    expect(r.ok).toBe(false);
    if (!r.ok) {
      expect(r.reason).toBe("schema-mismatch");
      expect(r.errors.length).toBeGreaterThan(0);
      expect(r.rawOutput).toBe('{"ok":"yes"}');
    }
  });
  it("rejects non-object JSON as schema-mismatch", () => {
    const r = extractOutput([assistantMsg("[1,2]")], schema);
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.reason).toBe("schema-mismatch");
  });
});
