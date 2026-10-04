import { mkdir, mkdtemp, realpath, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { createPathDenylist } from "@cheaper/core";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { loadWorkflowFile, type WorkflowDefinition } from "../src/index.js";
import { definition, fixtures } from "./fixtures.js";

describe("explicit workflow file loader", () => {
  let dir: string;
  let root: string;
  beforeEach(async () => {
    dir = await mkdtemp(path.join(tmpdir(), "workflow-loader-"));
    root = await realpath(dir);
  });
  afterEach(async () => {
    await rm(dir, { recursive: true, force: true });
  });

  function options(file = "workflow.json", denied: string[] = []) {
    return {
      root,
      path: file,
      symlinkPolicy: "within-workspace" as const,
      denylist: createPathDenylist(denied),
    };
  }
  async function save(raw: unknown) {
    await writeFile(path.join(root, "workflow.json"), JSON.stringify(raw));
  }

  it("loads a reusable definition without invocation values and compiles through the same compiler", async () => {
    const raw: WorkflowDefinition = {
      ...definition(),
      inputs: { target: { type: "string" }, config: { type: "object", default: { limit: 3 } } },
      jobs: { scan: { kind: "tool", tool: "read", input: { path: { $input: "target" } } } },
      outputs: { result: { schema: {}, value: { $from: "scan", path: "" } } },
    };
    await save(raw);
    const { compiler } = fixtures();
    const validate = vi.spyOn(compiler, "validateDefinition");
    const loaded = await loadWorkflowFile(options(), async () => compiler);
    expect(validate).toHaveBeenCalledWith(raw);
    expect(loaded.compiler).toBe(compiler);
    expect(loaded.definition).toEqual(raw);
    expect(compiler.projectInterface(loaded.definition).inputs.target?.required).toBe(true);
    await expect(loaded.compiler.compile(loaded.definition)).rejects.toMatchObject({
      code: "INVALID_INPUT",
    });
    const fromFile = await loaded.compiler.compile(loaded.definition, { target: "src" });
    const dynamic = await compiler.compile(raw, { target: "src" });
    expect(fromFile.inputs).toEqual(dynamic.inputs);
    expect(fromFile.jobs).toEqual(dynamic.jobs);
    expect(fromFile.order).toEqual(dynamic.order);
    expect(fromFile.edges).toEqual(dynamic.edges);
  });

  it.each([
    { ...definition(), schemaVersion: 2 },
    { ...definition(), inputs: { limit: { type: "integer", default: "bad" } } },
    {
      ...definition(),
      jobs: { a: { kind: "tool", tool: "read", input: { path: "x" }, needs: ["a"] } },
    },
    { ...definition(), outputs: { x: { schema: {}, value: { $input: "missing" } } } },
  ])("rejects invalid file definitions with canonical compiler semantics", async (raw) => {
    await save(raw);
    const { compiler } = fixtures();
    let directError: unknown;
    try {
      compiler.validateDefinition(raw);
    } catch (error) {
      directError = error;
    }
    expect(directError).toMatchObject({ code: "INVALID_REQUEST" });
    await expect(loadWorkflowFile(options(), () => compiler)).rejects.toEqual(directError);
  });

  it("maps read, non-file and parse errors before compiler construction", async () => {
    const factory = vi.fn(() => fixtures().compiler);
    await expect(loadWorkflowFile(options("missing.json"), factory)).rejects.toMatchObject({
      code: "INVALID_REQUEST",
      exitCode: 2,
    });
    await expect(loadWorkflowFile(options("."), factory)).rejects.toMatchObject({
      code: "INVALID_REQUEST",
    });
    await writeFile(path.join(root, "workflow.json"), "PRIVATE malformed JSON");
    await expect(loadWorkflowFile(options(), factory)).rejects.toMatchObject({
      code: "INVALID_INPUT",
      exitCode: 2,
    });
    expect(factory).not.toHaveBeenCalled();
  });

  it("rejects workspace escape, built-in deny and denied ancestors", async () => {
    const factory = vi.fn(() => fixtures().compiler);
    await save(definition());
    await mkdir(path.join(root, "blocked"));
    await writeFile(path.join(root, "blocked", "workflow.json"), JSON.stringify(definition()));
    for (const opts of [
      options("../workflow.json"),
      options(".env"),
      options("blocked/workflow.json", ["blocked"]),
    ]) {
      await expect(loadWorkflowFile(opts, factory)).rejects.toMatchObject({
        code: "INVALID_REQUEST",
      });
    }
    expect(factory).not.toHaveBeenCalled();
  });

  it("applies symlink policy and checks both logical and canonical deny paths", async () => {
    await mkdir(path.join(root, "target"));
    await writeFile(path.join(root, "target", "workflow.json"), JSON.stringify(definition()));
    await symlink(
      path.join(root, "target"),
      path.join(root, "alias"),
      process.platform === "win32" ? "junction" : "dir",
    );
    const factory = () => fixtures().compiler;
    await expect(loadWorkflowFile(options("alias/workflow.json"), factory)).resolves.toMatchObject({
      definition: definition(),
    });
    await expect(
      loadWorkflowFile({ ...options("alias/workflow.json"), symlinkPolicy: "deny" }, factory),
    ).rejects.toMatchObject({ code: "INVALID_REQUEST" });
    for (const denied of ["alias", "target"]) {
      await expect(
        loadWorkflowFile(options("alias/workflow.json", [denied]), factory),
      ).rejects.toMatchObject({ code: "INVALID_REQUEST" });
    }
    await symlink(
      path.dirname(root),
      path.join(root, "outside"),
      process.platform === "win32" ? "junction" : "dir",
    );
    await expect(loadWorkflowFile(options("outside/workflow.json"), factory)).rejects.toMatchObject(
      { code: "INVALID_REQUEST" },
    );
  });

  it("exposes default/input validation without compiling jobs and shares compile semantics", async () => {
    const { compiler } = fixtures();
    const raw = {
      ...definition(),
      inputs: { target: { type: "string" }, config: { type: "object", default: { limit: 3 } } },
    };
    const inputs = compiler.validateInputs(raw, { target: "src" });
    expect(inputs).toEqual({ target: "src", config: { limit: 3 } });
    (inputs.config as Record<string, unknown>).limit = 99;
    expect(compiler.validateInputs(raw, { target: "src" })).toEqual(
      (await compiler.compile(raw, { target: "src" })).inputs,
    );
    for (const provided of [{}, { target: 1 }, { target: "x", unknown: true }, []]) {
      expect(() => compiler.validateInputs(raw, provided)).toThrowError(
        expect.objectContaining({ code: "INVALID_INPUT" }),
      );
      await expect(compiler.compile(raw, provided)).rejects.toMatchObject({
        code: "INVALID_INPUT",
      });
    }
  });
  it.runIf(process.platform === "win32")(
    "rejects case aliases of real denied files and ancestors on Windows",
    async () => {
      await mkdir(path.join(root, "blocked"));
      await writeFile(path.join(root, "blocked", "workflow.json"), JSON.stringify(definition()));
      await writeFile(path.join(root, ".env"), JSON.stringify(definition()));
      const factory = vi.fn(() => fixtures().compiler);
      for (const opts of [
        options("Blocked/workflow.json", ["blocked"]),
        options("blocked/workflow.json", ["BLOCKED"]),
        options(".ENV"),
      ]) {
        await expect(loadWorkflowFile(opts, factory)).rejects.toMatchObject({
          code: "INVALID_REQUEST",
        });
      }
      expect(factory).not.toHaveBeenCalled();
    },
  );
});
