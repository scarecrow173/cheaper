import { countTextTokens } from "@cheaper/core";
import type { ToolDefinition } from "@earendil-works/pi-coding-agent";
import { describe, expect, it } from "vitest";
import { wrapToolMeter } from "../../src/tools/meter.js";

function fakeTool(result: unknown, throws = false): ToolDefinition {
  return {
    name: "fake",
    label: "fake",
    description: "fake",
    parameters: { type: "object", properties: {} },
    execute: async () => {
      if (throws) throw new Error("tool failed");
      return result;
    },
  } as unknown as ToolDefinition;
}

async function exec(tool: ToolDefinition): Promise<unknown> {
  return tool.execute("c1", {} as never, undefined, undefined, undefined as never);
}

describe("wrapToolMeter", () => {
  it("sums canonical tokens of text content parts across calls", async () => {
    const { tools, meter } = wrapToolMeter([
      fakeTool({ content: [{ type: "text", text: "hello world" }] }),
    ]);
    expect(meter.sourceTokens).toBe(0);
    await exec(tools[0] as ToolDefinition);
    const after = meter.sourceTokens;
    expect(after).toBe(countTextTokens("hello world"));
    await exec(tools[0] as ToolDefinition);
    expect(meter.sourceTokens).toBe(after * 2);
  });

  it("sums multiple text parts and ignores non-text parts", async () => {
    const { tools, meter } = wrapToolMeter([
      fakeTool({
        content: [
          { type: "text", text: "aa" },
          { type: "image", data: "bb" },
          { type: "text", text: "cc" },
        ],
      }),
    ]);
    await exec(tools[0] as ToolDefinition);
    expect(meter.sourceTokens).toBe(countTextTokens("aa") + countTextTokens("cc"));
  });

  it("does not count a thrown tool error", async () => {
    const { tools, meter } = wrapToolMeter([fakeTool(undefined, true)]);
    await expect(exec(tools[0] as ToolDefinition)).rejects.toThrow("tool failed");
    expect(meter.sourceTokens).toBe(0);
  });

  it("treats results without text content as zero", async () => {
    const { tools, meter } = wrapToolMeter([
      fakeTool({ content: [] }),
      fakeTool(undefined),
      fakeTool({ content: [{ type: "image", data: "x" }] }),
    ]);
    for (const t of tools) await exec(t);
    expect(meter.sourceTokens).toBe(0);
  });
});
