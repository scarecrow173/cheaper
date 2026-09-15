import { describe, expect, it } from "vitest";
import type { CheaperError } from "../../src/errors.js";
import { parseWorkerFile } from "../../src/worker/parse.js";

const SRC = "workers/w.md";

function expectInvalid(content: string, reasonMatch?: RegExp) {
  let parsed: ReturnType<typeof parseWorkerFile> | undefined;
  try {
    parsed = parseWorkerFile(content, SRC);
  } catch (e) {
    const err = e as CheaperError;
    expect(err.code).toBe("INVALID_WORKER_DEFINITION");
    expect(err.details?.sourcePath).toBe(SRC);
    if (reasonMatch) expect(String(err.details?.reason)).toMatch(reasonMatch);
    return;
  }
  expect.unreachable(`expected INVALID_WORKER_DEFINITION, parsed: ${JSON.stringify(parsed)}`);
}

describe("parseWorkerFile", () => {
  it("parses frontmatter and trimmed body", () => {
    const r = parseWorkerFile("---\nname: w\ntools: [read]\n---\n\n Do things.\n", SRC);
    expect(r.frontmatter).toEqual({ name: "w", tools: ["read"] });
    expect(r.body).toBe("Do things.");
  });
  it("requires first-line ---", () => {
    expectInvalid("name: w\n---\nbody\n");
    expectInvalid("");
    expectInvalid(" ---\nname: w\n---\nbody\n");
  });
  it("requires closing ---", () => {
    expectInvalid("---\nname: w\nbody without close\n");
  });
  it("rejects YAML parse errors", () => {
    expectInvalid("---\nname: [unclosed\n---\nbody\n", /yaml|parse/i);
  });
  it("rejects non-object frontmatter", () => {
    expectInvalid("---\n- a\n- b\n---\nbody\n");
    expectInvalid("---\n\n---\nbody\n");
  });
  it("rejects empty body", () => {
    expectInvalid("---\nname: w\n---\n   \n", /body/i);
  });
  it("keeps --- lines inside the body", () => {
    const r = parseWorkerFile("---\nname: w\n---\nfirst\n---\nsecond\n", SRC);
    expect(r.body).toBe("first\n---\nsecond");
  });
});
