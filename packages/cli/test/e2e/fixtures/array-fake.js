import { fauxAssistantMessage } from "@cheaper/runtime-pi/testsupport";

// CHEAPER_TEST_FAKE_STREAM fixture (scripted response array form): the CLI
// wraps this in createFakePi and registers the provider itself.
export default [
  fauxAssistantMessage(
    JSON.stringify({
      summary: "array form ok",
      findings: [],
      relevantFiles: [],
      suggestedReads: [],
      openQuestions: [],
    }),
  ),
];
