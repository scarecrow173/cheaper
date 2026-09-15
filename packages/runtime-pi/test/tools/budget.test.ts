import type { ToolDefinition } from "@earendil-works/pi-coding-agent";
import { Type } from "typebox";
import { describe, expect, it, vi } from "vitest";
import { wrapToolBudget } from "../../src/tools/budget.js";
import { ACCESS_DENIED_MESSAGE } from "../../src/tools/walker.js";

function first(tools: ToolDefinition[]): ToolDefinition {
  const t = tools[0];
  if (t === undefined) throw new Error("no tools");
  return t;
}

function makeTool(name: string, impl?: () => Promise<unknown> | unknown): ToolDefinition {
  return {
    name,
    label: name,
    description: `${name} tool`,
    parameters: Type.Object({}),
    execute: async () => ({
      content: [{ type: "text" as const, text: String((await impl?.()) ?? "ok") }],
      details: undefined,
    }),
  };
}

function deniedTool(): ToolDefinition {
  return {
    name: "denied",
    label: "denied",
    description: "always denied",
    parameters: Type.Object({}),
    execute: async () => {
      throw new Error(ACCESS_DENIED_MESSAGE);
    },
  };
}

describe("wrapToolBudget", () => {
  it("delegates calls under the budget and counts them", async () => {
    const impl = vi.fn();
    const budgeted = wrapToolBudget([makeTool("a", impl)], {
      maxToolCalls: 2,
      onLimit: () => {},
    });
    await first(budgeted.tools).execute("1", {}, undefined, undefined, undefined as never);
    await first(budgeted.tools).execute("2", {}, undefined, undefined, undefined as never);
    expect(impl).toHaveBeenCalledTimes(2);
    expect(budgeted.budget.toolCalls).toBe(2);
    expect(budgeted.budget.limitHit).toBeNull();
  });

  it("rejects calls over the budget: error result + limitHit + onLimit", async () => {
    const onLimit = vi.fn();
    const impl = vi.fn();
    const budgeted = wrapToolBudget([makeTool("a", impl)], { maxToolCalls: 1, onLimit });
    const t = first(budgeted.tools);
    await t.execute("1", {}, undefined, undefined, undefined as never);
    await expect(t.execute("2", {}, undefined, undefined, undefined as never)).rejects.toThrow(
      /maxToolCalls/,
    );
    expect(impl).toHaveBeenCalledTimes(1);
    expect(budgeted.budget.toolCalls).toBe(1);
    expect(budgeted.budget.rejectedToolCalls).toBe(1);
    expect(budgeted.budget.limitHit).toBe("maxToolCalls");
    expect(onLimit).toHaveBeenCalledTimes(1);
  });

  it("counts boundary-denied tool errors as rejectedToolCalls", async () => {
    const budgeted = wrapToolBudget([deniedTool()], { maxToolCalls: 5, onLimit: () => {} });
    await expect(
      first(budgeted.tools).execute("1", {}, undefined, undefined, undefined as never),
    ).rejects.toThrow(ACCESS_DENIED_MESSAGE);
    expect(budgeted.budget.rejectedToolCalls).toBe(1);
    expect(budgeted.budget.toolCalls).toBe(1);
  });

  it("does not count non-boundary tool errors as rejected", async () => {
    const boom: ToolDefinition = {
      name: "boom",
      label: "boom",
      description: "boom",
      parameters: Type.Object({}),
      execute: async () => {
        throw new Error("unrelated failure");
      },
    };
    const budgeted = wrapToolBudget([boom], { maxToolCalls: 5, onLimit: () => {} });
    await expect(
      first(budgeted.tools).execute("1", {}, undefined, undefined, undefined as never),
    ).rejects.toThrow("unrelated failure");
    expect(budgeted.budget.rejectedToolCalls).toBe(0);
    expect(budgeted.budget.toolCalls).toBe(1);
  });

  it("preserves tool name/label/description on wrapped definitions", () => {
    const budgeted = wrapToolBudget([makeTool("x")], { maxToolCalls: 1, onLimit: () => {} });
    const t = first(budgeted.tools);
    expect(t.name).toBe("x");
    expect(t.label).toBe("x");
    expect(t.description).toBe("x tool");
  });
});
