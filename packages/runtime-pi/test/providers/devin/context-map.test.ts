import type { Context } from "@earendil-works/pi-ai";
import { describe, expect, it } from "vitest";
import { mapContextToChat } from "../../../src/providers/devin/context-map.js";

describe("mapContextToChat", () => {
  it("maps system prompt, user text, assistant tool calls, and tool results", () => {
    const context: Context = {
      systemPrompt: "You are a worker.",
      messages: [
        { role: "user", content: "hello" },
        {
          role: "assistant",
          content: [
            { type: "text", text: "calling tool" },
            {
              type: "toolCall",
              id: "tc1",
              name: "read",
              arguments: { path: "a.ts" },
            },
          ],
          api: "devin-local",
          provider: "devin",
          model: "m",
          usage: {
            input: 0,
            output: 0,
            cacheRead: 0,
            cacheWrite: 0,
            totalTokens: 0,
            cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: 0 },
          },
          stopReason: "toolUse",
          timestamp: 0,
        },
        {
          role: "toolResult",
          toolCallId: "tc1",
          toolName: "read",
          content: [{ type: "text", text: "file body" }],
          isError: false,
          timestamp: 0,
        },
      ],
    } as unknown as Context;

    const mapped = mapContextToChat(context);
    expect(mapped.messages[0]).toEqual({ role: "system", content: "You are a worker." });
    expect(mapped.messages[1]).toEqual({ role: "user", content: "hello" });
    expect(mapped.messages[2]).toEqual({
      role: "assistant",
      content: "calling tool",
      tool_calls: [{ id: "tc1", name: "read", arguments: '{"path":"a.ts"}' }],
    });
    expect(mapped.messages[3]).toEqual({
      role: "tool",
      content: "file body",
      tool_call_id: "tc1",
    });
  });

  it("maps user image parts and context tools", () => {
    const context = {
      messages: [
        {
          role: "user",
          content: [
            { type: "text", text: "see" },
            { type: "image", mimeType: "image/png", data: "aGk=" },
          ],
        },
      ],
      tools: [{ name: "read", description: "read file", parameters: { type: "object" } }],
    } as unknown as Context;

    const mapped = mapContextToChat(context);
    expect(mapped.messages[0]?.content).toEqual([
      { type: "text", text: "see" },
      { type: "image", mimeType: "image/png", base64Data: "aGk=" },
    ]);
    expect(mapped.tools).toEqual([
      { name: "read", description: "read file", parameters: { type: "object" } },
    ]);
  });
});
