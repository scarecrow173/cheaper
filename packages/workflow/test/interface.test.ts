import type { ToolMetadata } from "@cheaper/tool-runtime";
import { describe, expect, it, vi } from "vitest";
import { definition, fixtures } from "./fixtures.js";

describe("invocation 無しの public interface", () => {
  it("public schema/default/requiredness のみ、private DAG と execution を含めない", () => {
    const resourceInference = vi.fn(() => {
      throw new Error("must not execute");
    });
    const { compiler, ports } = fixtures({ resources: { infer: resourceInference } });
    const getTool = vi.spyOn(ports.tools, "get");
    const getWorker = vi.spyOn(ports.workers, "get");
    const list = vi.spyOn(ports.tools, "list");
    const raw = definition();
    raw.inputs.required = { type: "string" };
    raw.inputs.optional = { type: ["object", "null"], default: null };
    raw.jobs.secret = {
      kind: "worker",
      worker: "bulk-reader",
      input: { task: { $input: "required" } },
      resources: { reads: [] },
    };
    raw.outputs.public = { schema: { type: "string" }, value: { $literal: "PRIVATE BINDING" } };
    const projection = compiler.projectInterface(raw);
    expect(projection).toEqual({
      schemaVersion: 1,
      inputs: {
        optional: { schema: raw.inputs.optional, required: false },
        required: { schema: raw.inputs.required, required: true },
      },
      outputs: { public: { schema: { type: "string" } } },
    });
    const serialized = JSON.stringify(projection);
    for (const secret of [
      "secret",
      "jobs",
      "bindings",
      "resources",
      "PRIVATE PROMPT",
      "PRIVATE BINDING",
      "$input",
      "$literal",
    ])
      expect(serialized).not.toContain(secret);
    expect(resourceInference).not.toHaveBeenCalled();
    expect(list).not.toHaveBeenCalled();
    expect(getTool).toHaveBeenCalledWith("read");
    expect(getWorker).toHaveBeenCalledWith("bulk-reader");
    projection.inputs.required!.schema.type = "number";
    expect(raw.inputs.required?.type).toBe("string");
  });
  it("invocation 不要でも definition/reference/default エラーは拒否", () => {
    const raw = definition();
    raw.jobs.a = { kind: "tool", tool: "grep", input: { $input: "unknown" } };
    expect(() => fixtures().compiler.projectInterface(raw)).toThrow(
      expect.objectContaining({ code: "INVALID_REQUEST" }),
    );
  });
  it("process も tool metadata のみで compile し handler を実行しない", async () => {
    const { ports, tools } = fixtures();
    const execute = vi.fn(() => {
      throw new Error("must not spawn");
    });
    // core vocabulary に process が統合される前でも port 経由で検査できる。
    const get = ports.tools.get;
    ports.tools.get = (name) =>
      name === "process"
        ? { ...tools[0]!, name: "process" as ToolMetadata["name"], execute }
        : get(name);
    const raw = definition();
    raw.jobs.a = { kind: "tool", tool: "process", input: { path: "named-task" } };
    const { WorkflowCompiler } = await import("../src/index.js");
    const compiler = new WorkflowCompiler(ports);
    expect(compiler.projectInterface(raw)).toEqual({ schemaVersion: 1, inputs: {}, outputs: {} });
    expect((await compiler.compile(raw)).jobs.a?.kind).toBe("tool");
    expect(execute).not.toHaveBeenCalled();
    expect(() =>
      compiler.validateDefinition({ ...raw, jobs: { a: { kind: "process", input: {} } } }),
    ).toThrow(expect.objectContaining({ code: "INVALID_REQUEST" }));
  });
});
