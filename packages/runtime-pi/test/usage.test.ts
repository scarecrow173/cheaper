import { describe, expect, it } from "vitest";
import { UsageCollector } from "../src/usage.js";

function usage(input: number, output: number, costTotal: number) {
  return {
    input,
    output,
    cacheRead: 0,
    cacheWrite: 0,
    totalTokens: input + output,
    cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: costTotal },
  };
}

function assistantMessageEnd(u: ReturnType<typeof usage>) {
  return {
    type: "message_end" as const,
    message: { role: "assistant" as const, content: [], usage: u },
  };
}

describe("UsageCollector", () => {
  it("sums usage from assistant message_end events", () => {
    const c = new UsageCollector();
    c.handleEvent(assistantMessageEnd(usage(10, 5, 0.01)));
    c.handleEvent(assistantMessageEnd(usage(20, 7, 0.02)));
    const u = c.snapshot().usage;
    expect(u).toEqual({
      input: 30,
      output: 12,
      cacheRead: 0,
      cacheWrite: 0,
      totalTokens: 42,
      costUsd: 0.03,
    });
  });

  it("includes cacheRead/cacheWrite in the sum", () => {
    const c = new UsageCollector();
    c.handleEvent(
      assistantMessageEnd({
        input: 1,
        output: 2,
        cacheRead: 3,
        cacheWrite: 4,
        totalTokens: 10,
        cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: 0.5 },
      }),
    );
    const u = c.snapshot().usage;
    expect(u.cacheRead).toBe(3);
    expect(u.cacheWrite).toBe(4);
    expect(u.totalTokens).toBe(10);
    expect(u.costUsd).toBe(0.5);
  });

  it("ignores non-assistant message_end events", () => {
    const c = new UsageCollector();
    c.handleEvent({
      type: "message_end",
      message: { role: "toolResult", content: [], usage: usage(9, 9, 9) },
    });
    expect(c.snapshot().usage.totalTokens).toBe(0);
  });

  it("counts turn_end and auto_retry_start", () => {
    const c = new UsageCollector();
    c.handleEvent({ type: "turn_end" });
    c.handleEvent({ type: "turn_end" });
    c.handleEvent({
      type: "auto_retry_start",
      attempt: 1,
      maxAttempts: 3,
      delayMs: 100,
      errorMessage: "e",
    });
    const s = c.snapshot();
    expect(s.turns).toBe(2);
    expect(s.transientRetries).toBe(1);
  });
});
