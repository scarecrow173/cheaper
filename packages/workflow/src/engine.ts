import { CheaperError, type ErrorCode } from "@cheaper/core";
import type { ExecutionMetrics, JsonValue, ResourceLease } from "@cheaper/tool-runtime";
import { cloneJson } from "./json.js";
import type { CompiledJob, CompiledWorkflow, WorkflowJobState } from "./types.js";

export interface WorkflowExecutionContext {
  jobId: string;
  job: CompiledJob;
  input: JsonValue;
  signal: AbortSignal;
  parentLease: ResourceLease;
}
export interface WorkflowExecutorResult {
  result: JsonValue;
  /** adapter が公開可能な field のみを選択する。input/result/error全体を入れない。 */
  meta?: Record<string, JsonValue>;
}
/** failure の選択済みmetadataをPi型やraw error payloadから分離して運ぶ。 */
export class WorkflowExecutorError extends CheaperError {
  readonly meta: Record<string, JsonValue>;
  constructor(
    code: ErrorCode,
    message: string,
    options?: {
      meta?: Record<string, JsonValue>;
      details?: Record<string, unknown>;
      cause?: unknown;
    },
  ) {
    super(code, message, options);
    this.meta = cloneJson(options?.meta ?? {}, "INVALID_OUTPUT") as Record<string, JsonValue>;
  }
}
export interface WorkerExecutor {
  execute(context: WorkflowExecutionContext): Promise<WorkflowExecutorResult>;
}
export interface ToolExecutor {
  execute(context: WorkflowExecutionContext): Promise<WorkflowExecutorResult>;
}
export interface WorkflowEnginePorts {
  worker: WorkerExecutor;
  tool: ToolExecutor;
  acquireJobLease(context: Omit<WorkflowExecutionContext, "parentLease">): Promise<ResourceLease>;
  releaseJobLease(lease: ResourceLease): void;
  abort(reason: unknown): void;
  executionMetrics(jobId?: string): ExecutionMetrics | null;
}
export interface WorkflowDiagnostic {
  code: ErrorCode | "DEPENDENCY_FAILED";
  message: string;
}
export interface WorkflowJobOutcome {
  id: string;
  kind: CompiledJob["kind"];
  status: "succeeded" | "failed" | "skipped" | "cancelled";
  meta: Record<string, JsonValue | ExecutionMetrics> & {
    durationMs: number | null;
    execution: ExecutionMetrics | null;
  };
  error?: WorkflowDiagnostic;
}
export interface WorkflowOutcome {
  ok: boolean;
  result: Record<string, JsonValue>;
  jobs: WorkflowJobOutcome[];
  meta: {
    durationMs: number;
    execution: ExecutionMetrics | null;
    outputErrors: { name: string; error: WorkflowDiagnostic }[];
  };
  error?: { code: ErrorCode; message: string; exitCode: number; retryable: boolean };
}
export interface WorkflowRunOptions {
  workflow: CompiledWorkflow;
  ports: WorkflowEnginePorts;
  maxJobs: number;
  timeoutMs: number;
  signal?: AbortSignal;
  abortGraceMs?: number;
  now?: () => number;
}

type Phase = "pending" | "admitting" | "ready" | "running" | WorkflowJobOutcome["status"];
interface JobExecution {
  id: string;
  job: CompiledJob;
  phase: Phase;
  remaining: number;
  children: JobExecution[];
  dependencyFailed: boolean;
  input?: JsonValue;
  lease?: ResourceLease;
  preparedAt?: number;
  outcome?: WorkflowJobOutcome;
}

/** validator details/cause は公開せず、診断の保持自体も UTF-8 cap 内に置く。 */
function diagnostic(
  error: unknown,
  fallback: ErrorCode = "TOOL_EXECUTION_FAILURE",
): WorkflowDiagnostic {
  const code = error instanceof CheaperError ? error.code : fallback;
  const source = error instanceof Error ? error.message : "job execution failed";
  let message = "";
  let bytes = 0;
  for (const point of source) {
    const size = Buffer.byteLength(point, "utf8");
    if (bytes + size > 2048) break;
    message += point;
    bytes += size;
  }
  return { code, message };
}

