import { createFakePi, fauxAssistantMessage } from "@cheaper/runtime-pi/testsupport";

// CHEAPER_TEST_FAKE_STREAM fixture (function form): registers the fake provider
// on the CLI-created ModelRuntime and returns the scripted streamFn.
export default async function createStream({ modelRuntime }) {
  const fake = createFakePi({
    responses: [
      fauxAssistantMessage(
        JSON.stringify({
          summary: "workspace summary",
          findings: [
            {
              title: "entry point",
              detail: "bin/cheaper.js loads dist/main.js",
              confidence: "high",
              locations: [{ path: "marker.txt", lines: { start: 1, end: 1 } }],
            },
          ],
          relevantFiles: [{ path: "marker.txt", reason: "fixture file" }],
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
