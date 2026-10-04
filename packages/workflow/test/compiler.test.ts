import { readFile } from "node:fs/promises";
import { compileSchema } from "@cheaper/core";
import { describe, expect, it } from "vitest";
import type { JsonValue, ResourceInference, WorkflowJobState } from "../src/index.js";
import { definition, fixtures } from "./fixtures.js";

describe("public inputs/defaults", () => {
  it("root defaults と JSON 型を保ち invocation object を変更/共有しない", async () => {
    const raw = definition();
    raw.inputs = {
      object: { type: "object", default: { nested: [1] } },
      flag: { type: "boolean", default: true },
      number: { type: "number", default: 3 },
      nil: { type: ["string", "null"], default: null },
      provided: { type: "array" },
    };
    raw.jobs.a = {
      kind: "tool",
      tool: "grep",
      input: Object.fromEntries(Object.keys(raw.inputs).map((name) => [name, { $input: name }])),
    };
    const input = { provided: ["caller"] };
    const { compiler } = fixtures();
    const a = await compiler.compile(raw, input);
    const b = await compiler.compile(raw, input);
    expect(a.jobs.a?.input).toEqual({
      object: { nested: [1] },
      flag: true,
      number: 3,
      nil: null,
      provided: ["caller"],
    });
    (a.inputs.object as { nested: number[] }).nested.push(2);
    expect(b.inputs.object).toEqual({ nested: [1] });
    expect(raw.inputs.object?.default).toEqual({ nested: [1] });
    expect(input).toEqual({ provided: ["caller"] });
    expect(a.materializeJobInput("a", new Map())).toEqual(b.jobs.a?.input);
  });

  it.each([{}, { x: "wrong" }, { x: 1, unknown: true }, null, [], "x"])(
    "missing/invalid/unknown/非 object を INVALID_INPUT: %j",
    async (input) => {
      const raw = definition();
      raw.inputs.x = { type: "number" };
      await expect(fixtures().compiler.compile(raw, input)).rejects.toMatchObject({
        code: "INVALID_INPUT",
      });
    },
  );
  it("invalid default は invocation 無しでも definition error", () => {
    const raw = definition();
    raw.inputs.x = { type: "number", default: "bad" };
    expect(() => fixtures().compiler.projectInterface(raw)).toThrow(
      expect.objectContaining({ code: "INVALID_REQUEST" }),
    );
  });
  it("nested defaults は補完しない", async () => {
    const raw = definition();
    raw.inputs.x = {
      type: "object",
      required: ["nested"],
      properties: { nested: { type: "number", default: 1 } },
    };
    await expect(fixtures().compiler.compile(raw, { x: {} })).rejects.toMatchObject({
      code: "INVALID_INPUT",
    });
  });
  it("local $defs は core 2020-12 semantics を使う", async () => {
    const raw = definition();
    raw.inputs.x = {
      $defs: { value: { type: "integer", minimum: 2 } },
      $ref: "#/$defs/value",
      default: 2,
    };
    expect((await fixtures().compiler.compile(raw)).inputs).toEqual({ x: 2 });
    await expect(fixtures().compiler.compile(raw, { x: 1 })).rejects.toMatchObject({
      code: "INVALID_INPUT",
    });
  });
  it("external $ref を fetch せず cause を保持して wrap", () => {
    const raw = definition();
    raw.inputs.x = { $ref: "https://example.invalid/schema.json" };
    expect(() => fixtures().compiler.validateDefinition(raw)).toThrow(
      expect.objectContaining({
        code: "INVALID_REQUEST",
        cause: expect.objectContaining({ code: "INVALID_INPUT_SCHEMA" }),
      }),
    );
  });
  it("undefined/NaN/cycle/class/accessor を JSON 境界で拒否", async () => {
    const raw = definition();
    raw.inputs.x = {};
    const cycle: Record<string, unknown> = {};
    cycle.self = cycle;
    for (const x of [
      undefined,
      Number.NaN,
      cycle,
      new Date(),
      {
        get surprise() {
          throw new Error("getter executed");
        },
      },
    ]) {
      await expect(fixtures().compiler.compile(raw, { x })).rejects.toMatchObject({
        code: "INVALID_INPUT",
      });
    }
  });
  it("prototype と衝突する own key を安全に扱う", async () => {
    const raw = definition();
    raw.inputs = JSON.parse(
      '{"__proto__":{"type":"number","default":1},"constructor":{"type":"number","default":2}}',
    );
    raw.jobs.a = { kind: "tool", tool: "grep", input: { $input: "__proto__" } };
    const compiled = await fixtures().compiler.compile(raw);
    expect(Object.hasOwn(compiled.inputs, "__proto__")).toBe(true);
    expect(compiled.materializeJobInput("a", new Map())).toBe(1);
  });
});

