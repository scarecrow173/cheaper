import { CheaperError, compileSchema } from "@cheaper/core";
import { type ExecutionMetrics, type ResourceLease, ResourceManager } from "@cheaper/tool-runtime";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  runWorkflow,
  type WorkflowEnginePorts,
  WorkflowExecutorError,
  type WorkflowExecutorResult,
} from "../src/engine.js";
import type { CompiledWorkflow } from "../src/index.js";
import { definition, fixtures } from "./fixtures.js";

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}
const flush = async () => {
  for (let i = 0; i < 20; i++) await Promise.resolve();
};
const toolResult = (text: string): WorkflowExecutorResult => ({
  result: { content: [{ type: "text", text }] },
});

function harness() {
  const resources = new ResourceManager(process.cwd(), "deny");
  const ports: WorkflowEnginePorts = {
    worker: { execute: vi.fn(async () => ({ result: "worker" })) },
    tool: { execute: vi.fn(async ({ jobId }) => toolResult(jobId)) },
    acquireJobLease: vi.fn(({ jobId, job, signal }) =>
      resources.request({
        ownerId: jobId,
        kind: "job",
        signal,
        prepare: () => ({
          claims: job.manifest
            ? [...job.manifest.reads, ...job.manifest.writes, ...job.manifest.produces]
            : [],
          readStrictness: "record",
        }),
        start: (lease) => lease,
      }),
    ),
    releaseJobLease: vi.fn((lease) => resources.release(lease)),
    abort: vi.fn(),
    executionMetrics: vi.fn(() => null),
  };
  return { ports, resources };
}
async function independent(...ids: string[]): Promise<CompiledWorkflow> {
  const raw = definition();
  for (const id of ids) raw.jobs[id] = { kind: "tool", tool: "grep", input: {} };
  return fixtures().compiler.compile(raw);
}

afterEach(() => vi.useRealTimers());

