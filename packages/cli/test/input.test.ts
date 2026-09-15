import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { Readable } from "node:stream";
import type { CheaperError } from "@cheaper/core";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { readTaskInput } from "../src/input.js";

let dir: string;
beforeEach(async () => {
  dir = await mkdtemp(path.join(tmpdir(), "cheaper-cli-input-"));
});
afterEach(async () => {
  await rm(dir, { recursive: true, force: true });
});

function stdinOf(text: string, isTTY = false) {
  const s = Readable.from([text]);
  if (isTTY) (s as Readable & { isTTY: boolean }).isTTY = true;
  return s;
}

async function codeOf(p: Promise<unknown>): Promise<string> {
  try {
    await p;
  } catch (e) {
    return (e as CheaperError).code;
  }
  throw new Error("did not throw");
}

describe("readTaskInput", () => {
  it("reads a JSON object from --input file", async () => {
    const file = path.join(dir, "req.json");
    await writeFile(file, '{"task":"x"}');
    expect(await readTaskInput(file, stdinOf(""))).toEqual({ task: "x" });
  });

  it("reads a JSON object from stdin when input is '-'", async () => {
    expect(await readTaskInput("-", stdinOf('{"a":1}'))).toEqual({ a: 1 });
  });

  it("does not wait on TTY stdin and rejects empty input", async () => {
    expect(await codeOf(readTaskInput("-", stdinOf("unread", true)))).toBe("INVALID_REQUEST");
  });

  it("rejects empty piped stdin", async () => {
    expect(await codeOf(readTaskInput("-", stdinOf("   \n")))).toBe("INVALID_REQUEST");
  });

  it("rejects a missing input file", async () => {
    expect(await codeOf(readTaskInput(path.join(dir, "nope.json"), stdinOf("")))).toBe(
      "INVALID_REQUEST",
    );
  });

  it("rejects non-JSON input", async () => {
    expect(await codeOf(readTaskInput("-", stdinOf("not json")))).toBe("INVALID_REQUEST");
  });

  it("rejects JSON that is not an object", async () => {
    expect(await codeOf(readTaskInput("-", stdinOf("[1,2]")))).toBe("INVALID_REQUEST");
    expect(await codeOf(readTaskInput("-", stdinOf('"str"')))).toBe("INVALID_REQUEST");
    expect(await codeOf(readTaskInput("-", stdinOf("null")))).toBe("INVALID_REQUEST");
    expect(await codeOf(readTaskInput("-", stdinOf("42")))).toBe("INVALID_REQUEST");
  });
});
