import * as filesystem from "node:fs/promises";
import { appendFile, mkdtemp, realpath, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { compileSchema, createPathDenylist } from "@cheaper/core";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createInvocationToolRuntime, createToolCatalog, createToolRuntime } from "../src/index.js";
import { createReadTool } from "../src/read.js";

vi.mock("node:fs/promises", async (original) => {
  const actual = await original<typeof import("node:fs/promises")>();
  return { ...actual, open: vi.fn(actual.open) };
});

let root: string;
beforeEach(async () => {
  root = await realpath(await mkdtemp(path.join(tmpdir(), "cheaper-artifact-read-")));
  await writeFile(path.join(root, "a.txt"), "hello\n");
});
afterEach(async () => {
  vi.restoreAllMocks();
  await rm(root, { recursive: true, force: true });
});
const options = () => ({
  root,
  symlinkPolicy: "deny" as const,
  names: ["read"] as const,
  maxToolCalls: 100,
  onLimit: vi.fn(),
});
const reference = {
  artifact: "artifact://test/value",
  runId: "test",
  producer: { jobId: "producer", name: "text" },
  bytes: 6,
  preview: "hello",
  truncated: true,
};
const ports = (cap = 6) => ({
  publisher: { maxArtifactBytes: cap, put: vi.fn(() => reference) },
  reader: {
    read: vi.fn(() => ({
      data: new Uint8Array(Buffer.from("hel")),
      offset: 0,
      nextOffset: 3,
      eof: false,
    })),
  },
});