describe("one-shot scheduling and public projection", () => {
  it.each([0, -1, Infinity, Number.NaN])(
    "invalid timeout %s はadmission前に拒否",
    async (timeoutMs) => {
      const workflow = await independent("a");
      const { ports } = harness();
      await expect(runWorkflow({ workflow, ports, maxJobs: 1, timeoutMs })).rejects.toMatchObject({
        code: "INVALID_REQUEST",
      });
      expect(ports.acquireJobLease).not.toHaveBeenCalled();
    },
  );

  it.each([-1, Infinity, Number.NaN])(
    "invalid grace %s はadmission前に拒否",
    async (abortGraceMs) => {
      const workflow = await independent("a");
      const { ports } = harness();
      await expect(
        runWorkflow({ workflow, ports, maxJobs: 1, timeoutMs: 100, abortGraceMs }),
      ).rejects.toMatchObject({ code: "INVALID_REQUEST" });
      expect(ports.acquireJobLease).not.toHaveBeenCalled();
    },
  );

  it("empty workflowもoutputsをvalidateしてtimerを掃除する", async () => {
    vi.useFakeTimers();
    const raw = definition();
    raw.outputs.ok = { schema: { type: "boolean" }, value: { $literal: true } };
    const workflow = await fixtures().compiler.compile(raw);
    const { ports } = harness();
    const outcome = await runWorkflow({ workflow, ports, maxJobs: 1, timeoutMs: 100 });
    expect(outcome).toMatchObject({ ok: true, result: { ok: true }, jobs: [] });
    expect(ports.acquireJobLease).not.toHaveBeenCalled();
    expect(vi.getTimerCount()).toBe(0);
  });

  it("inferred edgeと単独$fromもcompiled edgesに従いinput/output snapshotを使う", async () => {
    const { compiler } = fixtures({
      resources: {
        infer: (jobs) => ({
          manifests: Object.fromEntries(
            Object.keys(jobs).map((id) => [id, { reads: [], writes: [], produces: [] }]),
          ),
          edges: [{ from: "a", to: "b" }],
        }),
      },
    });
    const raw = definition();
    raw.jobs.a = { kind: "tool", tool: "grep", input: {} };
    raw.jobs.b = { kind: "tool", tool: "grep", input: {} };
    raw.jobs.c = {
      kind: "tool",
      tool: "grep",
      input: { value: { $from: "b", path: "/content/0/text" } },
    };
    raw.outputs.ok = { schema: {}, value: { $from: "c", path: "/content/0/text" } };
    const workflow = await compiler.compile(raw);
    const materialize = vi.spyOn(workflow, "materializeJobInput");
    const { ports } = harness();
    const a = deferred<WorkflowExecutorResult>();
    const starts: string[] = [];
    ports.tool.execute = vi.fn(async ({ jobId, input }) => {
      starts.push(jobId);
      if (jobId === "a") return a.promise;
      if (jobId === "c") expect(input).toEqual({ value: "b" });
      return toolResult(jobId);
    });
    const done = runWorkflow({ workflow, ports, maxJobs: 3, timeoutMs: 10000 });
    await flush();
    expect(starts).toEqual(["a"]);
    a.resolve(toolResult("a"));
    expect((await done).result).toEqual({ ok: "c" });
    expect(starts).toEqual(["a", "b", "c"]);
    expect(materialize.mock.calls.map(([id]) => id)).toEqual(["a", "b", "c"]);
  });

  it("failureもadapterが選択したmetadataを保持しerror内部payloadを公開しない", async () => {
    const workflow = await independent("a");
    const { ports } = harness();
    ports.tool.execute = vi.fn(async () => {
      throw new WorkflowExecutorError("TOOL_EXECUTION_FAILURE", "failed", {
        meta: { model: "selected", usage: { input: 10 }, durationMs: 9000 },
        details: { private: "PRIVATE DETAILS" },
        cause: "PRIVATE CAUSE",
      });
    });
    const outcome = await runWorkflow({
      workflow,
      ports,
      maxJobs: 1,
      timeoutMs: 10000,
      now: () => 1,
    });
    expect(outcome.jobs[0]?.meta).toMatchObject({
      model: "selected",
      usage: { input: 10 },
      durationMs: 0,
      execution: null,
    });
    expect(JSON.stringify(outcome)).not.toMatch(/PRIVATE DETAILS|PRIVATE CAUSE/);
  });
  it("job/overall metrics は owner snapshotでmetaのduration/executionを上書きする", async () => {
    const workflow = await independent("a", "b");
    const { ports } = harness();
    const metric = (operations: number): ExecutionMetrics => ({
      operations,
      byState: {
        queued: 0,
        running: 0,
        completed: operations,
        failed: 0,
        cancelled: 0,
        timed_out: 0,
      },
      byTool: {},
      maxConcurrency: 1,
      operationMs: 4,
      activeMs: 3,
      overlapMs: 1,
      queueMs: 0,
    });
    const global = metric(7);
    ports.executionMetrics = vi.fn((id) => (id ? metric(id === "a" ? 2 : 3) : global));
    ports.tool.execute = vi.fn(async ({ jobId }) => ({
      ...toolResult(jobId),
      meta: { durationMs: 9000, execution: { forged: true }, turns: 1 },
    }));
    const outcome = await runWorkflow({
      workflow,
      ports,
      maxJobs: 2,
      timeoutMs: 10000,
      now: () => 10,
    });
    expect(outcome.meta.execution?.operations).toBe(7);
    expect(outcome.jobs.map((job) => job.meta.execution?.operations)).toEqual([2, 3]);
    expect(outcome.jobs[0]?.meta).toMatchObject({ durationMs: 0, turns: 1 });
    global.operations = 100;
    expect(outcome.meta.execution?.operations).toBe(7);
  });
  it.each([0, -1, 1.5, Infinity, Number.MAX_SAFE_INTEGER + 1])(
    "invalid maxJobs %s は実行前に拒否",
    async (maxJobs) => {
      const workflow = await independent("a");
      const { ports } = harness();
      await expect(
        runWorkflow({ workflow, ports, maxJobs, timeoutMs: 10000 }),
      ).rejects.toMatchObject({ code: "INVALID_REQUEST" });
      expect(ports.acquireJobLease).not.toHaveBeenCalled();
    },
  );

  it("admission待機はslotを消費せず非conflict jobを進め、semantic edgesを変えない", async () => {
    const workflow = await independent("a", "b", "c");
    const sharedPath = `${process.cwd()}/resource`;
    for (const id of ["a", "b"]) {
      const job = workflow.jobs[id];
      if (job)
        job.manifest = {
          reads: [],
          writes: [
            {
              resource: { kind: "workspace", path: sharedPath, scope: "exact" },
              mode: "exclusive",
            },
          ],
          produces: [],
        };
    }
    const { ports } = harness();
    const a = deferred<WorkflowExecutorResult>();
    const starts: string[] = [];
    ports.tool.execute = vi.fn(async ({ jobId }) => {
      starts.push(jobId);
      return jobId === "a" ? a.promise : toolResult(jobId);
    });
    const done = runWorkflow({ workflow, ports, maxJobs: 2, timeoutMs: 10000 });
    await flush();
    expect(starts).toEqual(["a", "c"]);
    expect(workflow.edges).toEqual([]);
    a.resolve(toolResult("a"));
    expect((await done).ok).toBe(true);
    expect(starts).toEqual(["a", "c", "b"]);
  });

  it("output-only schema failure はINVALID_OUTPUTで独立outputを保持する", async () => {
    const { compiler, worker } = fixtures();
    worker.outputSchema = compileSchema({}, "output");
    const raw = definition();
    raw.jobs.a = { kind: "worker", worker: "bulk-reader", input: { task: "x" } };
    raw.outputs.bad = { schema: { type: "number" }, value: { $from: "a", path: "" } };
    raw.outputs.good = { schema: {}, value: { $literal: "kept" } };
    const workflow = await compiler.compile(raw);
    const { ports } = harness();
    const outcome = await runWorkflow({ workflow, ports, maxJobs: 1, timeoutMs: 10000 });
    expect(outcome).toMatchObject({
      ok: false,
      result: { good: "kept" },
      error: { code: "INVALID_OUTPUT" },
      meta: { outputErrors: [{ name: "bad" }] },
    });
    expect(outcome.jobs[0]?.status).toBe("succeeded");
  });

  it("diagnosticsはcodepoint-safe UTF-8上限でdetails/causeを公開しない", async () => {
    const workflow = await independent("a");
    const { ports } = harness();
    ports.tool.execute = vi.fn(async () => {
      throw new CheaperError("PARTIAL_MUTATION", "あ😀".repeat(1000), {
        details: { private: "PRIVATE DETAILS" },
        cause: "PRIVATE CAUSE",
      });
    });
    const outcome = await runWorkflow({ workflow, ports, maxJobs: 1, timeoutMs: 10000 });
    const message = outcome.jobs[0]?.error?.message ?? "";
    expect(Buffer.byteLength(message)).toBeLessThanOrEqual(2048);
    expect(message).not.toContain("�");
    expect(outcome.jobs[0]?.error?.code).toBe("PARTIAL_MUTATION");
    expect(JSON.stringify(outcome)).not.toMatch(/PRIVATE DETAILS|PRIVATE CAUSE/);
  });

  it("mixed fan-out は maxJobs 内で重なり、fan-in は全producer成功を待つ", async () => {
    const { compiler } = fixtures();
    const raw = definition();
    raw.jobs.a = { kind: "worker", worker: "bulk-reader", input: { task: "PRIVATE INPUT" } };
    raw.jobs.b = { kind: "tool", tool: "grep", input: {} };
    raw.jobs.c = {
      kind: "tool",
      tool: "grep",
      needs: ["a", "b"],
      input: { value: { $from: "a", path: "/payload" } },
    };
    raw.outputs.result = { schema: {}, value: { $from: "c", path: "/content/0/text" } };
    const workflow = await compiler.compile(raw);
    const { ports } = harness();
    const a = deferred<WorkflowExecutorResult>();
    const b = deferred<WorkflowExecutorResult>();
    const starts: string[] = [];
    ports.worker.execute = vi.fn(({ jobId }) => {
      starts.push(jobId);
      return a.promise;
    });
    ports.tool.execute = vi.fn(async ({ jobId, input }) => {
      starts.push(jobId);
      if (jobId === "b") return b.promise;
      expect(input).toEqual({ value: "PRIVATE RESULT" });
      return toolResult("public");
    });
    const done = runWorkflow({ workflow, ports, maxJobs: 2, timeoutMs: 10000 });
    await flush();
    expect(starts).toEqual(["a", "b"]);
    const producerValue = { payload: "PRIVATE RESULT" };
    a.resolve({ result: producerValue, meta: { model: "test" } });
    await flush();
    expect(starts).toEqual(["a", "b"]);
    producerValue.payload = "MUTATED RESULT";
    b.resolve(toolResult("private b"));
    const outcome = await done;
    expect(starts).toEqual(["a", "b", "c"]);
    expect(outcome).toMatchObject({ ok: true, result: { result: "public" } });
    expect(outcome.jobs.map((job) => job.id)).toEqual(["a", "b", "c"]);
    expect(JSON.stringify(outcome)).not.toMatch(/PRIVATE INPUT|PRIVATE RESULT|private b/);
    expect(outcome.jobs[0]?.meta.model).toBe("test");
    expect(ports.releaseJobLease).toHaveBeenCalledTimes(3);
  });

  it("maxJobs1 は直列化し整数風IDをcode-unit順に投影する", async () => {
    const workflow = await independent("2", "10", "a");
    const { ports } = harness();
    const holds = new Map<string, ReturnType<typeof deferred<WorkflowExecutorResult>>>();
    const starts: string[] = [];
    ports.tool.execute = vi.fn(({ jobId }) => {
      starts.push(jobId);
      const hold = deferred<WorkflowExecutorResult>();
      holds.set(jobId, hold);
      return hold.promise;
    });
    const done = runWorkflow({ workflow, ports, maxJobs: 1, timeoutMs: 10000 });
    await flush();
    expect(starts).toEqual(["10"]);
    holds.get("10")?.resolve(toolResult("x"));
    await flush();
    expect(starts).toEqual(["10", "2"]);
    holds.get("2")?.resolve(toolResult("x"));
    await flush();
    expect(starts).toEqual(["10", "2", "a"]);
    holds.get("a")?.resolve(toolResult("x"));
    expect((await done).jobs.map((job) => job.id)).toEqual(["10", "2", "a"]);
  });

  it.each(["input", "admission", "executor", "output"])(
    "%s error は下流だけskipし独立公開outputを保持する",
    async (boundary) => {
      const { compiler } = fixtures();
      const raw = definition();
      raw.jobs.a = { kind: "tool", tool: "grep", input: {} };
      raw.jobs.b = { kind: "tool", tool: "grep", needs: ["a"], input: {} };
      raw.jobs.c = { kind: "tool", tool: "grep", input: {} };
      raw.outputs.bad = { schema: {}, value: { $from: "a", path: "" } };
      raw.outputs.good = { schema: {}, value: { $from: "c", path: "/content/0/text" } };
      const workflow = await compiler.compile(raw);
      const { ports } = harness();
      const error = new CheaperError("INVALID_INPUT", "boundary failure", {
        details: { hidden: "PRIVATE DETAILS" },
        cause: "PRIVATE CAUSE",
      });
      const workflowRun: CompiledWorkflow = {
        ...workflow,
        materializeJobInput: (id, results) => {
          if (id === "a" && boundary === "input") throw error;
          return workflow.materializeJobInput(id, results);
        },
        validateJobOutput: workflow.validateJobOutput.bind(workflow),
        materializeOutput: workflow.materializeOutput.bind(workflow),
        materializeOutputs: workflow.materializeOutputs.bind(workflow),
      };
      const acquire = ports.acquireJobLease;
      ports.acquireJobLease = vi.fn((context) =>
        context.jobId === "a" && boundary === "admission"
          ? Promise.reject(error)
          : acquire(context),
      );
      ports.tool.execute = vi.fn(async ({ jobId }) => {
        if (jobId === "a") {
          if (boundary === "output") return { result: 42 };
          throw error;
        }
        return toolResult("kept");
      });
      const outcome = await runWorkflow({
        workflow: workflowRun,
        ports,
        maxJobs: 2,
        timeoutMs: 10000,
      });
      expect(outcome).toMatchObject({
        ok: false,
        result: { good: "kept" },
        error: { code: "TOOL_EXECUTION_FAILURE" },
      });
      expect(outcome.jobs.map((job) => job.status)).toEqual(["failed", "skipped", "succeeded"]);
      expect(outcome.jobs[1]?.error?.code).toBe("DEPENDENCY_FAILED");
      expect(outcome.meta.outputErrors.map((entry) => entry.name)).toEqual(["bad"]);
      expect(JSON.stringify(outcome)).not.toMatch(/PRIVATE DETAILS|PRIVATE CAUSE/);
    },
  );
});

