import {
  CheaperError,
  checkCapabilities,
  createPathDenylist,
  isCheaperError,
  loadModelConfig,
  parseDuration,
  restrictPolicy,
  type ToolName,
} from "@cheaper/core";
import {
  createModelRuntime,
  createPiToolHandlers,
  type ModelCatalog,
  type RunWorkerDeps,
  runWorker,
  type StreamFnLike,
  type WorkerRunMeta,
} from "@cheaper/runtime-pi";
import {
  createInvocationToolRuntime,
  type createToolRuntime,
  MutationJournal,
  type ResourceLease,
} from "@cheaper/tool-runtime";
import type { WorkflowExecutionContext, WorkflowExecutorResult } from "@cheaper/workflow";
import {
  projectArtifactMetadata,
  RunArtifactStore,
  runWorkflow,
  WorkflowExecutorError,
} from "@cheaper/workflow";
import type { WorkflowRunCliArgs } from "./args.js";
import type { CliHooks, CliIo } from "./cli.js";
import type { WorkflowEnvelope } from "./envelope.js";
import { readTaskInput } from "./input.js";
import { prepareWorkflowContext } from "./workflow-context.js";

type Runtime = Awaited<ReturnType<typeof createModelRuntime>>;
type StreamLoader = (env: NodeJS.ProcessEnv, runtime: Runtime) => Promise<StreamFnLike | undefined>;

