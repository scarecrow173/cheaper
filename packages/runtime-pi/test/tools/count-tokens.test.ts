import { mkdtemp, realpath, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { countFileTokens } from "@cheaper/core";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { createCountTokensTool } from "../../src/tools/count-tokens.js";
import { ACCESS_DENIED_MESSAGE } from "../../src/tools/walker.js";

let dir: string;
let root: string;

beforeEach(async () => {
  dir = await mkdtemp(path.join(tmpdir(), "cheaper-ct-"));
  root = await realpath(dir);
});
afterEach(async () => {
  await rm(dir, { recursive: true, force: true });
});

async function execute(tool: ReturnType<typeof createCountTokensTool>, p: { path: string }) {
  return tool.execute("c1", p as never, undefined, undefined, undefined as never);
}

describe("count_tokens tool", () => {
  it("returns the same canonical count as the core implementation", async () => {
    await writeFile(path.join(root, "a.txt"), "hello world\n", "utf8");
    const tool = createCountTokensTool({ root, symlinkPolicy: "deny" });
    const result = await execute(tool, { path: "a.txt" });
    const text = result.content
      .filter((c): c is { type: "text"; text: string } => c.type === "text")
      .map((c) => c.text)
      .join("");
    const parsed = JSON.parse(text) as {
      path: string;
      tokens: number;
      bytes: number;
      tokenizer: { name: string; implementation: string; version: string };
    };
    const direct = await countFileTokens({ root, path: "a.txt", symlinkPolicy: "deny" });
    expect(parsed.path).toBe("a.txt");
    expect(parsed.tokens).toBe(direct.tokens);
    expect(parsed.bytes).toBe(direct.bytes);
    expect(parsed.tokenizer).toEqual(direct.tokenizer);
    // 本文を含まない
    expect(text).not.toContain("hello world");
  });

  it("denies workspace escapes", async () => {
    const tool = createCountTokensTool({ root, symlinkPolicy: "deny" });
    await expect(execute(tool, { path: "../out.txt" })).rejects.toThrow(ACCESS_DENIED_MESSAGE);
    await expect(execute(tool, { path: "/etc/passwd" })).rejects.toThrow(ACCESS_DENIED_MESSAGE);
  });
});
