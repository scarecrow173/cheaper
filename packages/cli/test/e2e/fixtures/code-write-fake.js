import { createFakePi, fauxAssistantMessage, fauxToolCall } from "@cheaper/runtime-pi/testsupport";

// CHEAPER_TEST_FAKE_STREAM fixture (function form): one write tool call, then
// a schema-conforming final message. meta.changes must be measured by the
// wrapper, not reported by the model.
export default async function createStream({ modelRuntime }) {
  const fake = createFakePi({
    responses: [
      fauxAssistantMessage(fauxToolCall("write", { path: "out.txt", content: "hello\n" }), {
        stopReason: "toolUse",
      }),
      fauxAssistantMessage(JSON.stringify({ summary: "wrote out.txt", notes: [], unresolved: [] })),
    ],
  });
  modelRuntime.registerNativeProvider(fake.provider);
  await modelRuntime.refresh({ allowNetwork: false });
  return fake.streamFn;
}