export async function runWorkflowCommand(
  args: WorkflowRunCliArgs,
  io: CliIo,
  hooks: CliHooks,
  _startedAt: number,
  loadStream: StreamLoader,
): Promise<WorkflowEnvelope> {
  const prepared = await prepareWorkflowContext(args, io, hooks);
  const { root, dirs, policy, loaded, workers } = prepared;
  const inputs = args.input === undefined ? {} : await readTaskInput(args.input, io.stdin);
  let timeoutMs: number | undefined;
  if (args.timeout !== undefined) {
    try {
      timeoutMs = parseDuration(args.timeout);
    } catch {
      throw new CheaperError("INVALID_REQUEST", `invalid --timeout duration: ${args.timeout}`);
    }
  }
  const effectivePolicy = restrictPolicy(policy, {
    ...(timeoutMs !== undefined ? { timeoutMs } : {}),
  });
  const workflowPolicy = effectivePolicy.workflow;
  if (workflowPolicy === undefined)
    throw new CheaperError("INVALID_RUNTIME_POLICY", "workflow policy was not resolved");
  const workflow = await loaded.compiler.compile(loaded.definition, inputs);
  const now = hooks.now ?? Date.now;
  const external = hooks.signal;
  const abortController = new AbortController();
  const onSignal = () => abortController.abort(external?.reason);
  external?.addEventListener("abort", onSignal, { once: true });
  if (external?.aborted) onSignal();
  const onSigint = () => abortController.abort();
  const onSigterm = () => abortController.abort();
  process.on("SIGINT", onSigint);
  process.on("SIGTERM", onSigterm);

  let store: RunArtifactStore | undefined;
  let invocation: ReturnType<typeof createInvocationToolRuntime> | undefined;
  let modelPromise:
    | Promise<{
        runtime: Runtime;
        modelConfig: Awaited<ReturnType<typeof loadModelConfig>>;
        streamFn: StreamFnLike | undefined;
      }>
    | undefined;
  const ledger = new Map<string, ReturnType<RunArtifactStore["put"]>>();
  const ensureInvocation = () => {
    if (!invocation) {
      const artifactStore = new RunArtifactStore({
        ...workflowPolicy.artifacts,
        signal: abortController.signal,
      });
      store = artifactStore;
      const publisher = {
        maxArtifactBytes: artifactStore.maxArtifactBytes,
        put: (value: { producer: { jobId: string; name: string }; data: string | Uint8Array }) => {
          const ref = artifactStore.put(value);
          ledger.set(ref.artifact, ref);
          return ref;
        },
      };
      invocation = createInvocationToolRuntime({
        root,
        symlinkPolicy: effectivePolicy.filesystem.symlinks,
        process: effectivePolicy.process,
        concurrency: effectivePolicy.execution.concurrency,
        artifacts: { reader: artifactStore.reader, publisher },
        signal: abortController.signal,
      });
    }
    return invocation;
  };
  const ensureModel = () =>
    (modelPromise ??= (async () => {
      const modelConfig = await loadModelConfig({
        userConfigDir: dirs.userConfigDir,
        projectDir: dirs.projectDir,
      });
      const runtime = await (hooks.createModelRuntime ?? createModelRuntime)({ env: io.env });
      return { runtime, modelConfig, streamFn: await loadStream(io.env, runtime) };
    })());

  const ports = {
    async acquireJobLease(
      ctx: Omit<WorkflowExecutionContext, "parentLease">,
    ): Promise<ResourceLease> {
      let tools: ToolName[];
      if (ctx.job.kind === "tool") {
        if (!effectivePolicy.tools.allow.includes(ctx.job.tool as ToolName))
          throw new CheaperError("POLICY_VIOLATION", `tool not allowed: ${ctx.job.tool}`);
        tools = [ctx.job.tool as ToolName];
      } else {
        const worker = workers.get(ctx.job.worker);
        if (!worker)
          throw new CheaperError("WORKER_NOT_FOUND", `worker not found: ${ctx.job.worker}`);
        tools = checkCapabilities(worker, effectivePolicy);
      }
      const rt = ensureInvocation();
      return rt.acquireJobLease({
        ownerId: ctx.jobId,
        names: tools,
        ...(ctx.job.manifest ? { resourceManifest: ctx.job.manifest } : {}),
        signal: ctx.signal,
      });
    },
    releaseJobLease(lease: ResourceLease) {
      invocation?.releaseJobLease(lease);
    },
    abort(reason: unknown) {
      abortController.abort(reason);
      invocation?.abort(reason);
      store?.dispose();
    },
    executionMetrics(jobId?: string) {
      return invocation?.executionMetrics(jobId) ?? null;
    },
    worker: {
      async execute(ctx: WorkflowExecutionContext): Promise<WorkflowExecutorResult> {
        if (ctx.job.kind !== "worker")
          throw new CheaperError("INVALID_REQUEST", "worker executor received a tool job");
        const worker = workers.get(ctx.job.worker);
        if (!worker)
          throw new CheaperError("WORKER_NOT_FOUND", `worker not found: ${ctx.job.worker}`);
        const { runtime, modelConfig, streamFn } = await ensureModel();
        const deps: RunWorkerDeps = {
          worker,
          input: ctx.input,
          root,
          policy: effectivePolicy,
          modelConfig,
          catalog: runtime as ModelCatalog,
          modelRuntime: runtime,
          signal: ctx.signal,
          ...(streamFn ? { streamFn } : {}),
          execution: {
            runtime: ensureInvocation(),
            ownerId: ctx.jobId,
            parentLease: ctx.parentLease,
            ...(ctx.job.manifest ? { resourceManifest: ctx.job.manifest } : {}),
          },
        };
        try {
          const result = await (hooks.runWorker ?? runWorker)(deps);
          return {
            result: result.result as never,
            meta: JSON.parse(JSON.stringify(selectWorkerMeta(result.meta))),
          };
        } catch (error) {
          const meta = (error as { runMeta?: WorkerRunMeta }).runMeta;
          const code = isCheaperError(error) ? error.code : "TOOL_EXECUTION_FAILURE";
          throw new WorkflowExecutorError(
            code,
            error instanceof Error ? error.message : "Worker failed",
            { meta: meta ? JSON.parse(JSON.stringify(selectWorkerMeta(meta))) : {} },
          );
        }
      },
    },
    tool: {
      async execute(ctx: WorkflowExecutionContext): Promise<WorkflowExecutorResult> {
        const rt = ensureInvocation();
        if (ctx.job.kind !== "tool")
          throw new CheaperError("INVALID_REQUEST", "tool executor received a Worker job");
        const name = ctx.job.tool as ToolName;
        const journal = new MutationJournal();
        const names = [name];
        let scope!: ReturnType<typeof createToolRuntime>;
        const options = {
          root,
          symlinkPolicy: effectivePolicy.filesystem.symlinks,
          names,
          maxToolCalls: effectivePolicy.execution.maxToolCalls,
          onLimit() {},
          journal,
          denylist: createPathDenylist(effectivePolicy.filesystem.denyPaths),
          resourceManifest: ctx.job.manifest,
          parentLease: ctx.parentLease,
        };
        const handlerOptions = {
          ...options,
          ownerId: ctx.jobId,
          observeAccess: (
            ...values: Parameters<ReturnType<typeof createToolRuntime>["observeAccess"]>
          ) => scope.observeAccess(...values),
        };
        scope = rt.createWorkerScope({
          ...options,
          ownerId: ctx.jobId,
          handlers: createPiToolHandlers(handlerOptions),
        });
        try {
          const result = await scope.execute(name, ctx.input, { signal: ctx.signal });
          return {
            result: result as never,
            meta: JSON.parse(
              JSON.stringify({
                toolCalls: scope.budget.toolCalls,
                changes: await journal.finalize(),
              }),
            ),
          };
        } catch (error) {
          throw new WorkflowExecutorError(
            isCheaperError(error) ? error.code : "TOOL_EXECUTION_FAILURE",
            error instanceof Error ? error.message : "tool execution failed",
            {
              meta: JSON.parse(
                JSON.stringify({
                  toolCalls: scope.budget.toolCalls,
                  changes: await journal.finalize(),
                }),
              ),
            },
          );
        } finally {
          await scope.dispose();
        }
      },
    },
  };

  try {
    const outcome = await runWorkflow({
      workflow,
      ports,
      maxJobs: workflowPolicy.maxJobs,
      timeoutMs: effectivePolicy.execution.timeoutMs,
      signal: abortController.signal,
      now,
    });
    const success = outcome.ok;
    const artifacts = [...ledger.values()].map((reference) =>
      projectArtifactMetadata(reference, { includePreview: false }),
    );
    return {
      schemaVersion: 1,
      workflow: true,
      ok: success,
      worker: null,
      result: outcome.result,
      jobs: outcome.jobs,
      meta: { ...outcome.meta, artifacts },
      ...(!success && outcome.error ? { error: outcome.error } : {}),
    } as WorkflowEnvelope;
  } finally {
    external?.removeEventListener("abort", onSignal);
    process.off("SIGINT", onSigint);
    process.off("SIGTERM", onSigterm);
    store?.dispose();
    await invocation?.dispose();
  }
}

function selectWorkerMeta(meta: WorkerRunMeta) {
  return {
    model: meta.model,
    usage: meta.usage,
    turns: meta.turns,
    toolCalls: meta.toolCalls,
    rejectedToolCalls: meta.rejectedToolCalls,
    repairs: meta.repairs,
    transientRetries: meta.transientRetries,
    changes: meta.changes,
    ...(meta.savings ? { savings: meta.savings } : {}),
  };
}
