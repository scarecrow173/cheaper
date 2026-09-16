import { writeFileSync } from "node:fs";
import { createFakePi, fauxAssistantMessage, fauxToolCall } from "@cheaper/runtime-pi/testsupport";

// CHEAPER_TEST_FAKE_STREAM fixture (function form): one count_tokens tool call,
// then a schema-conforming final message. The wrapped streamFn captures the
// toolResult content into CHEAPER_TEST_CAPTURE so the e2e test can compare the
// tool's count against the `cheaper count-tokens` CLI output (parity).
export default async function createStream({ modelRuntime }) {
  const fake = createFakePi({
    responses: [
      fauxAssistantMessage(fauxToolCall("count_tokens", { path: "parity.txt" }), {
        stopReason: "toolUse",
      }),
      fauxAssistantMessage(
        JSON.stringify({
          summary: "counted parity.txt",
          findings: [],
          relevantFiles: [{ path: "parity.txt", reason: "counted file" }],
          suggestedReads: [],
          openQuestions: [],
        }),
      ),
    ],
  });
  modelRuntime.registerNativeProvider(fake.provider);
  await modelRuntime.refresh({ allowNetwork: false });
  const inner = fake.streamFn;
  return (model, context, opts) => {
    const capture = process.env.CHEAPER_TEST_CAPTURE;
    if (capture) {
      for (const m of context.messages) {
        if (m.role === "toolResult" && m.toolName === "count_tokens") {
          const text = m.content
            .filter((c) => c.type === "text")
            .map((c) => c.text)
            .join("");
          writeFileSync(capture, text);
        }
      }
    }
    return inner(model, context, opts);
  };
}
