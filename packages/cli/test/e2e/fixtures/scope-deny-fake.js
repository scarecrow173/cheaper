import { createFakePi, fauxAssistantMessage, fauxToolCall } from "@cheaper/runtime-pi/testsupport";

// CHEAPER_TEST_FAKE_STREAM fixture (function form): attempts one write outside
// the declared targets and one read of a denied credential file, then finishes
// with a schema-conforming message. Both calls must fail at the sandbox layer.
export default async function createStream({ modelRuntime }) {
  const fake = createFakePi({
    responses: [
      fauxAssistantMessage(fauxToolCall("write", { path: "evil.txt", content: "x" }), {
        stopReason: "toolUse",
      }),
      fauxAssistantMessage(fauxToolCall("read", { path: ".env" }), { stopReason: "toolUse" }),
      fauxAssistantMessage(
        JSON.stringify({
          summary: "blocked: out-of-scope write and denied read",
          notes: [],
          unresolved: ["evil.txt is outside targets", ".env is denied"],
        }),
      ),
    ],
  });
  modelRuntime.registerNativeProvider(fake.provider);
  await modelRuntime.refresh({ allowNetwork: false });
  return fake.streamFn;
}
