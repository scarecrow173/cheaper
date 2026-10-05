import { appendFileSync } from "node:fs";
const supportUrl = new URL("../../packages/runtime-pi/dist/testsupport.js", import.meta.url);
const { createFakePi, fauxAssistantMessage, fauxToolCall } = await import(supportUrl.href);

function response(profile, index) {
  const large = profile === "large";
  return fauxAssistantMessage(JSON.stringify({
    summary: large ? `seeded compact summary ${index}` : "seeded compact summary",
    findings: large ? [{
      title: `fixture finding ${index}`,
      detail: "x".repeat(1000),
      confidence: "high",
      locations: [{ path: "README.md", lines: { start: 1, end: 1 } }],
    }] : [],
    relevantFiles: [],
    suggestedReads: [],
    openQuestions: [],
  }));
}

export default async function createStream({ modelRuntime }) {
  const profiles = (process.env.CHEAPER_BENCH_RESPONSE_PROFILES ?? "compact")
    .split(",")
    .filter(Boolean);
  const responseDelayMs = Number(process.env.CHEAPER_BENCH_RESPONSE_DELAY_MS ?? "0");
  const steps = [];
  if (process.env.CHEAPER_BENCH_TOOL_CALL_PATH) {
    steps.push(fauxAssistantMessage(
      fauxToolCall("read", { path: process.env.CHEAPER_BENCH_TOOL_CALL_PATH }),
      { stopReason: "toolUse" },
    ));
  }
  for (let index = 0; index < profiles.length; index += 1) {
    steps.push(async () => {
      if (responseDelayMs > 0) await new Promise((resolve) => setTimeout(resolve, responseDelayMs));
      return response(profiles[index], index);
    });
  }
  const fake = createFakePi({ responses: steps });
  modelRuntime.registerNativeProvider(fake.provider);
  await modelRuntime.refresh({ allowNetwork: false });
  const promptLog = process.env.CHEAPER_BENCH_PROMPT_LOG;
  const timingLog = process.env.CHEAPER_BENCH_TIMING_LOG;
  return (model, context, options) => {
    if (promptLog) {
      const prompt = JSON.stringify(context);
      appendFileSync(promptLog, `${JSON.stringify({ bytes: Buffer.byteLength(prompt) })}\n`);
    }
    const id = `${process.pid}-${Date.now()}-${Math.random().toString(36).slice(2)}`;
    const atStart = process.hrtime.bigint().toString();
    if (timingLog) appendFileSync(timingLog, `${JSON.stringify({ id, phase: "start", at: atStart })}\n`);
    const source = fake.streamFn(model, context, options);
    const originalIterator = source[Symbol.asyncIterator].bind(source);
    source[Symbol.asyncIterator] = async function* () {
      try {
        const iterator = originalIterator();
        while (true) {
          const item = await iterator.next();
          if (item.done) break;
          yield item.value;
        }
      } finally {
        if (timingLog) appendFileSync(timingLog, `${JSON.stringify({ id, phase: "end", at: process.hrtime.bigint().toString() })}\n`);
      }
    };
    return source;
  };
}
