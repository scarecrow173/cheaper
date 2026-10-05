import { spawn } from "node:child_process";
import { stat } from "node:fs/promises";
import path from "node:path";
import {
  CheaperError,
  claimContains,
  MAX_PROCESS_OUTPUT_BYTES,
  normalizeResourceManifest,
  type PathDenylist,
  type ProcessPolicy,
  parseDuration,
  processTaskResources,
  type ResolvedProcessPolicy,
  resolveProcessTask,
  resolveWorkspacePath,
  type SymlinkPolicy,
} from "@cheaper/core";
import { Type } from "typebox";
import type { CanonicalToolResult } from "./catalog.js";
import type { CanonicalToolDefinition } from "./definition.js";
import { resolveProcessExecutable } from "./process-executable.js";
import { createProcessTree, type ProcessTree } from "./process-tree.js";
import type { OperationDeclaration } from "./scheduler.js";

export { MAX_PROCESS_OUTPUT_BYTES } from "@cheaper/core";
export const processSchema = Type.Object(
  {
    task: Type.String({
      minLength: 1,
      maxLength: 64,
      pattern: "^[a-z][a-z0-9_-]*$",
      description: "Name of a policy-defined process task",
    }),
  },
  { additionalProperties: false },
);
const streamSchema = Type.Object(
  {
    text: Type.String({ maxLength: MAX_PROCESS_OUTPUT_BYTES }),
    bytes: Type.Integer({ minimum: 0 }),
    capturedBytes: Type.Integer({ minimum: 0, maximum: MAX_PROCESS_OUTPUT_BYTES }),
    truncated: Type.Boolean(),
  },
  { additionalProperties: false },
);
export const processDetailsSchema = Type.Object(
  {
    task: Type.String(),
    state: Type.Union([Type.Literal("completed"), Type.Literal("failed")]),
    exitCode: Type.Union([Type.Integer(), Type.Null()]),
    signal: Type.Union([Type.String(), Type.Null()]),
    stdout: streamSchema,
    stderr: streamSchema,
  },
  { additionalProperties: false },
);
interface Options {
  root: string;
  symlinkPolicy: SymlinkPolicy;
  process: ResolvedProcessPolicy;
  onTimeout?: ((reason: CheaperError) => void) | undefined;
  denylist?: PathDenylist | undefined;
  writeScope?: ReadonlySet<string> | undefined;
}
export function processOperationDeclaration(
  policy: ProcessPolicy,
  input: { task: string },
): OperationDeclaration {
  const task = resolveProcessTask(policy, input.task);
  return {
    access: task.outputs.length === 0 ? "shared" : "exclusive",
    pool: { name: "process", maxConcurrent: policy.maxConcurrent },
  };
}
function minimalEnv(explicit: Record<string, string> = {}): NodeJS.ProcessEnv {
  const env: NodeJS.ProcessEnv = { NODE_V8_COVERAGE: undefined };
  for (const [key, value] of Object.entries(explicit))
    env[process.platform === "win32" ? key.toUpperCase() : key] = value;
  if (process.platform === "win32") {
    // libuv は未指定の Windows required env を親から補完する。空値でその継承を止める。
    for (const key of [
      "HOMEDRIVE",
      "HOMEPATH",
      "LOGONSERVER",
      "PATH",
      "SYSTEMDRIVE",
      "TEMP",
      "USERDOMAIN",
      "USERNAME",
      "USERPROFILE",
      "WINDIR",
    ]) {
      if (!Object.hasOwn(env, key)) env[key] = "";
    }
    if (!Object.hasOwn(env, "SYSTEMROOT")) env.SYSTEMROOT = process.env.SystemRoot ?? "C:\\Windows";
  }
  return env;
}
function deny(display: string): never {
  throw new CheaperError("POLICY_VIOLATION", "Access denied for declared process path", {
    details: { path: display },
  });
}
async function prepare(
  options: Options,
  name: string,
): Promise<{ executable: string; cwd: string }> {
  const task = resolveProcessTask(options.process, name);
  const cwd = await resolveWorkspacePath(options.root, task.cwd ?? ".", options.symlinkPolicy);
  if (options.denylist?.isDenied(cwd.display)) deny(cwd.display);
  if (!(await stat(cwd.real)).isDirectory())
    throw new CheaperError("INVALID_REQUEST", "process cwd must be an existing directory");
  const declaration = processTaskResources(options.process, name);
  const manifest = await normalizeResourceManifest(
    options.root,
    declaration,
    options.symlinkPolicy,
  );
  for (const input of [...task.inputs, ...task.outputs])
    if (options.denylist?.isDenied(input)) deny(input);
  for (const claim of [...manifest.reads, ...manifest.writes, ...manifest.produces]) {
    const display =
      path.relative(options.root, claim.resource.path).split(path.sep).join("/") || ".";
    if (options.denylist?.isDenied(display)) deny(display);
  }
  if (options.writeScope !== undefined) {
    const allowed = await normalizeResourceManifest(
      options.root,
      {
        reads: [],
        writes: [],
        produces: [...options.writeScope].map((value) => ({
          path: value,
          scope: "exact" as const,
        })),
      },
      options.symlinkPolicy,
    );
    if (
      manifest.writes.some(
        (claim) => !allowed.produces.some((scope) => claimContains(scope, claim)),
      )
    )
      throw new CheaperError("POLICY_VIOLATION", "process outputs exceed worker writeScope");
  }
  return {
    cwd: cwd.real,
    executable: await resolveProcessExecutable(task.command[0] as string, options),
  };
}