describe("definition と deterministic DAG", () => {
  it("needs/from edge dedupe と definition 順に依存しない JSON 表現", async () => {
    const raw = definition();
    raw.jobs = {
      z: {
        kind: "tool",
        tool: "grep",
        input: { $from: "a", path: "/content" },
        needs: ["b", "a", "a"],
      },
      b: { kind: "tool", tool: "grep", input: {} },
      a: { kind: "tool", tool: "grep", input: {} },
    };
    const reversed = structuredClone(raw);
    reversed.jobs = Object.fromEntries(Object.entries(reversed.jobs).reverse());
    reversed.jobs.z!.needs = ["a", "b"];
    const a = await fixtures().compiler.compile(raw);
    const b = await fixtures().compiler.compile(reversed);
    expect(a.order).toEqual(["a", "b", "z"]);
    expect(a.edges).toEqual([
      { from: "a", to: "z" },
      { from: "b", to: "z" },
    ]);
    expect(JSON.stringify(a)).toBe(JSON.stringify(b));
    expect(Object.keys(a.jobs)).toEqual(["a", "b", "z"]);
  });
  it("default/literal の偽 from を edge にせず job hook でも再解釈しない", async () => {
    const raw = definition();
    raw.inputs.x = { type: "object", default: { $from: "ghost", path: "" } };
    raw.jobs.a = {
      kind: "tool",
      tool: "grep",
      input: { x: { $input: "x" }, y: { $literal: { $from: "ghost", path: "" } } },
    };
    const compiled = await fixtures().compiler.compile(raw);
    expect(compiled.edges).toEqual([]);
    expect(compiled.materializeJobInput("a", new Map())).toEqual({
      x: { $from: "ghost", path: "" },
      y: { $from: "ghost", path: "" },
    });
  });
  it.each([
    { kind: "tool", tool: "ghost", input: {} },
    { kind: "worker", worker: "ghost", input: {} },
    { kind: "tool", tool: "grep", input: { $input: "ghost" } },
    { kind: "tool", tool: "grep", input: { $from: "ghost", path: "" } },
    { kind: "tool", tool: "grep", input: {}, needs: ["a"] },
    { kind: "tool", tool: "grep", input: {}, needs: ["unknown"] },
    { kind: "tool", tool: "grep", input: {}, extra: 1 },
    {
      kind: "tool",
      tool: "grep",
      input: {},
      resources: { reads: [{ path: { $from: "a", path: "" }, scope: "exact" }] },
    },
  ])("不正 job/reference/resource を INVALID_REQUEST: %j", (job) => {
    const raw = { ...definition(), jobs: { a: job } };
    expect(() => fixtures().compiler.validateDefinition(raw)).toThrow(
      expect.objectContaining({ code: "INVALID_REQUEST" }),
    );
  });
  it("unknown top-level/output fields と output 一般 object は拒否", () => {
    for (const raw of [
      { ...definition(), extra: true },
      { ...definition(), outputs: { x: { schema: {}, value: {}, extra: 1 } } },
    ]) {
      expect(() => fixtures().compiler.validateDefinition(raw)).toThrow(
        expect.objectContaining({ code: "INVALID_REQUEST" }),
      );
    }
  });
  it("cycle は invocation 不要の definition validation でも拒否", () => {
    const raw = definition();
    raw.jobs.a = { kind: "tool", tool: "grep", input: {}, needs: ["b"] };
    raw.jobs.b = { kind: "tool", tool: "grep", input: {}, needs: ["a"] };
    expect(() => fixtures().compiler.validateDefinition(raw)).toThrow(
      expect.objectContaining({ code: "INVALID_REQUEST" }),
    );
  });
  it("Worker capability の欠落 tool を catalog で拒否", () => {
    const { compiler, worker } = fixtures();
    worker.tools = ["write"];
    const raw = definition();
    raw.jobs.a = { kind: "worker", worker: "bulk-reader", input: { task: "x" } };
    expect(() => compiler.validateDefinition(raw)).toThrow(
      expect.objectContaining({ code: "INVALID_REQUEST" }),
    );
  });
  it("明確に不可能な pointer は compile 前、open/union/ref は runtime", async () => {
    const raw = definition();
    raw.jobs.a = { kind: "tool", tool: "read", input: { path: "src/a" } };
    raw.outputs.x = { schema: {}, value: { $from: "a", path: "/no-such-property" } };
    expect(() => fixtures().compiler.validateDefinition(raw)).toThrow(
      expect.objectContaining({ code: "INVALID_REQUEST" }),
    );
    raw.outputs.x.value = { $from: "a", path: "/details/not-statically-known" };
    expect((await fixtures().compiler.compile(raw)).outputs.x).toBeDefined();
  });
});