describe("artifact read modes", () => {
  it("publishes without returning source text and fixes producer to the scope owner", async () => {
    const artifacts = ports();
    const invocation = createInvocationToolRuntime({ root, symlinkPolicy: "deny", artifacts });
    const runtime = invocation.createWorkerScope({ ...options(), ownerId: "job-a" });
    const result = await runtime.execute("read", { path: "a.txt", artifact: "text" });
    expect(result).toEqual({ content: [], details: { artifact: reference } });
    expect(artifacts.publisher.put).toHaveBeenCalledWith({
      producer: { jobId: "job-a", name: "text" },
      data: Buffer.from("hello\n"),
    });
    const metadata = runtime.catalog.get("read");
    if (!metadata) throw new Error("missing read metadata");
    expect(compileSchema(metadata.outputSchema, "output").validate(result).ok).toBe(true);
    await runtime.dispose();
    expect(artifacts.reader.read).not.toHaveBeenCalled();
    await invocation.dispose();
  });
  it("reads byte slices without workspace access and exposes empty resource declarations", async () => {
    const artifacts = ports();
    const denylist = createPathDenylist(["**"]);
    const denied = vi.spyOn(denylist, "isDenied");
    const runtime = createToolRuntime({ ...options(), artifacts, denylist });
    const input = { source: { kind: "artifact", reference }, offset: 0, maxBytes: 3 };
    expect(await runtime.execute("read", input)).toEqual({
      content: [{ type: "text", text: "hel" }],
      details: { offset: 0, nextOffset: 3, eof: false },
    });
    expect(artifacts.reader.read).toHaveBeenCalledWith(input.source, { offset: 0, maxBytes: 3 });
    expect(denied).not.toHaveBeenCalled();
    expect(createToolCatalog().get("read")?.resources?.(input)).toEqual({
      reads: [],
      writes: [],
      produces: [],
    });
    await runtime.dispose();
  });
  it.each([
    { path: "a.txt", artifact: "text", offset: 1 },
    { path: "a.txt", source: { kind: "artifact", reference }, offset: 0, maxBytes: 3 },
    { source: { kind: "artifact", reference }, offset: 0, maxBytes: 0 },
    { path: "a.txt", artifact: "text", producer: { jobId: "spoof" } },
    { path: "artifact://test/value" },
  ])("rejects mixed modes and invalid ranges before admission: %j", async (input) => {
    const runtime = createToolRuntime({ ...options(), artifacts: ports() });
    await expect(runtime.execute("read", input)).rejects.toMatchObject({ code: "INVALID_INPUT" });
    expect(runtime.budget.toolCalls).toBe(0);
    await runtime.dispose();
  });
  it.each([0, 5])("refuses overflow before publisher mutation for cap %i", async (cap) => {
    const artifacts = ports(cap);
    const runtime = createToolRuntime({ ...options(), artifacts });
    await expect(
      runtime.execute("read", { path: "a.txt", artifact: "text" }),
    ).rejects.toMatchObject({ code: "POLICY_VIOLATION" });
    expect(artifacts.publisher.put).not.toHaveBeenCalled();
    await runtime.dispose();
  });
  it.each([
    { path: "a.txt", artifact: "text" },
    { source: { kind: "artifact", reference }, offset: 0, maxBytes: 3 },
  ])("rejects missing borrowed ports: %j", async (input) => {
    const runtime = createToolRuntime(options());
    await expect(runtime.execute("read", input)).rejects.toMatchObject({ code: "INVALID_REQUEST" });
    await runtime.dispose();
  });
  it.each([
    ["secret.txt", "secret"],
    ["image.PNG", "text"],
    ["binary.bin", "a\0b"],
  ])("retains workspace publication denial for %s", async (file, data) => {
    await writeFile(path.join(root, file), data);
    const artifacts = ports(100);
    const runtime = createToolRuntime({
      ...options(),
      artifacts,
      denylist: createPathDenylist(["secret.txt"]),
    });
    await expect(runtime.execute("read", { path: file, artifact: "text" })).rejects.toThrow();
    expect(artifacts.publisher.put).not.toHaveBeenCalled();
    await runtime.dispose();
  });
  it("bounds reads to cap+1 and rejects a file that grows after stat", async () => {
    const artifacts = ports(6);
    const originalOpen =
      await vi.importActual<typeof import("node:fs/promises")>("node:fs/promises");
    let observed = 0;
    let closed = false;
    vi.mocked(filesystem.open).mockImplementationOnce(async (...args) => {
      const handle = await originalOpen.open(...args);
      const read = handle.read.bind(handle);
      const close = handle.close.bind(handle);
      vi.spyOn(handle, "read").mockImplementation(async (...readArgs: Parameters<typeof read>) => {
        const result = await read(...readArgs);
        observed += result.bytesRead;
        if (observed === 2) await appendFile(path.join(root, "a.txt"), "growth");
        return result;
      });
      vi.spyOn(handle, "close").mockImplementation(async () => {
        closed = true;
        await close();
      });
      return handle;
    });
    const tool = createReadTool({ ...options(), artifacts, chunkBytes: 2 });
    await expect(tool.execute({ path: "a.txt", artifact: "text" })).rejects.toMatchObject({
      code: "POLICY_VIOLATION",
    });
    expect(observed).toBe(7);
    expect(closed).toBe(true);
    expect(artifacts.publisher.put).not.toHaveBeenCalled();
  });
  it("rechecks abort after I/O before publication and closes the handle", async () => {
    const artifacts = ports();
    const controller = new AbortController();
    const originalOpen =
      await vi.importActual<typeof import("node:fs/promises")>("node:fs/promises");
    let closed = false;
    const reason = new Error("cancel publication");
    vi.mocked(filesystem.open).mockImplementationOnce(async (...args) => {
      const handle = await originalOpen.open(...args);
      const read = handle.read.bind(handle);
      const close = handle.close.bind(handle);
      vi.spyOn(handle, "read").mockImplementation(async (...readArgs: Parameters<typeof read>) => {
        const result = await read(...readArgs);
        controller.abort(reason);
        return result;
      });
      vi.spyOn(handle, "close").mockImplementation(async () => {
        closed = true;
        await close();
      });
      return handle;
    });
    const tool = createReadTool({ ...options(), artifacts, chunkBytes: 2 });
    await expect(tool.execute({ path: "a.txt", artifact: "text" }, controller.signal)).rejects.toBe(
      reason,
    );
    expect(closed).toBe(true);
    expect(artifacts.publisher.put).not.toHaveBeenCalled();
  });
});