type StreamCapture = { text: string; bytes: number; capturedBytes: number; truncated: boolean };
function capture() {
  const chunks: Buffer[] = [];
  let bytes = 0,
    capturedBytes = 0;
  return {
    get bytes() {
      return bytes;
    },
    append(chunk: Buffer, remaining: number): number {
      bytes += chunk.length;
      const length = Math.min(chunk.length, remaining);
      if (length > 0) {
        chunks.push(Buffer.from(chunk.subarray(0, length)));
        capturedBytes += length;
      }
      return length;
    },
    finish(): StreamCapture {
      // invalid UTF-8 の replacement 拡大と途中 codepoint を同じ byte cap 内に収める。
      const encoded = Buffer.from(Buffer.concat(chunks, capturedBytes).toString("utf8"));
      const text = new TextDecoder("utf-8").decode(encoded.subarray(0, capturedBytes), {
        stream: true,
      });
      return {
        text,
        bytes,
        capturedBytes,
        truncated: bytes > capturedBytes || encoded.length > capturedBytes,
      };
    },
  };
}

async function execute(
  options: Options,
  name: string,
  outerSignal: AbortSignal,
): Promise<CanonicalToolResult> {
  const timeoutController = new AbortController();
  const signal = AbortSignal.any([outerSignal, timeoutController.signal]);
  if (signal.aborted) throw signal.reason;
  const task = resolveProcessTask(options.process, name);
  const prepared = await prepare(options, name);
  if (signal.aborted) throw signal.reason;
  const stdout = capture(),
    stderr = capture();
  let remaining = options.process.maxOutputBytes;
  const child = (() => {
    try {
      return spawn(prepared.executable, task.command.slice(1), {
        cwd: prepared.cwd,
        env: minimalEnv(task.env),
        shell: false,
        windowsHide: true,
        detached: process.platform !== "win32",
        stdio: ["ignore", "pipe", "pipe"],
      });
    } catch (error) {
      throw new CheaperError("TOOL_EXECUTION_FAILURE", "managed process spawn failed", {
        details: {
          task: name,
          state: "failed",
          exitCode: null,
          signal: null,
          stdout: stdout.finish(),
          stderr: stderr.finish(),
          spawnCode: (error as NodeJS.ErrnoException).code ?? "SPAWN_ERROR",
        },
        cause: error,
      });
    }
  })();
  const onStdout = (chunk: Buffer) => {
    remaining -= stdout.append(chunk, remaining);
  };
  const onStderr = (chunk: Buffer) => {
    remaining -= stderr.append(chunk, remaining);
  };
  child.stdout.on("data", onStdout);
  child.stderr.on("data", onStderr);
  const startedAt = performance.now();
  let phase = "running";
  let tree: ProcessTree | undefined;
  const diagnostic = (event: string) => {
    try {
      if (!process.stderr.writableNeedDrain && !process.stderr.destroyed)
        process.stderr.write(
          `${JSON.stringify({
            event,
            task: name,
            pid: child.pid ?? null,
            elapsedMs: Math.round(performance.now() - startedAt),
            phase,
            outputBytes: stdout.bytes + stderr.bytes,
          })}\n`,
        );
    } catch {
      /* 診断の失敗は child の結果を変えない。 */
    }
  };
  const timeout = setTimeout(() => {
    if (signal.aborted || phase !== "running") return;
    const reason = new CheaperError("TIMEOUT", "managed process execution timed out", {
      details: { task: name },
    });
    options.onTimeout?.(reason);
    timeoutController.abort(reason);
  }, parseDuration(options.process.timeout));
  const heartbeat = setInterval(
    () => diagnostic("process.heartbeat"),
    parseDuration(options.process.heartbeat.interval),
  );
  child.once("spawn", () => {
    if (child.pid !== undefined) tree ??= createProcessTree(child.pid);
  });
  let cleanup: Promise<void> | undefined;
  let cleanupError: unknown;
  const cleanupTree = () => {
    if (cleanup) return cleanup;
    phase = "cleanup";
    clearTimeout(timeout);
    tree ??= child.pid === undefined ? undefined : createProcessTree(child.pid);
    cleanup = (
      tree?.terminate(parseDuration(options.process.killGrace)) ?? Promise.resolve()
    ).catch((error: unknown) => {
      cleanupError = error;
      diagnostic("process.cleanup-failed");
      child.kill("SIGKILL");
      child.stdout.destroy();
      child.stderr.destroy();
    });
    return cleanup;
  };
  const onAbort = () => {
    void cleanupTree();
  };
  child.once("exit", cleanupTree);
  signal.addEventListener("abort", onAbort, { once: true });
  if (signal.aborted) onAbort();
  try {
    let spawnError: NodeJS.ErrnoException | undefined;
    const ended = await new Promise<{ code: number | null; signal: NodeJS.Signals | null }>(
      (resolve) => {
        child.once("error", (error: NodeJS.ErrnoException) => {
          spawnError = error;
          void cleanupTree();
        });
        child.once("close", (code, exitSignal) => resolve({ code, signal: exitSignal }));
      },
    );
    await cleanup;
    const details = {
      task: name,
      state: ended.code === 0 && !spawnError ? "completed" : "failed",
      exitCode: ended.code,
      signal: ended.signal,
      stdout: stdout.finish(),
      stderr: stderr.finish(),
    };
    if (signal.aborted) throw signal.reason;
    if (cleanupError)
      throw new CheaperError("TOOL_EXECUTION_FAILURE", "managed process cleanup failed", {
        details,
        cause: cleanupError,
      });
    if (spawnError || ended.code !== 0)
      throw new CheaperError("TOOL_EXECUTION_FAILURE", "managed process failed", {
        details: {
          ...details,
          ...(spawnError ? { spawnCode: spawnError.code ?? "SPAWN_ERROR" } : {}),
        },
      });
    // 表示だけを折り返す。raw stdout/stderr は details に保持し、同期 BPE の長い run を防ぐ。
    const display = (value: StreamCapture): string => {
      const parts: string[] = [];
      for (let start = 0; start < value.text.length; ) {
        let end = Math.min(start + 64, value.text.length);
        const last = value.text.charCodeAt(end - 1);
        if (end < value.text.length && last >= 0xd800 && last <= 0xdbff) end--;
        parts.push(value.text.slice(start, end));
        start = end;
      }
      return `${value.bytes} bytes, ${value.capturedBytes} captured, truncated=${value.truncated}\n${parts.join("\n")}`;
    };
    const text = `process ${name}: completed, exitCode=0\nstdout: ${display(details.stdout)}\nstderr: ${display(details.stderr)}`;
    return { content: [{ type: "text", text }], details };
  } finally {
    clearTimeout(timeout);
    clearInterval(heartbeat);
    signal.removeEventListener("abort", onAbort);
    child.stdout.removeListener("data", onStdout);
    child.stderr.removeListener("data", onStderr);
    child.stdout.destroy();
    child.stderr.destroy();
    child.removeAllListeners();
  }
}
export function createProcessTool(options: Options): CanonicalToolDefinition {
  const opts = {
    ...options,
    ...(options.writeScope ? { writeScope: new Set(options.writeScope) } : {}),
  };
  return {
    name: "process",
    label: "Process",
    description: `Run a policy-defined native process task: ${Object.keys(opts.process.tasks).join(", ")}. No arbitrary command or shell.`,
    parameters: processSchema,
    execute: (input, signal) =>
      execute(opts, (input as { task: string }).task, signal ?? new AbortController().signal),
  };
}
