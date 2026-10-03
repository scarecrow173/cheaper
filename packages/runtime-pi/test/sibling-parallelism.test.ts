import { mkdtemp, readFile, realpath, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { defineTool, ModelRuntime, type ToolDefinition } from "@earendil-works/pi-coding-agent";
import { Type } from "typebox";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { MutationJournal } from "../src/journal.js";
import { createWorkerSession, type WorkerSessionOptions } from "../src/session.js";
import { createSandboxedTools } from "../src/tools/registry.js";
import { createFakePi, fauxAssistantMessage, fauxToolCall } from "./fixtures/fake-stream.js";

let dir: string;
let root: string;
let runtime: ModelRuntime;

beforeEach(async () => {
  dir = await mkdtemp(path.join(tmpdir(), "cheaper-sibling-"));
  root = await realpath(dir);
  runtime = await ModelRuntime.create({ refreshOnCreate: false });
});
afterEach(async () => {
  await rm(dir, { recursive: true, force: true });
});

const GATE_DEADLINE_MS = 1_000;

function deferred() {
  let resolve!: () => void;
  const promise = new Promise<void>((res) => {
    resolve = res;
  });
  return { promise, resolve };
}

/** deadline 付き wait（unbounded await による hang を防ぐ）。 */
function withDeadline(promise: Promise<void>): Promise<void> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  const deadline = new Promise<never>((_, rej) => {
    timer = setTimeout(() => rej(new Error("gate deadline exceeded")), GATE_DEADLINE_MS);
  });
  // gate 早期解決時に timer が残らないよう確実に解除する
  return Promise.race([promise, deadline]).finally(() => clearTimeout(timer));
}

async function makeSession(
  fake: ReturnType<typeof createFakePi>,
  tools: ToolDefinition[],
  toolNames: string[],
  opts: Partial<WorkerSessionOptions> = {},
) {
  runtime.registerNativeProvider(fake.provider);
  return createWorkerSession({
    cwd: root,
    model: fake.model,
    modelRuntime: runtime,
    systemPrompt: "You are a test worker.",
    customTools: tools,
    toolNames,
    maxOutputTokens: 512,
    transientRetries: 0,
    streamFn: fake.streamFn,
    ...opts,
  });
}

function siblingBatch(names: string[]) {
  return fauxAssistantMessage(
    names.map((name) => fauxToolCall(name, {})),
    { stopReason: "toolUse" },
  );
}

