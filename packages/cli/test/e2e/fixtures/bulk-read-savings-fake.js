import { createFakePi, fauxAssistantMessage, fauxToolCall } from "@cheaper/runtime-pi/testsupport";

// CHEAPER_TEST_FAKE_STREAM fixture (function form): performs a real sandboxed
// `read` tool call so meta.savings.sourceTokens is actually measured, then
// returns a small structured result.
export default async function createStream({ modelRuntime }) {
  const fake = createFakePi({
    responses: [
      fauxAssistantMessage(fauxToolCall("read", { path: "big-input.txt" }), {
        stopReason: "toolUse",
      }),
      fauxAssistantMessage(
        JSON.stringify({
          summary: "savings summary",
          findings: [],
          relevantFiles: [{ path: "big-input.txt", reason: "read" }],
          suggestedReads: [],
          openQuestions: [],
        }),
      ),
    ],
  });
  modelRuntime.registerNativeProvider(fake.provider);
  await modelRuntime.refresh({ allowNetwork: false });
  return fake.streamFn;
}
