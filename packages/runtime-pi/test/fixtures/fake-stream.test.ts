import type { AssistantMessageEvent, Context } from "@earendil-works/pi-ai";
import { describe, expect, it } from "vitest";
import { createFakePi, fauxAssistantMessage, fauxText, fauxToolCall } from "./fake-stream.js";

const ctx: Context = { messages: [{ role: "user", content: "hi", timestamp: 0 }] };

async function drain(stream: AsyncIterable<AssistantMessageEvent>) {
  const events: AssistantMessageEvent[] = [];
  for await (const e of stream) events.push(e);
  return events;
}

describe("createFakePi", () => {
  it("emits a scripted text message ending with done(stop)", async () => {
    const fake = createFakePi({ responses: [fauxAssistantMessage("hello world")] });
    const stream = fake.streamFn(fake.model, ctx, undefined);
    const events = await drain(stream);
    const types = events.map((e) => e.type);
    expect(types[0]).toBe("start");
    expect(types.at(-1)).toBe("done");
    expect(types).toContain("text_start");
    expect(types).toContain("text_end");
    const last = events.at(-1);
    expect(last?.type === "done" && last.reason).toBe("stop");
    const message = await stream.result();
    expect(message.content).toEqual([{ type: "text", text: "hello world" }]);
  });

  it("emits scripted tool calls ending with done(toolUse)", async () => {
    const fake = createFakePi({
      responses: [
        fauxAssistantMessage(fauxToolCall("read", { path: "a.txt" }), {
          stopReason: "toolUse",
        }),
      ],
    });
    const events = await drain(fake.streamFn(fake.model, ctx, undefined));
    const end = events.find((e) => e.type === "toolcall_end");
    expect(end?.type === "toolcall_end" && end.toolCall.name).toBe("read");
    const last = events.at(-1);
    expect(last?.type === "done" && last.reason).toBe("toolUse");
  });

  it("emits error events for scripted error messages", async () => {
    const fake = createFakePi({
      responses: [fauxAssistantMessage("", { stopReason: "error", errorMessage: "boom" })],
    });
    const events = await drain(fake.streamFn(fake.model, ctx, undefined));
    const last = events.at(-1);
    expect(last?.type).toBe("error");
    expect(last?.type === "error" && last.error.errorMessage).toBe("boom");
  });

  it("emits error when the response queue is exhausted", async () => {
    const fake = createFakePi();
    const events = await drain(fake.streamFn(fake.model, ctx, undefined));
    const last = events.at(-1);
    expect(last?.type === "error" && last.reason).toBe("error");
  });

  it("honors a pre-aborted options.signal", async () => {
    const fake = createFakePi({ responses: [fauxAssistantMessage("never")] });
    const ac = new AbortController();
    ac.abort();
    const events = await drain(fake.streamFn(fake.model, ctx, { signal: ac.signal }));
    const last = events.at(-1);
    expect(last?.type === "error" && last.reason).toBe("aborted");
  });

  it("honors an abort delivered while streaming", async () => {
    const fake = createFakePi({
      responses: [fauxAssistantMessage("x".repeat(2000))],
    });
    const ac = new AbortController();
    const stream = fake.streamFn(fake.model, ctx, { signal: ac.signal });
    ac.abort();
    const events = await drain(stream);
    const last = events.at(-1);
    expect(last?.type === "error" && last.reason).toBe("aborted");
  });

  it("records calls with context and options (incl. maxTokens)", async () => {
    const fake = createFakePi({ responses: [fauxAssistantMessage("ok")] });
    await drain(fake.streamFn(fake.model, ctx, { maxTokens: 1234 }));
    expect(fake.calls).toHaveLength(1);
    expect(fake.calls[0]?.options?.maxTokens).toBe(1234);
    expect(fake.calls[0]?.context).toBe(ctx);
    expect(fake.state.callCount).toBe(1);
  });

  it("supports factory response steps receiving context/options/state", async () => {
    const fake = createFakePi();
    fake.setResponses([
      (_context, options) => fauxAssistantMessage(`mt=${String(options?.maxTokens)}`),
    ]);
    const stream = fake.streamFn(fake.model, ctx, { maxTokens: 77 });
    const message = await stream.result();
    expect(message.content).toEqual([{ type: "text", text: "mt=77" }]);
  });

  it("mixed text + toolCall content emits both event families", async () => {
    const fake = createFakePi({
      responses: [
        fauxAssistantMessage([fauxText("checking"), fauxToolCall("ls", {})], {
          stopReason: "toolUse",
        }),
      ],
    });
    const events = await drain(fake.streamFn(fake.model, ctx, undefined));
    const types = events.map((e) => e.type);
    expect(types).toContain("text_end");
    expect(types).toContain("toolcall_end");
  });
});