/** Node の最大delayを超えるdeadlineも1msへ丸めず、timeout境界でのみ再armする。 */
function deadline(
  delayMs: number,
  callback: () => void,
  startedAt = performance.now(),
): () => void {
  const end = startedAt + delayMs;
  let timer: ReturnType<typeof setTimeout> | undefined;
  const arm = () => {
    const remaining = end - performance.now();
    if (remaining <= 0) {
      callback();
      return;
    }
    const delay = Math.min(2147483647, Math.max(1, Math.ceil(remaining)));
    timer = setTimeout(() => {
      if (performance.now() < end) arm();
      else callback();
    }, delay);
  };
  arm();
  return () => clearTimeout(timer);
}

/** runtime/store の生成を持たず、compiled semantic edges だけを実行する。 */
export async function runWorkflow(options: WorkflowRunOptions): Promise<WorkflowOutcome> {
  const { workflow, ports, maxJobs, timeoutMs } = options;
  const abortGraceMs = options.abortGraceMs ?? 5000;
  if (
    !Number.isSafeInteger(maxJobs) ||
    maxJobs <= 0 ||
    !Number.isFinite(timeoutMs) ||
    timeoutMs <= 0 ||
    !Number.isFinite(abortGraceMs) ||
    abortGraceMs < 0
  )
    throw new CheaperError("INVALID_REQUEST", "invalid workflow execution options");
  const timeoutStartedAt = performance.now();
  const now = options.now ?? (() => performance.now());
  const startedAt = now();
  const controller = new AbortController();
  const jobs = Object.keys(workflow.jobs)
    .sort()
    .map(
      (id): JobExecution => ({
        id,
        job: workflow.jobs[id] as CompiledJob,
        phase: "pending",
        remaining: 0,
        children: [],
        dependencyFailed: false,
      }),
    );
  const byId = new Map(jobs.map((job) => [job.id, job]));
  for (const edge of workflow.edges) {
    const parent = byId.get(edge.from) as JobExecution;
    const child = byId.get(edge.to) as JobExecution;
    parent.children.push(child);
    child.remaining++;
  }
  const results = new Map<string, JsonValue>();
  let running = 0;
  let terminal = 0;
  let pumping = false;
  let scheduled = false;
  let outerError: CheaperError | undefined;
  let cancelTimeout: (() => void) | undefined;
  let cancelGrace: (() => void) | undefined;
  let finished!: () => void;
  const settled = new Promise<void>((resolve) => {
    finished = resolve;
  });

  function schedule() {
    if (scheduled) return;
    scheduled = true;
    queueMicrotask(() => {
      scheduled = false;
      pump();
    });
  }
  function finishJob(
    job: JobExecution,
    status: WorkflowJobOutcome["status"],
    error?: WorkflowDiagnostic,
    meta: Record<string, JsonValue> = {},
  ) {
    if (job.outcome) return;
    if (job.phase === "running") running--;
    job.phase = status;
    job.outcome = {
      id: job.id,
      kind: job.job.kind,
      status,
      meta: {
        ...(cloneJson(meta, "INVALID_OUTPUT") as Record<string, JsonValue>),
        durationMs: job.preparedAt === undefined ? null : Math.max(0, now() - job.preparedAt),
        execution: structuredClone(ports.executionMetrics(job.id)),
      },
      ...(error ? { error } : {}),
    };
    terminal++;
    for (const child of job.children) {
      child.remaining--;
      if (status !== "succeeded") child.dependencyFailed = true;
    }
    if (terminal === jobs.length) finished();
    schedule();
  }
  function admit(job: JobExecution) {
    try {
      job.input = workflow.materializeJobInput(job.id, results);
      job.preparedAt = now();
      job.phase = "admitting";
      const admission = ports.acquireJobLease({
        jobId: job.id,
        job: job.job,
        input: job.input,
        signal: controller.signal,
      });
      void admission.then(
        (lease) => {
          if (outerError) {
            ports.releaseJobLease(lease);
            return;
          }
          job.lease = lease;
          job.phase = "ready";
          schedule();
        },
        (error: unknown) => finishJob(job, "failed", diagnostic(error)),
      );
    } catch (error) {
      finishJob(job, "failed", diagnostic(error));
    }
  }
  function execute(job: JobExecution) {
    job.phase = "running";
    running++;
    const lease = job.lease as ResourceLease;
    // finally は論理terminalではなく実executorのsettleに結びつける。
    void (async () => {
      try {
        const executor = job.job.kind === "worker" ? ports.worker : ports.tool;
        const value = await executor.execute({
          jobId: job.id,
          job: job.job,
          input: job.input as JsonValue,
          signal: controller.signal,
          parentLease: lease,
        });
        if (job.outcome) return;
        if (outerError) {
          finishJob(job, "cancelled", diagnostic(outerError));
          return;
        }
        const output = workflow.validateJobOutput(job.id, value.result);
        const meta = cloneJson(value.meta ?? {}, "INVALID_OUTPUT") as Record<string, JsonValue>;
        results.set(job.id, output);
        finishJob(job, "succeeded", undefined, meta);
      } catch (error) {
        finishJob(
          job,
          outerError ? "cancelled" : "failed",
          diagnostic(outerError ?? error),
          error instanceof WorkflowExecutorError ? error.meta : {},
        );
      } finally {
        ports.releaseJobLease(lease);
        delete job.lease;
        schedule();
      }
    })();
  }
  function pump() {
    if (pumping || outerError) return;
    pumping = true;
    try {
      // 同期validation失敗からのskipも一つのeventで伝播する。
      let progressed: boolean;
      do {
        progressed = false;
        for (const job of jobs) {
          if (job.phase !== "pending" || job.remaining !== 0) continue;
          if (job.dependencyFailed)
            finishJob(job, "skipped", {
              code: "DEPENDENCY_FAILED",
              message: "required dependency failed",
            });
          else admit(job);
          progressed = true;
        }
      } while (progressed);
      for (const job of jobs) {
        if (running >= maxJobs) break;
        if (job.phase === "ready") execute(job);
      }
      if (jobs.length === 0) finished();
    } finally {
      pumping = false;
    }
  }
  function stop(reason: unknown) {
    if (outerError) return;
    const abortedAt = performance.now();
    outerError =
      reason instanceof CheaperError && reason.code === "TIMEOUT"
        ? new CheaperError("TIMEOUT", "workflow timed out")
        : new CheaperError("ABORTED", "workflow aborted");
    controller.abort(reason);
    ports.abort(reason);
    for (const job of jobs) {
      if (job.outcome || job.phase === "running") continue;
      if (job.lease) {
        ports.releaseJobLease(job.lease);
        delete job.lease;
      }
      finishJob(job, "cancelled", diagnostic(outerError));
    }
    const detach = () => {
      for (const job of jobs)
        if (job.phase === "running") finishJob(job, "cancelled", diagnostic(outerError));
    };
    if (running > 0) {
      if (abortGraceMs === 0) detach();
      else cancelGrace = deadline(abortGraceMs, detach, abortedAt);
    }
    if (terminal === jobs.length) finished();
  }
  const onAbort = () => stop(options.signal?.reason);
  try {
    options.signal?.addEventListener("abort", onAbort, { once: true });
    if (options.signal?.aborted) onAbort();
    else {
      cancelTimeout = deadline(
        timeoutMs,
        () => stop(new CheaperError("TIMEOUT", "workflow timed out")),
        timeoutStartedAt,
      );
      pump();
    }
    await settled;
  } finally {
    cancelTimeout?.();
    cancelGrace?.();
    options.signal?.removeEventListener("abort", onAbort);
  }
  const states = new Map<string, WorkflowJobState>(
    jobs.map((job) => [
      job.id,
      {
        terminal: true,
        ...(results.has(job.id) ? { result: results.get(job.id) as JsonValue } : {}),
      },
    ]),
  );
  const outputs: [string, JsonValue][] = [];
  const outputErrors: WorkflowOutcome["meta"]["outputErrors"] = [];
  for (const name of Object.keys(workflow.outputs).sort()) {
    try {
      outputs.push([name, workflow.materializeOutput(name, states)]);
    } catch (error) {
      outputErrors.push({ name, error: diagnostic(error, "INVALID_OUTPUT") });
    }
  }
  const error =
    outerError ??
    (jobs.some((job) => job.phase !== "succeeded")
      ? new CheaperError("TOOL_EXECUTION_FAILURE", "workflow job failed")
      : outputErrors.length
        ? new CheaperError("INVALID_OUTPUT", "workflow output validation failed")
        : undefined);
  return {
    ok: error === undefined,
    result: Object.fromEntries(outputs),
    jobs: jobs.map((job) => job.outcome as WorkflowJobOutcome),
    meta: {
      durationMs: Math.max(0, now() - startedAt),
      execution: structuredClone(ports.executionMetrics()),
      outputErrors,
    },
    ...(error
      ? {
          error: {
            code: error.code,
            message: error.message,
            exitCode: error.exitCode,
            retryable: error.retryable,
          },
        }
      : {}),
  };
}