describe("job input の最終 validation と snapshot", () => {
  it.each([
    { kind: "tool", tool: "read", input: { path: 7 } },
    { kind: "worker", worker: "bulk-reader", input: { task: 7 } },
  ])("from 無し input は compile 時 full validation: %j", async (job) => {
    await expect(
      fixtures().compiler.compile({ ...definition(), jobs: { a: job } }),
    ).rejects.toMatchObject({ code: "INVALID_INPUT" });
  });
  it.each(["tool", "worker"] as const)(
    "from 有り %s input は deferred、解決後 validation を skip しない",
    async (kind) => {
      const raw = definition();
      raw.jobs.source = { kind: "tool", tool: "grep", input: {} };
      raw.jobs.consumer =
        kind === "tool"
          ? { kind, tool: "read", input: { path: { $from: "source", path: "/details/value" } } }
          : {
              kind,
              worker: "bulk-reader",
              input: { task: { $from: "source", path: "/details/value" } },
            };
      const compiled = await fixtures().compiler.compile(raw);
      const results = new Map<string, JsonValue>([
        ["source", { content: [], details: { value: 7 } }],
      ]);
      expect(() => compiled.materializeJobInput("consumer", results)).toThrow(
        expect.objectContaining({ code: "INVALID_INPUT" }),
      );
      results.set("source", { content: [], details: { value: "valid" } });
      expect(compiled.materializeJobInput("consumer", results)).toEqual(
        kind === "tool" ? { path: "valid" } : { task: "valid" },
      );
      expect(() => compiled.materializeJobInput("consumer", new Map())).toThrow(
        expect.objectContaining({ code: "INVALID_INPUT" }),
      );
      expect(() => compiled.materializeJobInput("ghost", results)).toThrow(
        expect.objectContaining({ code: "INVALID_REQUEST" }),
      );
    },
  );
  it("definition/invocation/display/catalog の mutation は validation snapshot を変更しない", async () => {
    const raw = definition();
    raw.inputs.x = { type: "string" };
    raw.jobs.a = { kind: "worker", worker: "bulk-reader", input: { task: { $input: "x" } } };
    const caller = { x: "original" };
    const { compiler, worker } = fixtures();
    const compiled = await compiler.compile(raw, caller);
    caller.x = "changed";
    raw.jobs.a.input = { task: 9 };
    compiled.inputs.x = false;
    compiled.jobs.a!.input = { task: false };
    worker.inputSchema = compileSchema({}, "input");
    expect(compiled.materializeJobInput("a", new Map())).toEqual({ task: "original" });
  });
});