// instrumented probe tool の `parameters` は `Type.Object({})` 必須 —
// schema 不適合の call は prepareToolCall が kind: "immediate" を返して
// execute に到達せず、「直列実行」と誤診する。
describe("sibling tool-call parallelism (pinned pi-agent-core@0.85.1)", () => {
  it("executes N sibling tool calls concurrently (bounded gate)", { timeout: 10_000 }, async () => {
    const started: string[] = [];
    const gate = deferred();
    const gatedProbe = (name: string): ToolDefinition =>
      defineTool({
        name,
        label: name,
        description: `deterministic probe tool ${name}`,
        parameters: Type.Object({}),
        async execute() {
          started.push(name);
          if (started.length >= 3) gate.resolve();
          await withDeadline(gate.promise);
          return {
            content: [{ type: "text" as const, text: `${name} ok` }],
            details: undefined,
          };
        },
      });
    const names = ["probe_1", "probe_2", "probe_3"];
    const fake = createFakePi({
      responses: [siblingBatch(names), fauxAssistantMessage('{"done":true}')],
    });
    const events: string[] = [];
    const { session } = await makeSession(fake, names.map(gatedProbe), names);
    session.subscribe((e) => {
      if (e.type === "tool_execution_start" || e.type === "tool_execution_end") {
        events.push(`${e.type}:${e.toolName}`);
      }
    });
    await session.prompt("go");

    // 全 call が execute に到達（kind: "prepared"）したことの確認。
    // immediate 発生時は fixture 不備として区別する。
    expect(started).toHaveLength(3);
    // 補助 assert: prepared-call 限定で全 start が最初の end より前に揃う
    // （sequential path は start/end 交互になる）。
    const firstEnd = events.findIndex((e) => e.startsWith("tool_execution_end:"));
    const starts = events
      .map((e, i) => ({ e, i }))
      .filter((x) => x.e.startsWith("tool_execution_start:"));
    expect(firstEnd).toBeGreaterThan(-1);
    expect(starts).toHaveLength(3);
    expect(starts.at(-1)?.i).toBeLessThan(firstEnd);
    // NOTE: 「全 N 個同時 in-flight」は現行 Pi の unbounded Promise.all を pin
    // する意図的に厳しい基準。将来 Pi が batch 並行度 cap を導入した場合は
    // 「少なくとも 2 個の overlap」への緩和を要する。
    await session.dispose();
  });

  it("emits toolResult messages in original call order, not completion order", {
    timeout: 10_000,
  }, async () => {
    // reverse-completion gate: probe_i は probe_{i+1} の完了を待つため、
    // 並列実行下で完了順が確定的に probe_3 → probe_2 → probe_1 になる。
    // 直列 regression 時は probe_1 が unreachable marker を待って
    // deadline で clean fail する（sleep 不使用）。
    const completions: string[] = [];
    const done2 = deferred();
    const done3 = deferred();
    const orderedProbe = (name: string, waitFor?: Promise<void>, done?: () => void) =>
      defineTool({
        name,
        label: name,
        description: `deterministic probe tool ${name}`,
        parameters: Type.Object({}),
        async execute() {
          if (waitFor) await withDeadline(waitFor);
          completions.push(name);
          done?.();
          return {
            content: [{ type: "text" as const, text: `${name} ok` }],
            details: undefined,
          };
        },
      });
    const names = ["probe_1", "probe_2", "probe_3"];
    const tools = [
      orderedProbe("probe_1", done2.promise),
      orderedProbe("probe_2", done3.promise, done2.resolve),
      orderedProbe("probe_3", undefined, done3.resolve),
    ];
    const fake = createFakePi({
      responses: [siblingBatch(names), fauxAssistantMessage('{"done":true}')],
    });
    const { session } = await makeSession(fake, tools, names);
    await session.prompt("go");

    // 完了順は確定的に逆順であることを前提確認（assert 対象の識別力）。
    expect(completions).toEqual(["probe_3", "probe_2", "probe_1"]);
    // toolResult message は call 順で emit される（tool_execution_end
    // event は完了順のため観測対象は messages のみ）。
    const toolResults = session.messages
      .filter((m) => m.role === "toolResult")
      .map((m) => m.toolName);
    expect(toolResults).toEqual(names);
    await session.dispose();
  });

  it('fails every sibling call unexecuted when stopReason is "length"', {
    timeout: 10_000,
  }, async () => {
    const started: string[] = [];
    const countingProbe = (name: string): ToolDefinition =>
      defineTool({
        name,
        label: name,
        description: `deterministic probe tool ${name}`,
        parameters: Type.Object({}),
        async execute() {
          started.push(name);
          return {
            content: [{ type: "text" as const, text: `${name} ok` }],
            details: undefined,
          };
        },
      });
    const names = ["probe_1", "probe_2", "probe_3"];
    const ends: { toolName: string; isError: boolean }[] = [];
    const fake = createFakePi({
      responses: [
        fauxAssistantMessage(
          names.map((name) => fauxToolCall(name, {})),
          { stopReason: "length" },
        ),
        fauxAssistantMessage('{"done":true}'),
      ],
    });
    const { session } = await makeSession(fake, names.map(countingProbe), names);
    session.subscribe((e) => {
      if (e.type === "tool_execution_end") {
        ends.push({ toolName: e.toolName, isError: e.isError });
      }
    });
    await session.prompt("go");

    // truncated assistant message 由来の sibling call は全員未実行のまま
    // error result になる（execute は一度も呼ばれない）。
    expect(started).toHaveLength(0);
    const toolResults = session.messages.filter((m) => m.role === "toolResult");
    expect(toolResults).toHaveLength(3);
    expect(toolResults.every((m) => m.isError)).toBe(true);
    expect(ends).toHaveLength(3);
    expect(ends.every((e) => e.isError)).toBe(true);
    await session.dispose();
  });

  it("serializes sibling write→edit on the same file in call order (withFileMutationQueue)", {
    timeout: 10_000,
  }, async () => {
    // 実 sandboxed write/edit を通す withFileMutationQueue 継承 pin。
    // edit の oldText "hello" は write の結果にしか存在しないため、
    // queue が機能していれば両 call 成功 + 最終内容 "world" になる。
    // 検出の非対称性: queue regression 時、write の writeFile が edit の
    // access より先に完了すれば edit が成功して false-PASS し得る
    // （PASS 経路は決定的、regression 検出側は timing 依存）。強化する
    // 場合は createWriteOperations/createEditOperations seam 経由で
    // op 呼出順を記録し、write.writeFile 完了が edit.access 開始に
    // 先行することを assert する。
    const journal = new MutationJournal();
    const sandboxed = createSandboxedTools({
      root,
      symlinkPolicy: "deny",
      journal,
      names: ["write", "edit"],
      maxToolCalls: 50,
      onLimit: () => {},
    });
    const fake = createFakePi({
      responses: [
        fauxAssistantMessage(
          [
            fauxToolCall("write", { path: "f.txt", content: "hello" }),
            fauxToolCall("edit", {
              path: "f.txt",
              edits: [{ oldText: "hello", newText: "world" }],
            }),
          ],
          { stopReason: "toolUse" },
        ),
        fauxAssistantMessage('{"done":true}'),
      ],
    });
    const { session } = await makeSession(fake, sandboxed.tools, sandboxed.toolNames);
    await session.prompt("go");

    const toolResults = session.messages.filter((m) => m.role === "toolResult");
    expect(toolResults).toHaveLength(2);
    expect(toolResults.every((m) => !m.isError)).toBe(true);
    expect(await readFile(path.join(root, "f.txt"), "utf8")).toBe("world");
    await session.dispose();
  });
});
