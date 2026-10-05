import path from "node:path";
import {
  CheaperError,
  canonicalizeWorkspaceRoot,
  createPathDenylist,
  loadPolicy,
  resolveConfigDirs,
  resolveWorker,
  type WorkerDefinition,
} from "@cheaper/core";
import { createToolCatalog } from "@cheaper/tool-runtime";
import { loadWorkflowFile, WorkflowCompiler, WorkspaceResourceInference } from "@cheaper/workflow";
import type { WorkflowInterfaceCliArgs, WorkflowRunCliArgs } from "./args.js";
import type { CliHooks, CliIo } from "./cli.js";
import { defaultBuiltinDir } from "./count.js";

export async function prepareWorkflowContext(
  args: WorkflowInterfaceCliArgs | WorkflowRunCliArgs,
  io: CliIo,
  hooks: CliHooks,
) {
  const builtinDir = hooks.builtinDir ?? defaultBuiltinDir;
  const root = await canonicalizeWorkspaceRoot(args.cwd ?? io.cwd);
  const dirs = resolveConfigDirs({ cwd: root, builtinDir, env: io.env });
  const policy = await loadPolicy({
    builtinPolicyPath: path.join(builtinDir, "policy.yaml"),
    userConfigDir: dirs.userConfigDir,
    projectDir: dirs.projectDir,
  });
  const workers = new Map<string, WorkerDefinition>();
  const loaded = await loadWorkflowFile(
    {
      root,
      path: args.path,
      symlinkPolicy: policy.filesystem.symlinks,
      denylist: createPathDenylist(policy.filesystem.denyPaths),
    },
    async (raw) => {
      const failures = new Map<string, unknown>();
      const jobs =
        raw && typeof raw === "object" ? (raw as Record<string, unknown>).jobs : undefined;
      if (jobs && typeof jobs === "object" && !Array.isArray(jobs)) {
        const names = new Set<string>();
        for (const job of Object.values(jobs))
          if (
            job &&
            typeof job === "object" &&
            (job as { kind?: unknown }).kind === "worker" &&
            typeof (job as { worker?: unknown }).worker === "string"
          )
            names.add((job as { worker: string }).worker);
        for (const name of [...names].sort())
          try {
            workers.set(name, await resolveWorker(name, dirs));
          } catch (error) {
            if (
              !(error instanceof CheaperError) ||
              !["WORKER_NOT_FOUND", "INVALID_WORKER_NAME"].includes(error.code)
            )
              failures.set(name, error);
          }
      }
      const tools = createToolCatalog({ process: policy.process });
      return new WorkflowCompiler({
        tools,
        workers: {
          get(name) {
            if (failures.has(name)) throw failures.get(name);
            return workers.get(name);
          },
        },
        resources: new WorkspaceResourceInference({
          workspaceRoot: root,
          symlinkPolicy: policy.filesystem.symlinks,
          tools,
        }),
      });
    },
  );
  return { root, dirs, policy, loaded, workers };
}
