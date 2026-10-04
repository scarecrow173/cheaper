import path from "node:path";
import Ajv2020Ns from "ajv/dist/2020.js";
import { CheaperError } from "../errors.js";
import type { ResourceManifestDeclaration, ScopeDeclaration } from "../resources/normalization.js";

export interface ProcessTask {
  command: string[];
  inputs: string[];
  outputs: string[];
  cwd?: string;
  env?: Record<string, string>;
}
export interface ProcessPolicy {
  maxConcurrent: number;
  tasks: Record<string, ProcessTask>;
}

const boundedString = { type: "string", minLength: 1, maxLength: 4096 };
const paths = { type: "array", maxItems: 256, items: boundedString };
export const PROCESS_POLICY_PROPERTIES = {
  maxConcurrent: { type: "integer", minimum: 1, maximum: Number.MAX_SAFE_INTEGER },
  tasks: {
    type: "object",
    maxProperties: 128,
    propertyNames: { pattern: "^[a-z][a-z0-9_-]{0,63}$" },
    additionalProperties: {
      type: "object",
      additionalProperties: false,
      required: ["command", "inputs", "outputs"],
      properties: {
        command: { type: "array", minItems: 1, maxItems: 256, items: boundedString },
        inputs: paths,
        outputs: paths,
        cwd: boundedString,
        env: {
          type: "object",
          maxProperties: 64,
          propertyNames: { pattern: "^[A-Za-z_][A-Za-z0-9_]*$" },
          additionalProperties: { type: "string", maxLength: 4096 },
        },
      },
    },
  },
};
const validate = new Ajv2020Ns.default({ strict: true, allErrors: true }).compile({
  type: "object",
  additionalProperties: false,
  properties: PROCESS_POLICY_PROPERTIES,
});
function invalid(reason: string): never {
  throw new CheaperError("INVALID_RUNTIME_POLICY", `invalid process policy: ${reason}`);
}

/** alias を realpath 解決した後も同じ禁止 executable 規則を利用する。 */
export function isShellProcessExecutable(executable: string): boolean {
  const name = executable.split(/[\\/]/).at(-1)?.toLowerCase() ?? "";
  return (
    /^(?:sh|bash|dash|zsh|fish|ksh|csh|tcsh|ash|cmd|powershell|pwsh|busybox)(?:\.exe)?$/.test(
      name,
    ) || /\.(?:cmd|bat|ps1|sh)$/.test(name)
  );
}
function relative(input: string): void {
  if (
    input.includes("\0") ||
    path.isAbsolute(input) ||
    path.win32.isAbsolute(input) ||
    /^[a-z]:/i.test(input) ||
    input.split(/[\\/]/).includes("..")
  )
    invalid("path must be workspace-relative without traversal");
}

/** typed runtime options も同じ config boundary で検証し、caller の可変値を保持しない。 */
export function resolveProcessPolicy(input?: Partial<ProcessPolicy>): ProcessPolicy {
  const raw = input ?? {};
  if (!validate(raw)) invalid("missing, unknown or invalid config field");
  const tasks: Record<string, ProcessTask> = {};
  for (const [name, task] of Object.entries(raw.tasks ?? {}).sort(([a], [b]) =>
    a.localeCompare(b),
  )) {
    if (isShellProcessExecutable(task.command[0] as string))
      invalid("shell or script shim executable is forbidden");
    if (task.command.some((arg) => /[\0\r\n|&;<>`]|\$\(/.test(arg)))
      invalid("shell syntax in argv is forbidden");
    relative(task.cwd ?? ".");
    for (const value of [...task.inputs, ...task.outputs]) relative(value);
    const env: Record<string, string> = {};
    const seen = new Set<string>();
    for (const [key, value] of Object.entries(task.env ?? {}).sort(([a], [b]) =>
      a.localeCompare(b),
    )) {
      const identity = process.platform === "win32" ? key.toUpperCase() : key;
      if (seen.has(identity) || value.includes("\0"))
        invalid("invalid or duplicate environment variable");
      seen.add(identity);
      env[key] = value;
    }
    tasks[name] = {
      command: [...task.command],
      inputs: [...task.inputs],
      outputs: [...task.outputs],
      cwd: task.cwd ?? ".",
      env,
    };
  }
  return { maxConcurrent: raw.maxConcurrent ?? 1, tasks };
}

export function resolveProcessTask(policy: ProcessPolicy, name: string): ProcessTask {
  if (!Object.hasOwn(policy.tasks, name)) {
    throw new CheaperError("POLICY_VIOLATION", "unknown or unauthorized process task", {
      details: { task: name },
    });
  }
  return policy.tasks[name] as ProcessTask;
}

/** workspace root 相対の宣言。glob の safety 拡大と ownership 検証は core resources が所有する。 */
export function processTaskResources(
  policy: ProcessPolicy,
  name: string,
): ResourceManifestDeclaration {
  const task = resolveProcessTask(policy, name);
  const scopes = (patterns: string[]): ScopeDeclaration[] =>
    patterns.map((value) => ({
      path: value,
      scope: /(?:^|[\\/])\*\*$/.test(value) ? "subtree" : "exact",
    }));
  return {
    reads: scopes(task.inputs),
    writes: scopes(task.outputs),
    produces: scopes(task.outputs),
  };
}
