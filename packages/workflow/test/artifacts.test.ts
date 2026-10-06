import { mkdtemp, realpath, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import {
  type ArtifactPublisher,
  type ArtifactReader,
  type ArtifactReference,
  createInvocationToolRuntime,
  type ReadSource,
} from "@cheaper/tool-runtime";
import { describe, expect, it, vi } from "vitest";
import * as workflow from "../src/index.js";

const { RunArtifactStore, withArtifactStore, projectArtifactMetadata } = workflow;
const limits = {
  maxArtifactBytes: 32,
  maxTotalBytes: 48,
  maxPreviewBytes: 5,
  maxReadBytes: 8,
};
const producer = { jobId: "A1", name: "stdout" };
const source = (reference: ArtifactReference) => ({ kind: "artifact" as const, reference });

describe("run-scoped artifact contract", () => {
  it("borrows the canonical store across scopes and retains identity/caps/cleanup checks", async () => {
    const root = await realpath(await mkdtemp(path.join(tmpdir(), "cheaper-artifact-ports-")));
    const store = new RunArtifactStore(limits);
    const publisher: ArtifactPublisher = store;
    expect(publisher.maxArtifactBytes).toBe(limits.maxArtifactBytes);
    const runtime = createInvocationToolRuntime({
      root,
      symlinkPolicy: "deny",
      artifacts: { reader: store.reader, publisher },
    });
    const scope = (ownerId: string) =>
      runtime.createWorkerScope({
        root,
        symlinkPolicy: "deny",
        ownerId,
        names: ["read"],
        maxToolCalls: 100,
        onLimit: vi.fn(),
      });
    const first = scope("producer");
    const second = scope("consumer");
    try {
      await writeFile(path.join(root, "a.txt"), "hello world");
      const published = await first.execute("read", { path: "a.txt", artifact: "text" });
      const ref = (published.details as { artifact: ArtifactReference }).artifact;
      expect(published.content).toEqual([]);
      expect(ref.producer).toEqual({ jobId: "producer", name: "text" });
      const stats = store.stats;
      await expect(
        first.execute("read", { path: "a.txt", artifact: "text" }),
      ).rejects.toMatchObject({ code: "INVALID_INPUT" });
      expect(store.stats).toEqual(stats);
      await first.dispose();
      expect(store.stats.closed).toBe(false);
      const read = (reference: ArtifactReference, maxBytes = 3) =>
        second.execute("read", { source: source(reference), offset: 0, maxBytes });
      expect(await read(ref)).toEqual({
        content: [{ type: "text", text: "hel" }],
        details: { offset: 0, nextOffset: 3, eof: false },
      });
      await expect(read({ ...ref, preview: "tampered" })).rejects.toMatchObject({
        code: "INVALID_INPUT",
      });
      const foreign = new RunArtifactStore(limits);
      try {
        await expect(read(foreign.put({ producer, data: "foreign" }))).rejects.toMatchObject({
          code: "INVALID_INPUT",
        });
      } finally {
        foreign.dispose();
      }
      await expect(read(ref, 9)).rejects.toMatchObject({ code: "POLICY_VIOLATION" });
      await runtime.dispose();
      expect(store.stats.closed).toBe(false);
      expect(projectArtifactMetadata(ref).availability).toBe("unavailable");
      store.dispose();
      expect(() => store.reader.read(source(ref), { offset: 0, maxBytes: 3 })).toThrowError(
        expect.objectContaining({ code: "INVALID_REQUEST" }),
      );
    } finally {
      await runtime.dispose();
      store.dispose();
      await rm(root, { recursive: true, force: true });
    }
  });
  it("retains provenance and plain JSON metadata with an opaque artifact URI", () => {
    expect(RunArtifactStore).toBeTypeOf("function");
    const store = new RunArtifactStore(limits);
    const ref = store.put({ producer, data: "日本🙂ab" });
    expect(ref).toMatchObject({
      producer: { jobId: "A1", name: "stdout" },
      bytes: 12,
      preview: "日",
      truncated: true,
      runId: store.runId,
    });
    expect(ref.artifact).toMatch(/^artifact:\/\/[a-f0-9-]{36}\/[a-f0-9-]{36}$/);
    expect(ref.artifact).not.toContain("stdout");
    expect(Object.isFrozen(ref)).toBe(true);
    expect(Object.isFrozen(ref.producer)).toBe(true);
    expect(Object.getPrototypeOf(ref)).toBe(Object.prototype);
    expect(Object.values(Object.getOwnPropertyDescriptors(ref)).every((d) => "value" in d)).toBe(
      true,
    );
    expect(Object.hasOwn(ref, "toJSON")).toBe(false);
    const json: workflow.JsonValue = ref;
    const cloned = workflow.materializeBindings({ $literal: json }, {});
    expect(cloned).toEqual(ref);
    store.dispose();
  });

  it("rejects per-artifact and total overflow atomically", () => {
    const store = new RunArtifactStore({ ...limits, maxArtifactBytes: 8, maxTotalBytes: 12 });
    const a = store.put({ producer, data: "12345678" });
    expect(() =>
      store.put({ producer: { jobId: "A2", name: "x" }, data: "123456789" }),
    ).toThrowError(expect.objectContaining({ code: "POLICY_VIOLATION" }));
    expect(store.stats).toEqual({ bytes: 8, artifacts: 1, closed: false });
    store.put({ producer: { jobId: "A2", name: "x" }, data: "1234" });
    expect(() => store.put({ producer: { jobId: "A3", name: "x" }, data: "1" })).toThrowError(
      expect.objectContaining({ code: "POLICY_VIOLATION" }),
    );
    expect(store.stats).toEqual({ bytes: 12, artifacts: 2, closed: false });
    expect(
      Buffer.from(store.reader.read(source(a), { offset: 0, maxBytes: 8 }).data).toString(),
    ).toBe("12345678");
    store.dispose();
  });

  it("bounds UTF-8 and invalid-byte previews", () => {
    for (const [data, cap, preview] of [
      ["日🙂x", 3, "日"],
      ["日🙂x", 7, "日🙂"],
      [new Uint8Array([0xff, 0xff, 0xff]), 4, "�"],
      ["abc", 0, ""],
    ] as const) {
      const store = new RunArtifactStore({ ...limits, maxPreviewBytes: cap });
      const ref = store.put({ producer, data });
      expect(ref.preview).toBe(preview);
      expect(Buffer.byteLength(ref.preview)).toBeLessThanOrEqual(cap);
      expect(ref.truncated).toBe(true);
      store.dispose();
    }
    const store = new RunArtifactStore(limits);
    expect(store.put({ producer, data: "abc" })).toMatchObject({
      preview: "abc",
      truncated: false,
    });
    store.dispose();
  });

  it("preserves a UTF-8 BOM in bounded preview metadata", () => {
    const store = new RunArtifactStore({ ...limits, maxPreviewBytes: 4 });
    const ref = store.put({ producer, data: "\uFEFFx" });
    expect(ref).toMatchObject({ preview: "\uFEFFx", truncated: false, bytes: 4 });
    store.dispose();
  });

  it("keeps the invocation signal fixed when caller options change", async () => {
    const controller = new AbortController();
    const options: workflow.ArtifactStoreOptions = { ...limits, signal: controller.signal };
    await expect(
      withArtifactStore(options, async (store) => {
        store.put({ producer, data: "hello" });
        options.signal = new AbortController().signal;
        controller.abort();
        return "completed";
      }),
    ).rejects.toMatchObject({ code: "ABORTED" });
  });

  it("returns bounded copies without retaining caller mutations", () => {
    const store = new RunArtifactStore(limits);
    const input = new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8, 9]);
    const ref = store.put({ producer, data: input });
    input.fill(0);
    const first = store.reader.read(source(ref), { offset: 0, maxBytes: 4 });
    expect([...first.data]).toEqual([1, 2, 3, 4]);
    expect(first).toMatchObject({ offset: 0, nextOffset: 4, eof: false });
    first.data.fill(0);
    expect([...store.reader.read(source(ref), { offset: 4, maxBytes: 8 }).data]).toEqual([
      5, 6, 7, 8, 9,
    ]);
    expect([...store.reader.read(source(ref), { offset: 0, maxBytes: 4 }).data]).toEqual([
      1, 2, 3, 4,
    ]);
    expect(store.reader.read(source(ref), { offset: 9, maxBytes: 4 })).toMatchObject({
      nextOffset: 9,
      eof: true,
    });
    expect(() => store.reader.read(source(ref), { offset: 0, maxBytes: 9 })).toThrowError(
      expect.objectContaining({ code: "POLICY_VIOLATION" }),
    );
    store.dispose();
    store.dispose();
    expect(store.stats).toEqual({ bytes: 0, artifacts: 0, closed: true });
  });

  it("rejects duplicate producers without overwriting a value", () => {
    const store = new RunArtifactStore(limits);
    const ref = store.put({ producer, data: "first" });
    expect(() => store.put({ producer, data: "next" })).toThrow();
    expect(
      Buffer.from(store.reader.read(source(ref), { offset: 0, maxBytes: 8 }).data).toString(),
    ).toBe("first");
    store.dispose();
  });

  it("rejects foreign runs, malformed URIs and altered provenance", () => {
    const a = new RunArtifactStore(limits);
    const b = new RunArtifactStore(limits);
    const ref = a.put({ producer, data: "secret" });
    expect(() => b.reader.read(source(ref), { offset: 0, maxBytes: 8 })).toThrow();
    for (const artifact of [
      "../secret",
      "file:///etc/passwd",
      "C:\\private.txt",
      "/etc/passwd",
      `artifact://${a.runId}/../../secret`,
      `artifact://${a.runId}/%2e%2e`,
      `${ref.artifact}?path=/etc/passwd`,
      `${ref.artifact}/extra`,
      `artifact://${a.runId}/00000000-0000-0000-0000-000000000000`,
      `${ref.artifact}\n`,
    ]) {
      expect(() =>
        a.reader.read(source({ ...ref, artifact }), { offset: 0, maxBytes: 8 }),
      ).toThrow();
    }
    for (const altered of [
      { ...ref, runId: b.runId },
      { ...ref, producer: { jobId: "A2", name: "stdout" } },
      { ...ref, bytes: 0 },
      { ...ref, preview: "forged" },
    ]) {
      expect(() => a.reader.read(source(altered), { offset: 0, maxBytes: 8 })).toThrow();
    }
    a.dispose();
    b.dispose();
  });

  it("rejects a workspace source at the artifact reader boundary", () => {
    const store = new RunArtifactStore(limits);
    const ref = store.put({ producer, data: "hello" });
    const workspace: ReadSource = { kind: "workspace", path: ref.artifact };
    // @ts-expect-error workspace sources cannot enter the artifact port.
    expect(() => store.reader.read(workspace, { offset: 0, maxBytes: 8 })).toThrow();
    store.dispose();
  });

  it("serializes metadata only and supports includePreview", () => {
    const store = new RunArtifactStore({ ...limits, maxArtifactBytes: 4096, maxTotalBytes: 4096 });
    const ref = store.put({ producer, data: `head${"PRIVATE BODY".repeat(200)}` });
    const output = JSON.stringify(store);
    expect(output).not.toContain("PRIVATE BODY");
    expect(JSON.parse(output).artifacts[0].preview).toBe("headP");
    expect(store.metadata({ includePreview: false })[0]).not.toHaveProperty("preview");
    store.dispose();
    const final = projectArtifactMetadata(ref);
    expect(final).toMatchObject({ availability: "unavailable", preview: "headP", bytes: 2404 });
    expect(projectArtifactMetadata(ref, { includePreview: false })).not.toHaveProperty("preview");
    expect(final).not.toHaveProperty("data");
    expect(store.metadata()).toEqual([]);
    expect(JSON.stringify(store)).not.toContain("headP");
  });

  it("withArtifactStore disposes on completion and failure", async () => {
    let borrowed: ArtifactReader | undefined;
    let ref: ArtifactReference | undefined;
    let owned: InstanceType<typeof RunArtifactStore> | undefined;
    const value = await withArtifactStore(limits, async (store) => {
      owned = store;
      borrowed = store.reader;
      ref = store.put({ producer, data: "hello" });
      return ref;
    });
    expect(value).toEqual(ref);
    expect(owned?.stats).toEqual({ bytes: 0, artifacts: 0, closed: true });
    expect(() => borrowed?.read(source(value), { offset: 0, maxBytes: 8 })).toThrow();
    await expect(
      withArtifactStore(limits, async (store) => {
        owned = store;
        store.put({ producer, data: "hello" });
        throw new Error("producer failed");
      }),
    ).rejects.toThrow("producer failed");
    expect(owned?.stats).toEqual({ bytes: 0, artifacts: 0, closed: true });
  });

  it("rejects pre-aborted signals without invoking the callback", async () => {
    const controller = new AbortController();
    controller.abort();
    expect(() => new RunArtifactStore({ ...limits, signal: controller.signal })).toThrowError(
      expect.objectContaining({ code: "ABORTED" }),
    );
    let invoked = false;
    await expect(
      withArtifactStore({ ...limits, signal: controller.signal }, async () => {
        invoked = true;
      }),
    ).rejects.toMatchObject({ code: "ABORTED" });
    expect(invoked).toBe(false);
  });

  it("clears immediately on inflight abort and removes its listener", async () => {
    const controller = new AbortController();
    const remove = vi.spyOn(controller.signal, "removeEventListener");
    let borrowed: ArtifactReader | undefined;
    let ref: ArtifactReference | undefined;
    let owned: InstanceType<typeof RunArtifactStore> | undefined;
    const result = withArtifactStore({ ...limits, signal: controller.signal }, async (store) => {
      owned = store;
      borrowed = store.reader;
      ref = store.put({ producer, data: "hello" });
      await Promise.resolve();
      controller.abort();
      expect(store.stats).toEqual({ bytes: 0, artifacts: 0, closed: true });
      expect(() =>
        store.reader.read(source(ref as ArtifactReference), { offset: 0, maxBytes: 8 }),
      ).toThrowError(expect.objectContaining({ code: "ABORTED" }));
      return ref;
    });
    await expect(result).rejects.toMatchObject({ code: "ABORTED" });
    expect(owned?.stats.closed).toBe(true);
    expect(() =>
      borrowed?.read(source(ref as ArtifactReference), { offset: 0, maxBytes: 8 }),
    ).toThrow();
    expect(remove).toHaveBeenCalledWith("abort", expect.any(Function));
    remove.mockRestore();
  });

  it("validates injected caps and read ranges", () => {
    for (const cap of [-1, NaN, Infinity, Number.MAX_SAFE_INTEGER + 1]) {
      for (const field of Object.keys(limits)) {
        expect(() => new RunArtifactStore({ ...limits, [field]: cap })).toThrow();
      }
    }
    expect(() => new RunArtifactStore({ ...limits, maxReadBytes: 0 })).toThrow();
    const store = new RunArtifactStore(limits);
    const ref = store.put({ producer, data: "x" });
    for (const range of [
      { offset: -1, maxBytes: 1 },
      { offset: 2, maxBytes: 1 },
      { offset: 0.5, maxBytes: 1 },
      { offset: 0, maxBytes: 0 },
      { offset: 0, maxBytes: Infinity },
    ]) {
      expect(() => store.reader.read(source(ref), range)).toThrow();
    }
    store.dispose();
    expect(() => store.put({ producer, data: "x" })).toThrow();
  });
});
