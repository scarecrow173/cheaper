import { mkdtemp, realpath, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { CheaperError, compileSchema } from "@cheaper/core";
import { ModelRuntime } from "@earendil-works/pi-coding-agent";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { repairOutput } from "../src/repair.js";
import { createFakePi, fauxAssistantMessage } from "./fixtures/fake-stream.js";

let dir: string;
let root: string;
let runtime: ModelRuntime;

beforeEach(async () => {
  dir = await mkdtemp(path.join(tmpdir(), "cheaper-repair-"));
  root = await realpath(dir);
  runtime = await ModelRuntime.create({ refreshOnCreate: false });
});
afterEach(async () => {
  await rm(dir, { recursive: true, force: true });
});

const schema = compileSchema(
  {
    type: "object",
    required: ["ok"],
    properties: { ok: { type: "boolean" } },
  },
  "output",
);

function repairDeps(fake: ReturnType<typeof createFakePi>, attempts: number) {
  runtime.registerNativeProvider(fake.provider);
  return {
    cwd: root,
    model: fake.model,
    modelRuntime: runtime,
    maxOutputTokens: 1024,
    schema,
    rawOutput: "broken output",
    errors: [{ instancePath: "/ok", message: "must be boolean" }],
    attempts,
    streamFn: fake.streamFn,
  };
}

describe("repairOutput", () => {
  it("returns the repaired result on the first attempt", async () => {
    const fake = createFakePi({
      responses: [fauxAssistantMessage('{"ok":true}')],
    });
    const r = await repairOutput(repairDeps(fake, 3));
    expect(r.result).toEqual({ ok: true });
    expect(r.repairs).toBe(1);
  });

  it("creates a session with no tools and sends raw output + errors", async () => {
    const fake = createFakePi({
      responses: [fauxAssistantMessage('{"ok":true}')],
    });
    await repairOutput(repairDeps(fake, 3));
    const call = fake.calls[0];
    // noTools:"all" → context.tools は空
    expect(call?.context.tools ?? []).toEqual([]);
    // systemPrompt に schema、user message に raw output と errors
    expect(call?.context.systemPrompt).toContain('"ok"');
    const userText = JSON.stringify(call?.context.messages);
    expect(userText).toContain("broken output");
    expect(userText).toContain("must be boolean");
  });

  it("retries until a valid result, counting repairs", async () => {
    const fake = createFakePi({
      responses: [fauxAssistantMessage("still bad"), fauxAssistantMessage('{"ok":false}')],
    });
    const r = await repairOutput(repairDeps(fake, 3));
    expect(r.result).toEqual({ ok: false });
    expect(r.repairs).toBe(2);
    expect(fake.state.callCount).toBe(2);
  });

  it("throws INVALID_OUTPUT with capped details when attempts are exhausted", async () => {
    const fake = createFakePi({
      responses: [fauxAssistantMessage("bad 1"), fauxAssistantMessage("bad 2")],
    });
    const err = await repairOutput(repairDeps(fake, 2)).then(
      () => undefined,
      (e: unknown) => e,
    );
    expect(err).toBeInstanceOf(CheaperError);
    const ce = err as CheaperError;
    expect(ce.code).toBe("INVALID_OUTPUT");
    const details = ce.details as { errors: unknown[]; rawOutput: string };
    expect(details.errors.length).toBeLessThanOrEqual(20);
    expect(details.rawOutput.length).toBeLessThanOrEqual(2000);
    expect(fake.state.callCount).toBe(2);
  });

  it("stops early when the outer signal is already aborted", async () => {
    const fake = createFakePi({
      responses: [fauxAssistantMessage("bad"), fauxAssistantMessage("bad")],
    });
    const ac = new AbortController();
    ac.abort();
    const err = await repairOutput({ ...repairDeps(fake, 3), signal: ac.signal }).then(
      () => undefined,
      (e: unknown) => e,
    );
    expect((err as CheaperError).code).toBe("INVALID_OUTPUT");
    expect(fake.state.callCount).toBe(0);
  });
});
