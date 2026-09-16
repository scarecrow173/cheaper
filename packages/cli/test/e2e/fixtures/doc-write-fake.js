import { createFakePi, fauxAssistantMessage, fauxToolCall } from "@cheaper/runtime-pi/testsupport";

// CHEAPER_TEST_FAKE_STREAM fixture (function form): one write tool call for a
// document, then a schema-conforming final message including an unresolved item.
export default async function createStream({ modelRuntime }) {
  const fake = createFakePi({
    responses: [
      fauxAssistantMessage(
        fauxToolCall("write", { path: "docs/guide.md", content: "# Guide\n\nbody\n" }),
        { stopReason: "toolUse" },
      ),
      fauxAssistantMessage(
        JSON.stringify({
          summary: "wrote docs/guide.md",
          notes: [],
          unresolved: ["release date not provided"],
        }),
      ),
    ],
  });
  modelRuntime.registerNativeProvider(fake.provider);
  await modelRuntime.refresh({ allowNetwork: false });
  return fake.streamFn;
}
