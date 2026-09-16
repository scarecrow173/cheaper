import path from "node:path";
import { fileURLToPath } from "node:url";
import { type ConfigDirs, resolveWorker, validateWorkerInput } from "@cheaper/core";
import { describe, expect, it } from "vitest";

const cliRoot = fileURLToPath(new URL("..", import.meta.url));
const dirs: ConfigDirs = {
  userConfigDir: path.join(cliRoot, "no-such-user-config"),
  projectDir: path.join(cliRoot, "no-such-project"),
  builtinDir: cliRoot,
};

describe("built-in worker definitions", () => {
  it("bulk-reader resolves through core validate with read-only tools", async () => {
    const def = await resolveWorker("bulk-reader", dirs);
    expect(def.origin).toBe("builtin");
    expect(def.tools).toEqual(["read", "grep", "find", "ls", "count_tokens"]);
    expect(def.tools).not.toContain("write");
    expect(def.tools).not.toContain("edit");
    expect(def.cheaper.model.tier).toBe("low");
    expect(def.inputSchema).toBeDefined();
    expect(def.outputSchema).toBeDefined();
  });

  it("code-writer resolves through core validate with write tools", async () => {
    const def = await resolveWorker("code-writer", dirs);
    expect(def.origin).toBe("builtin");
    expect(def.tools).toEqual(["read", "grep", "find", "ls", "write", "edit", "count_tokens"]);
    expect(def.cheaper.model.tier).toBe("low");
    expect(def.inputSchema).toBeDefined();
    expect(def.outputSchema).toBeDefined();
  });

  it("doc-writer resolves through core validate with write tools", async () => {
    const def = await resolveWorker("doc-writer", dirs);
    expect(def.origin).toBe("builtin");
    expect(def.tools).toEqual(["read", "grep", "find", "ls", "write", "edit", "count_tokens"]);
    expect(def.cheaper.model.tier).toBe("low");
    expect(def.inputSchema).toBeDefined();
    expect(def.outputSchema).toBeDefined();
  });

  it("the removed cheap-writer worker no longer resolves", async () => {
    await expect(resolveWorker("cheap-writer", dirs)).rejects.toThrowError();
  });

  it("bulk-reader input schema accepts a minimal input and rejects violations", async () => {
    const def = await resolveWorker("bulk-reader", dirs);
    expect(() => validateWorkerInput(def, { task: "what does x do" })).not.toThrow();
    expect(() =>
      validateWorkerInput(def, {
        task: "t",
        paths: ["src/**/*.ts"],
        hints: ["sym"],
        maxFindings: 5,
      }),
    ).not.toThrow();
    expect(() => validateWorkerInput(def, {})).toThrowError(/input schema/);
    expect(() => validateWorkerInput(def, { task: "t", paths: ["../out"] })).toThrowError(
      /input schema/,
    );
    expect(() => validateWorkerInput(def, { task: "t", maxFindings: 99 })).toThrowError(
      /input schema/,
    );
  });

  it("code-writer input schema enforces required targets", async () => {
    const def = await resolveWorker("code-writer", dirs);
    expect(() =>
      validateWorkerInput(def, {
        task: "add tests",
        targets: [{ path: "a.test.ts", intent: "create" }],
      }),
    ).not.toThrow();
    expect(() => validateWorkerInput(def, { task: "x" })).toThrowError(/input schema/);
    expect(() =>
      validateWorkerInput(def, {
        task: "x",
        targets: [{ path: "a.ts", intent: "destroy" }],
      }),
    ).toThrowError(/input schema/);
    expect(() => validateWorkerInput(def, { task: "x", targets: [] })).toThrowError(/input schema/);
  });

  it("bulk-reader output schema validates a conforming result", async () => {
    const def = await resolveWorker("bulk-reader", dirs);
    const ok = def.outputSchema.validate({
      summary: "s",
      findings: [
        {
          title: "t",
          detail: "d",
          confidence: "high",
          locations: [{ path: "src/a.ts", lines: { start: 1, end: 3 }, excerpt: "x" }],
        },
      ],
      relevantFiles: [{ path: "src/a.ts", reason: "r" }],
      suggestedReads: [{ path: "src/a.ts", lines: { start: 1, end: 10 }, reason: "r" }],
      openQuestions: ["q"],
    });
    expect(ok.ok).toBe(true);
    const bad = def.outputSchema.validate({
      summary: "s",
      findings: [],
      relevantFiles: [],
      suggestedReads: [{ path: "/abs", reason: "r" }],
      openQuestions: [],
    });
    expect(bad.ok).toBe(false);
  });

  it("doc-writer input schema requires documentType and accepts decisions/facts", async () => {
    const def = await resolveWorker("doc-writer", dirs);
    expect(() =>
      validateWorkerInput(def, {
        task: "write an ADR for the tokenizer choice",
        documentType: "adr",
        targets: [{ path: "docs/adr/0011-tokenizer.md", intent: "create" }],
        decisions: ["use js-tiktoken as the canonical tokenizer"],
        facts: ["no tokenizer exists in the dependency tree"],
        references: ["docs/specs/0001-cheaper-v1-worker-runtime-cli.md"],
        constraints: ["follow MADR format"],
      }),
    ).not.toThrow();
    // documentType は必須
    expect(() =>
      validateWorkerInput(def, {
        task: "x",
        targets: [{ path: "a.md", intent: "create" }],
      }),
    ).toThrowError(/input schema/);
    // documentType の enum 外は拒否
    expect(() =>
      validateWorkerInput(def, {
        task: "x",
        documentType: "poem",
        targets: [{ path: "a.md", intent: "create" }],
      }),
    ).toThrowError(/input schema/);
    // workspace 外 path は拒否
    expect(() =>
      validateWorkerInput(def, {
        task: "x",
        documentType: "guide",
        targets: [{ path: "../a.md", intent: "create" }],
      }),
    ).toThrowError(/input schema/);
  });

  it("code-writer output schema has no changed-files field", async () => {
    const def = await resolveWorker("code-writer", dirs);
    const ok = def.outputSchema.validate({ summary: "s", notes: [], unresolved: [] });
    expect(ok.ok).toBe(true);
    const bad = def.outputSchema.validate({
      summary: "s",
      notes: [],
      unresolved: [],
      changedFiles: ["a.ts"],
    });
    expect(bad.ok).toBe(false);
  });
});
