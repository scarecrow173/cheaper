import { compileSchema, type WorkerDefinition } from "@cheaper/core";
import { describe, expect, it } from "vitest";
import { buildSystemPrompt } from "../src/run.js";

// buildSystemPrompt の system prompt 契約を pin する（DESIGN-034zAKcWuB8SuFkOePsubg）。
// 実 provider に流れる system prompt を構成するため、Worker 作成時の指示品質に
// 直接関わる。guidance bullet は toolNames 非空のときのみ emit される。
const outputSchema = compileSchema(
  { type: "object", required: ["ok"], properties: { ok: { type: "boolean" } } },
  "output",
);

function worker(): WorkerDefinition {
  return {
    name: "test-worker",
    description: "test",
    tools: ["read"],
    cheaper: {
      model: { tier: "low" },
      output: { schema: { inline: {} } },
    },
    instructions: "Do the task.",
    sourcePath: "test-worker.md",
    origin: "explicit",
    outputSchema,
  };
}

// DESIGN-034zAKcWuB8SuFkOePsubg が定める exact な英文 bullet（順序込み）
const GUIDANCE_BULLETS = [
  "- Independent tool calls must be issued together in the same assistant turn; do not serialize independent reads, searches, or token counting across multiple turns.",
  "- Do not poll or re-ask whether a call finished: the runtime owns timeouts and process lifecycle, and every tool result is returned automatically as input for the next turn.",
  "- Do not batch unsafe mutations: mutation calls that depend on each other or apply multiple mutations to the same target, and reads that expect the result of a sibling write or edit, are not independent — issue them in separate turns.",
  "- Keep each same-turn batch and its per-call arguments small enough to fit within the output token limit applied to each response; an oversized response is truncated and every tool call in it fails unexecuted.",
  "- This batching guidance does not change the final-message contract: the last message is still a single JSON object that validates against the output schema below.",
];

describe("buildSystemPrompt", () => {
  it("appends the output contract with available tools and schema", () => {
    const prompt = buildSystemPrompt(worker(), ["read", "write"]);
    expect(prompt.startsWith("Do the task.")).toBe(true);
    expect(prompt).toContain("- Available tools: read, write");
    expect(prompt).toContain("Output schema:");
    expect(prompt).toContain('"type": "object"');
    expect(prompt).toContain(
      "- The final message must be a single JSON object that validates against the output schema below.",
    );
  });

  it("lists no tools and emits no guidance when the allowlist is empty", () => {
    const prompt = buildSystemPrompt(worker(), []);
    expect(prompt).toContain("- Available tools: none");
    for (const bullet of GUIDANCE_BULLETS) {
      expect(prompt).not.toContain(bullet);
    }
  });

  it("emits same-turn batching guidance in order when tools are available (issue #55)", () => {
    const prompt = buildSystemPrompt(worker(), ["read", "write"]);
    let prev = -1;
    for (const bullet of GUIDANCE_BULLETS) {
      const idx = prompt.indexOf(bullet);
      expect(idx).toBeGreaterThan(prev);
      prev = idx;
    }
    // guidance は Available tools 行と Output schema 間に配置される
    const toolsIdx = prompt.indexOf("- Available tools:");
    const schemaIdx = prompt.indexOf("Output schema:");
    const guidanceIdx = prompt.indexOf("in the same assistant turn");
    expect(toolsIdx).toBeGreaterThanOrEqual(0);
    expect(guidanceIdx).toBeGreaterThan(toolsIdx);
    expect(guidanceIdx).toBeLessThan(schemaIdx);
  });

  it("keeps the output schema contract after guidance", () => {
    const prompt = buildSystemPrompt(worker(), ["read"]);
    const schemaIdx = prompt.indexOf("Output schema:");
    const guidanceIdx = prompt.indexOf("batching guidance does not change");
    expect(guidanceIdx).toBeGreaterThanOrEqual(0);
    expect(schemaIdx).toBeGreaterThan(guidanceIdx);
  });
});
