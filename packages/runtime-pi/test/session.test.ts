import { mkdtemp, realpath, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { ModelRuntime, SessionManager, SettingsManager } from "@earendil-works/pi-coding-agent";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { MutationJournal } from "../src/journal.js";
import { createWorkerSession } from "../src/session.js";
import { createSandboxedTools } from "../src/tools/registry.js";
import { createFakePi, fauxAssistantMessage, fauxToolCall } from "./fixtures/fake-stream.js";

let dir: string;
let root: string;
let runtime: ModelRuntime;

beforeEach(async () => {
  dir = await mkdtemp(path.join(tmpdir(), "cheaper-sess-"));
  root = await realpath(dir);
  runtime = await ModelRuntime.create({ refreshOnCreate: false });
});
afterEach(async () => {
  await rm(dir, { recursive: true, force: true });
});

async function makeSession(
  fake: ReturnType<typeof createFakePi>,
  opts: Partial<Parameters<typeof createWorkerSession>[0]> = {},
) {
  runtime.registerNativeProvider(fake.provider);
  const journal = new MutationJournal();
  const sandboxed = createSandboxedTools({
    root,
    symlinkPolicy: "deny",
    journal,
    names: ["read", "write", "grep", "find", "ls"],
    maxToolCalls: 50,
    onLimit: () => {},
  });
  return createWorkerSession({
    cwd: root,
    model: fake.model,
    modelRuntime: runtime,
    systemPrompt: "You are a test worker.",
    customTools: sandboxed.tools,
    toolNames: sandboxed.toolNames,
    maxOutputTokens: 512,
    transientRetries: 0,
    streamFn: fake.streamFn,
    ...opts,
  });
}

describe("createWorkerSession", () => {
  it("configures thinkingLevel off, in-memory managers, and disabled compaction", async () => {
    const fake = createFakePi({ responses: [fauxAssistantMessage("{}")] });
    const { session } = await makeSession(fake);
    expect(session.thinkingLevel).toBe("off");
    expect(session.sessionManager).toBeInstanceOf(SessionManager);
    expect(session.settingsManager).toBeInstanceOf(SettingsManager);
    expect(session.settingsManager.getCompactionSettings().enabled).toBe(false);
    expect(session.sessionFile).toBeUndefined(); // inMemory → 永続化無し
    await session.dispose();
  });

  it("registers exactly the allowlisted tools (no builtins)", async () => {
    const fake = createFakePi({ responses: [fauxAssistantMessage("{}")] });
    const { session } = await makeSession(fake);
    expect(session.getActiveToolNames().sort()).toEqual(
      ["find", "grep", "ls", "read", "write"].sort(),
    );
    await session.dispose();
  });

  it("exposes the worker system prompt through the resource loader", async () => {
    const fake = createFakePi({ responses: [fauxAssistantMessage("{}")] });
    const { session } = await makeSession(fake);
    expect(session.systemPrompt).toContain("You are a test worker.");
    await session.dispose();
  });

  it("injects maxTokens = min(policy, model.maxTokens) into stream options", async () => {
    const fake = createFakePi({ responses: [fauxAssistantMessage("{}")] });
    const { session, maxTokens } = await makeSession(fake, {
      maxOutputTokens: 512,
    });
    expect(maxTokens).toBe(512);
    const stream = session.agent.streamFunction(fake.model, { messages: [] }, {});
    for await (const _e of stream) void 0;
    expect(fake.calls[0]?.options?.maxTokens).toBe(512);
    await session.dispose();
  });

  it("clamps to model.maxTokens when policy allows more", async () => {
    const fake = createFakePi({ responses: [fauxAssistantMessage("{}")] });
    const { session, maxTokens } = await makeSession(fake, {
      maxOutputTokens: 1_000_000,
    });
    expect(maxTokens).toBe(fake.model.maxTokens);
    const stream = session.agent.streamFunction(fake.model, { messages: [] }, {});
    for await (const _e of stream) void 0;
    expect(fake.calls[0]?.options?.maxTokens).toBe(fake.model.maxTokens);
    await session.dispose();
  });

  it("runs prompt() end-to-end through the fake stream seam", async () => {
    const fake = createFakePi({
      responses: [fauxAssistantMessage('{"ok":true}')],
    });
    const { session } = await makeSession(fake);
    await session.prompt("Task input (JSON): {}");
    expect(fake.state.callCount).toBe(1);
    const last = session.messages.at(-1);
    expect(last?.role).toBe("assistant");
    await session.dispose();
  });

  it("executes sandboxed custom tools during a run", async () => {
    const fake = createFakePi({
      responses: [
        fauxAssistantMessage(fauxToolCall("write", { path: "w.txt", content: "hi" }), {
          stopReason: "toolUse",
        }),
        fauxAssistantMessage('{"done":true}'),
      ],
    });
    const { session } = await makeSession(fake);
    await session.prompt("go");
    const { readFile } = await import("node:fs/promises");
    expect(await readFile(path.join(root, "w.txt"), "utf8")).toBe("hi");
    await session.dispose();
  });

  it("enables retry settings when transientRetries > 0", async () => {
    const fake = createFakePi({ responses: [fauxAssistantMessage("{}")] });
    const { session } = await makeSession(fake, { transientRetries: 2 });
    const retry = session.settingsManager.getRetrySettings();
    expect(retry.enabled).toBe(true);
    expect(retry.maxRetries).toBe(2);
    await session.dispose();
  });
});