describe("全 terminal 後の output hooks", () => {
  it("readiness、schema、missing と partial output の単一/batch 差", async () => {
    const raw = definition();
    raw.jobs.a = { kind: "tool", tool: "grep", input: {} };
    raw.jobs.failed = { kind: "tool", tool: "grep", input: {} };
    raw.outputs = {
      good: { schema: { type: "number" }, value: { $from: "a", path: "/details/value" } },
      bad: { schema: {}, value: { $from: "failed", path: "" } },
      constant: { schema: { type: "string" }, value: { $literal: "public" } },
    };
    const compiled = await fixtures().compiler.compile(raw);
    const states = new Map<string, WorkflowJobState>([
      ["a", { terminal: true, result: { content: [], details: { value: 5 } } }],
      ["failed", { terminal: false }],
    ]);
    expect(() => compiled.materializeOutput("constant", states)).toThrow(
      expect.objectContaining({ code: "INVALID_OUTPUT" }),
    );
    states.set("failed", { terminal: true });
    expect(compiled.materializeOutput("good", states)).toBe(5);
    expect(compiled.materializeOutput("constant", states)).toBe("public");
    expect(() => compiled.materializeOutput("bad", states)).toThrow(
      expect.objectContaining({ code: "INVALID_OUTPUT" }),
    );
    expect(() => compiled.materializeOutputs(states)).toThrow(
      expect.objectContaining({ code: "INVALID_OUTPUT" }),
    );
    states.set("failed", { terminal: true, result: null });
    expect(compiled.materializeOutputs(states)).toEqual({ bad: null, constant: "public", good: 5 });
    states.set("a", { terminal: true, result: { content: [], details: { value: "bad" } } });
    expect(() => compiled.materializeOutput("good", states)).toThrow(
      expect.objectContaining({ code: "INVALID_OUTPUT" }),
    );
    states.delete("a");
    expect(() => compiled.materializeOutputs(states)).toThrow(
      expect.objectContaining({ code: "INVALID_OUTPUT" }),
    );
  });
  it("public output schema/default と binding は snapshot、false JSON result は missing ではない", async () => {
    const raw = definition();
    raw.jobs.a = { kind: "tool", tool: "grep", input: {} };
    raw.outputs.x = { schema: { type: "boolean" }, value: { $from: "a", path: "" } };
    const compiled = await fixtures().compiler.compile(raw);
    compiled.outputs.x!.schema.type = "number";
    raw.outputs.x.value = { $literal: true };
    expect(
      compiled.materializeOutput("x", new Map([["a", { terminal: true, result: false }]])),
    ).toBe(false);
  });
  it("missing Pointer は output error、outputs が空でも全 job terminal を要求", async () => {
    const raw = definition();
    raw.jobs.a = { kind: "tool", tool: "grep", input: {} };
    const empty = await fixtures().compiler.compile(raw);
    expect(() => empty.materializeOutputs(new Map())).toThrow(
      expect.objectContaining({ code: "INVALID_OUTPUT" }),
    );
    expect(empty.materializeOutputs(new Map([["a", { terminal: true }]]))).toEqual({});
    raw.outputs.x = { schema: {}, value: { $from: "a", path: "/details/missing" } };
    const compiled = await fixtures().compiler.compile(raw);
    expect(() =>
      compiled.materializeOutput(
        "x",
        new Map([["a", { terminal: true, result: { content: [], details: {} } }]]),
      ),
    ).toThrow(expect.objectContaining({ code: "INVALID_OUTPUT" }));
  });
});