describe("outer cancellation and physical lease lifetime", () => {
  it("timeoutの起点はtimerをarmした時点ではなくengine開始時点", async () => {
    vi.useFakeTimers();
    const workflow = await independent("a");
    const { ports } = harness();
    const result = deferred<WorkflowExecutorResult>();
    ports.tool.execute = vi.fn(() => result.promise);
    let initial = true;
    const now = () => {
      if (initial) {
        initial = false;
        vi.advanceTimersByTime(7);
      }
      return performance.now();
    };
    const done = runWorkflow({ workflow, ports, maxJobs: 1, timeoutMs: 10, abortGraceMs: 0, now });
    await flush();
    await vi.advanceTimersByTimeAsync(2);
    expect(ports.abort).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);
    expect(ports.abort).toHaveBeenCalledTimes(1);
    result.resolve(toolResult("late"));
    expect((await done).error?.code).toBe("TIMEOUT");
  });

  it("graceの起点はabort処理終了ではなくfirst outer abortの時点", async () => {
    vi.useFakeTimers();
    const workflow = await independent("a");
    const { ports } = harness();
    ports.abort = vi.fn(() => vi.advanceTimersByTime(7));
    const result = deferred<WorkflowExecutorResult>();
    ports.tool.execute = vi.fn(() => result.promise);
    const outer = new AbortController();
    const completed = vi.fn();
    const done = runWorkflow({
      workflow,
      ports,
      maxJobs: 1,
      timeoutMs: 100,
      abortGraceMs: 10,
      signal: outer.signal,
    }).then((outcome) => {
      completed();
      return outcome;
    });
    await flush();
    outer.abort();
    await vi.advanceTimersByTimeAsync(2);
    expect(completed).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);
    expect(completed).toHaveBeenCalledTimes(1);
    result.resolve(toolResult("late"));
    expect((await done).error?.code).toBe("ABORTED");
  });

  it("default graceは5000msでconflict admissionをactual late-settleまで保持する", async () => {
    vi.useFakeTimers();
    const workflow = await independent("a");
    const claim = {
      resource: {
        kind: "workspace" as const,
        path: `${process.cwd()}/held`,
        scope: "exact" as const,
      },
      mode: "exclusive" as const,
    };
    const job = workflow.jobs.a;
    if (!job) throw new Error("missing a");
    job.manifest = { reads: [], writes: [claim], produces: [] };
    const { ports, resources } = harness();
    const result = deferred<WorkflowExecutorResult>();
    ports.tool.execute = vi.fn(() => result.promise);
    const done = runWorkflow({ workflow, ports, maxJobs: 1, timeoutMs: 10 });
    const completed = vi.fn();
    void done.then(completed);
    await flush();
    await vi.advanceTimersByTimeAsync(10);
    await vi.advanceTimersByTimeAsync(4999);
    expect(completed).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);
    const outcome = await done;
    expect(outcome.error?.code).toBe("TIMEOUT");
    const granted = vi.fn();
    const foreign = resources.request({
      ownerId: "foreign",
      kind: "job",
      prepare: () => ({ claims: [claim], readStrictness: "record" }),
      start: (lease) => {
        granted();
        return lease;
      },
    });
    await flush();
    expect(granted).not.toHaveBeenCalled();
    result.resolve(toolResult("late"));
    resources.release(await foreign);
    expect(granted).toHaveBeenCalledTimes(1);
    expect(ports.releaseJobLease).toHaveBeenCalledTimes(1);
    expect(vi.getTimerCount()).toBe(0);
  });

  it("timeoutが最初なら後続signal理由を採用せずgrace0で終了する", async () => {
    vi.useFakeTimers();
    const workflow = await independent("a");
    const { ports } = harness();
    const result = deferred<WorkflowExecutorResult>();
    ports.tool.execute = vi.fn(() => result.promise);
    const outer = new AbortController();
    const done = runWorkflow({
      workflow,
      ports,
      maxJobs: 1,
      timeoutMs: 10,
      abortGraceMs: 0,
      signal: outer.signal,
    });
    await flush();
    await vi.advanceTimersByTimeAsync(10);
    outer.abort(new Error("later signal"));
    const outcome = await done;
    expect(outcome.error?.code).toBe("TIMEOUT");
    expect(ports.abort).toHaveBeenCalledTimes(1);
    result.reject(new Error("late rejection"));
    await flush();
    expect(ports.releaseJobLease).toHaveBeenCalledTimes(1);
  });

  it("finiteな長いtimeout/graceはNode timer上限で早期cancel/detachしない", async () => {
    vi.useFakeTimers();
    const workflow = await independent("a");
    const { ports } = harness();
    const result = deferred<WorkflowExecutorResult>();
    ports.tool.execute = vi.fn(() => result.promise);
    const outer = new AbortController();
    const done = runWorkflow({
      workflow,
      ports,
      maxJobs: 1,
      timeoutMs: 2147483650,
      abortGraceMs: 2147483650,
      signal: outer.signal,
    });
    await flush();
    await vi.advanceTimersByTimeAsync(100);
    expect(ports.abort).not.toHaveBeenCalled();
    const completed = vi.fn();
    void done.then(completed);
    outer.abort();
    await vi.advanceTimersByTimeAsync(100);
    expect(completed).not.toHaveBeenCalled();
    result.resolve(toolResult("cancelled"));
    expect((await done).error?.code).toBe("ABORTED");
    expect(vi.getTimerCount()).toBe(0);
  });

  it("preabort はmaterialize/admission/executor0で全jobsをcancelしfirst reasonを渡す", async () => {
    const workflow = await independent("a", "b");
    const materialize = vi.spyOn(workflow, "materializeJobInput");
    const { ports } = harness();
    const outer = new AbortController();
    const reason = new Error("stop");
    outer.abort(reason);
    const outcome = await runWorkflow({
      workflow,
      ports,
      maxJobs: 2,
      timeoutMs: 10000,
      signal: outer.signal,
    });
    expect(outcome).toMatchObject({
      ok: false,
      error: { code: "ABORTED", exitCode: 8 },
      meta: { execution: null },
    });
    expect(outcome.jobs.map((job) => job.status)).toEqual(["cancelled", "cancelled"]);
    expect(outcome.jobs.every((job) => job.meta.durationMs === null)).toBe(true);
    expect(materialize).not.toHaveBeenCalled();
    expect(ports.acquireJobLease).not.toHaveBeenCalled();
    expect(ports.tool.execute).not.toHaveBeenCalled();
    expect(ports.abort).toHaveBeenCalledExactlyOnceWith(reason);
  });

  it("outer timeout は共通graceで論理終了しactual settleまでleaseを保持する", async () => {
    vi.useFakeTimers();
    const workflow = await independent("a", "b");
    const { ports } = harness();
    const a = deferred<WorkflowExecutorResult>();
    const b = deferred<WorkflowExecutorResult>();
    let executorSignal: AbortSignal | undefined;
    ports.tool.execute = vi.fn(({ jobId, signal }) => {
      executorSignal = signal;
      return jobId === "a" ? a.promise : b.promise;
    });
    const completed = vi.fn();
    const done = runWorkflow({ workflow, ports, maxJobs: 2, timeoutMs: 20, abortGraceMs: 10 }).then(
      (value) => {
        completed();
        return value;
      },
    );
    await flush();
    await vi.advanceTimersByTimeAsync(20);
    expect(executorSignal?.aborted).toBe(true);
    expect(executorSignal?.reason).toMatchObject({ code: "TIMEOUT" });
    expect(completed).not.toHaveBeenCalled();
    a.resolve(toolResult("late cooperative result"));
    await flush();
    expect(ports.releaseJobLease).toHaveBeenCalledTimes(1);
    expect(completed).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(10);
    const outcome = await done;
    expect(outcome).toMatchObject({ ok: false, error: { code: "TIMEOUT", exitCode: 7 } });
    expect(outcome.jobs.map((job) => job.status)).toEqual(["cancelled", "cancelled"]);
    const snapshot = JSON.stringify(outcome);
    b.resolve({ result: { content: [] }, meta: { hidden: "LATE META" } });
    await flush();
    expect(ports.releaseJobLease).toHaveBeenCalledTimes(2);
    expect(JSON.stringify(outcome)).toBe(snapshot);
    expect(ports.abort).toHaveBeenCalledTimes(1);
  });

  it("abort はready leaseを即release、late admission grantもrelease、runningをdetachする", async () => {
    vi.useFakeTimers();
    const workflow = await independent("a", "b", "c");
    const { ports } = harness();
    const a = deferred<WorkflowExecutorResult>();
    const admission = deferred<ResourceLease>();
    let lateLease: ResourceLease | undefined;
    const acquire = ports.acquireJobLease;
    ports.acquireJobLease = vi.fn(async (context) => {
      const lease = await acquire(context);
      if (context.jobId !== "c") return lease;
      lateLease = lease;
      return admission.promise;
    });
    ports.tool.execute = vi.fn(() => a.promise);
    const outer = new AbortController();
    const reason = new Error("external stop");
    const done = runWorkflow({
      workflow,
      ports,
      maxJobs: 1,
      timeoutMs: 20,
      abortGraceMs: 5,
      signal: outer.signal,
    });
    await flush();
    expect(ports.tool.execute).toHaveBeenCalledTimes(1);
    outer.abort(reason);
    expect(ports.releaseJobLease).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(5);
    const outcome = await done;
    expect(outcome.error?.code).toBe("ABORTED");
    expect(ports.abort).toHaveBeenCalledExactlyOnceWith(reason);
    if (!lateLease) throw new Error("late lease was not prepared");
    admission.resolve(lateLease);
    await flush();
    expect(ports.releaseJobLease).toHaveBeenCalledTimes(2);
    a.reject(new Error("LATE ERROR"));
    await flush();
    expect(ports.releaseJobLease).toHaveBeenCalledTimes(3);
    expect(ports.tool.execute).toHaveBeenCalledTimes(1);
    expect(outcome.jobs.map((job) => job.status)).toEqual(["cancelled", "cancelled", "cancelled"]);
    await vi.advanceTimersByTimeAsync(100);
    expect(ports.abort).toHaveBeenCalledTimes(1);
    expect(vi.getTimerCount()).toBe(0);
  });

  it("child timeoutは当該jobだけfailedでoutercancelを起こさない", async () => {
    const workflow = await independent("a", "b");
    const { ports } = harness();
    ports.tool.execute = vi.fn(async ({ jobId }) => {
      if (jobId === "a") throw new CheaperError("TIMEOUT", "child timeout");
      return toolResult("independent");
    });
    const outcome = await runWorkflow({ workflow, ports, maxJobs: 2, timeoutMs: 10000 });
    expect(outcome.error?.code).toBe("TOOL_EXECUTION_FAILURE");
    expect(outcome.jobs.map((job) => job.status)).toEqual(["failed", "succeeded"]);
    expect(outcome.jobs[0]?.error?.code).toBe("TIMEOUT");
    expect(ports.abort).not.toHaveBeenCalled();
  });
});