describe("resource 推論 port は未実装を成功にしない", () => {
  it("省略/{} は保持し manifest 無し、空配列は port 必須", async () => {
    const raw = definition();
    raw.jobs.a = { kind: "tool", tool: "grep", input: {} };
    raw.jobs.b = { kind: "tool", tool: "grep", input: {}, resources: {} };
    const compiled = await fixtures().compiler.compile(raw);
    expect(Object.hasOwn(compiled.jobs.a!, "resources")).toBe(false);
    expect(compiled.jobs.b!.resources).toEqual({});
    expect(Object.hasOwn(compiled.jobs.a!, "manifest")).toBe(false);
    raw.jobs.b.resources = { reads: [] };
    await expect(fixtures().compiler.compile(raw)).rejects.toMatchObject({
      code: "INVALID_REQUEST",
    });
  });
  it("async port に解決済み path と Worker metadata、全 manifest と辺を保持", async () => {
    const raw = definition();
    raw.inputs.path = { type: "string", default: "src" };
    raw.jobs.a = {
      kind: "tool",
      tool: "grep",
      input: {},
      resources: { produces: [{ path: { $input: "path" }, scope: "subtree" }] },
    };
    raw.jobs.b = {
      kind: "worker",
      worker: "bulk-reader",
      input: { task: "inspect" },
      resources: { reads: [] },
    };
    const manifests = {
      a: { reads: [], writes: [], produces: [] },
      b: { reads: [], writes: [], produces: [] },
    };
    const resources: ResourceInference = {
      async infer(jobs) {
        expect(jobs.a!.job.resources).toEqual({ produces: [{ path: "src", scope: "subtree" }] });
        expect(jobs.b!.worker?.name).toBe("bulk-reader");
        return { manifests, edges: [{ from: "a", to: "b" }] };
      },
    };
    const compiled = await fixtures({ resources }).compiler.compile(raw);
    expect(compiled.edges).toEqual([{ from: "a", to: "b" }]);
    expect(compiled.jobs.a!.manifest).toEqual(manifests.a);
    expect(compiled.jobs.b!.manifest).toEqual(manifests.b);
    expect(compiled.jobs.a!.manifest).not.toBe(manifests.a);
  });
  it("非 string resource path は INVALID_INPUT", async () => {
    const raw = definition();
    raw.inputs.path = { type: "number", default: 7 };
    raw.jobs.a = {
      kind: "tool",
      tool: "grep",
      input: {},
      resources: { reads: [{ path: { $input: "path" }, scope: "exact" }] },
    };
    await expect(fixtures().compiler.compile(raw)).rejects.toMatchObject({ code: "INVALID_INPUT" });
  });
  it("manifest 欠落、port throw、追加辺 self/unknown/cycle を拒否", async () => {
    const raw = definition();
    raw.jobs.a = { kind: "tool", tool: "grep", input: {}, resources: { reads: [] } };
    raw.jobs.b = { kind: "tool", tool: "grep", input: {}, needs: ["a"] };
    const manifests = {
      a: { reads: [], writes: [], produces: [] },
      b: { reads: [], writes: [], produces: [] },
    };
    await expect(
      fixtures({
        resources: { infer: () => ({ manifests: { a: manifests.a }, edges: [] }) },
      }).compiler.compile(raw),
    ).rejects.toMatchObject({ code: "INVALID_REQUEST" });
    const failure = new Error("normalization failed");
    await expect(
      fixtures({
        resources: {
          infer: () => {
            throw failure;
          },
        },
      }).compiler.compile(raw),
    ).rejects.toBe(failure);
    for (const edges of [
      [{ from: "a", to: "a" }],
      [{ from: "ghost", to: "b" }],
      [{ from: "b", to: "a" }],
    ]) {
      await expect(
        fixtures({ resources: { infer: () => ({ manifests, edges }) } }).compiler.compile(raw),
      ).rejects.toMatchObject({ code: "INVALID_REQUEST" });
    }
  });
});

it("compact fixture は実在 bulk-reader schema に適合する", async () => {
  const schema = JSON.parse(
    await readFile(
      new URL("../../cli/workers/schemas/bulk-reader.input.schema.json", import.meta.url),
      "utf8",
    ),
  );
  const { compiler, worker } = fixtures();
  worker.inputSchema = compileSchema(schema, "input");
  const raw = definition();
  raw.inputs.path = { type: "string", default: "src/index.ts" };
  raw.jobs = {
    read: { kind: "tool", tool: "read", input: { path: { $input: "path" } } },
    analyze: {
      kind: "worker",
      worker: "bulk-reader",
      input: { task: { $from: "read", path: "/content/0/text" } },
    },
  };
  raw.outputs.summary = {
    schema: { type: "string" },
    value: { $from: "analyze", path: "/summary" },
  };
  const compiled = await compiler.compile(raw);
  expect(
    compiled.materializeJobInput(
      "analyze",
      new Map([["read", { content: [{ type: "text", text: "Explain the source" }] }]]),
    ),
  ).toEqual({ task: "Explain the source" });
  expect(
    compiled.materializeOutput(
      "summary",
      new Map([
        ["read", { terminal: true }],
        ["analyze", { terminal: true, result: { summary: "public" } }],
      ]),
    ),
  ).toBe("public");
});
