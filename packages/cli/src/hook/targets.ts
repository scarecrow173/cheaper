// Target adapters for `cheaper hook` (DESIGN-034z1EhuztlLSb2x2XXxgD).
// Each adapter normalizes the agent's native stdin payload into a
// canonical routing request and renders the shared policy verdict in
// the target's response contract. Ported 1:1 from the JS adapters:
//   windsurf   (pre-read-code.js / pre-run-command.js)
//   copilot    (pre-read-code-copilot.js)
//   pretooluse (pre-read-code-pretooluse.js, claude + codex shared)
//   cursor     (pre-read-code-cursor.js)

import { evaluate, type RoutingRequest, type Verdict } from "./routing-policy.js";

export type HookTarget = "windsurf" | "copilot" | "pretooluse" | "cursor";

export const HOOK_TARGETS: readonly HookTarget[] = [
  "windsurf",
  "copilot",
  "pretooluse",
  "cursor",
];

export interface HookResponse {
  /** stdout に書く text（無い場合は出力しない）。 */
  stdout?: string;
  /** stderr に書く text。 */
  stderr?: string;
  exit: number;
}

export interface HookDeps {
  env?: NodeJS.ProcessEnv;
  builtinDir?: string | undefined;
  /** hook 呼び出し側の cwd（payload に cwd が無い場合の fallback）。 */
  cwd: string;
}

const SILENT_ALLOW: HookResponse = { exit: 0 };

function windsurfExpectedEvent(event: string): string | null {
  if (event === "pre-read-code") return "pre_read_code";
  if (event === "pre-run-command") return "pre_run_command";
  return null;
}

// --- windsurf: deny = stderr guidance + exit 2, allow = silent exit 0 ---

function windsurfNormalize(
  event: string,
  input: unknown,
  cwd: string,
): RoutingRequest | null {
  const expected = windsurfExpectedEvent(event);
  if (expected === null || !input || typeof input !== "object") return null;
  const payload = input as {
    agent_action_name?: unknown;
    tool_info?: { file_path?: unknown; command_line?: unknown } | unknown;
  };
  if (payload.agent_action_name !== expected) return null;
  const toolInfo =
    payload.tool_info && typeof payload.tool_info === "object"
      ? (payload.tool_info as { file_path?: unknown; command_line?: unknown })
      : {};
  if (expected === "pre_read_code") {
    const filePath = toolInfo.file_path;
    if (typeof filePath !== "string" || filePath === "") return null;
    return { kind: "file-read", path: filePath, cwd, sourceTool: "windsurf:pre_read_code" };
  }
  const command = toolInfo.command_line;
  if (typeof command !== "string" || command === "") return null;
  return { kind: "shell", command, cwd, sourceTool: "windsurf:pre_run_command" };
}

function windsurfRender(verdict: Verdict): HookResponse {
  if (verdict.decision === "deny") {
    return { stderr: `${verdict.reason}\n`, exit: 2 };
  }
  return SILENT_ALLOW;
}

// --- copilot: deny = permissionDecision JSON + exit 0, never nonzero ---

const COPILOT_READ_TOOLS = new Set(["view"]);
const COPILOT_SHELL_TOOLS = new Set(["bash", "powershell", "shell"]);

function normalizeToolArgs(toolArgs: unknown): Record<string, unknown> | null {
  let args = toolArgs;
  if (typeof args === "string") {
    try {
      args = JSON.parse(args);
    } catch {
      return null;
    }
  }
  if (!args || typeof args !== "object") {
    return null;
  }
  return args as Record<string, unknown>;
}

function pickString(args: Record<string, unknown>, keys: readonly string[]): string | null {
  for (const key of keys) {
    const v = args[key];
    if (typeof v === "string" && v !== "") {
      return v;
    }
  }
  return null;
}

function copilotHandle(input: unknown, cwd: string): RoutingRequest | null | HookResponse {
  if (!input || typeof input !== "object") {
    return null;
  }
  const payload = input as {
    toolName?: unknown;
    toolArgs?: unknown;
    tool_name?: unknown;
    command?: unknown;
    file_path?: unknown;
  };
  if (typeof payload.toolName !== "string") {
    // Foreign payload shape (e.g. this hook entry cross-firing under a
    // target whose preToolUse uses snake_case fields). Emit a dual
    // schema allow so Cursor-style permission hooks do not treat a
    // silent exit 0 as a block; Copilot reads `permissionDecision`
    // and sees an explicit allow.
    if (payload.tool_name || payload.command || payload.file_path) {
      return {
        stdout: `${JSON.stringify({ permission: "allow", permissionDecision: "allow" })}\n`,
        exit: 0,
      };
    }
    return null;
  }
  const args = normalizeToolArgs(payload.toolArgs);
  if (COPILOT_READ_TOOLS.has(payload.toolName) && args) {
    const filePath = pickString(args, ["path", "file_path", "filePath"]);
    if (filePath !== null) {
      return {
        kind: "file-read",
        path: filePath,
        cwd,
        sourceTool: `copilot:${payload.toolName}`,
      };
    }
    return null;
  }
  if (COPILOT_SHELL_TOOLS.has(payload.toolName) && args) {
    const command = pickString(args, ["command", "command_line", "commandLine", "cmd"]);
    if (command !== null) {
      return { kind: "shell", command, cwd, sourceTool: `copilot:${payload.toolName}` };
    }
  }
  return null;
}

function copilotRender(verdict: Verdict): HookResponse {
  if (verdict.decision === "deny") {
    return {
      stdout: `${JSON.stringify({
        permissionDecision: "deny",
        permissionDecisionReason: verdict.reason.split("\n").join(" "),
      })}\n`,
      exit: 0,
    };
  }
  return SILENT_ALLOW;
}

// --- pretooluse (claude / codex): deny = hookSpecificOutput JSON + exit 0 ---

const PRETOOLUSE_READ_TOOLS = new Set(["Read"]);
const PRETOOLUSE_SHELL_TOOLS = new Set(["Bash", "PowerShell"]);

function pretooluseNormalize(input: unknown, cwd: string): RoutingRequest | null {
  if (!input || typeof input !== "object") return null;
  const payload = input as { tool_name?: unknown; tool_input?: unknown };
  if (typeof payload.tool_name !== "string") return null;
  const toolInput =
    payload.tool_input && typeof payload.tool_input === "object"
      ? (payload.tool_input as { file_path?: unknown; command?: unknown })
      : {};
  if (PRETOOLUSE_READ_TOOLS.has(payload.tool_name) && typeof toolInput.file_path === "string") {
    return {
      kind: "file-read",
      path: toolInput.file_path,
      cwd,
      sourceTool: `pretooluse:${payload.tool_name}`,
    };
  }
  if (PRETOOLUSE_SHELL_TOOLS.has(payload.tool_name) && typeof toolInput.command === "string") {
    return {
      kind: "shell",
      command: toolInput.command,
      cwd,
      sourceTool: `pretooluse:${payload.tool_name}`,
    };
  }
  return null;
}

function pretooluseRender(verdict: Verdict): HookResponse {
  if (verdict.decision === "deny") {
    return {
      stdout: `${JSON.stringify({
        hookSpecificOutput: {
          hookEventName: "PreToolUse",
          permissionDecision: "deny",
          permissionDecisionReason: verdict.reason.split("\n").join(" "),
        },
      })}\n`,
      exit: 0,
    };
  }
  return SILENT_ALLOW;
}

// --- cursor: always emit schema-valid permission JSON ---

function cursorNormalize(input: unknown, cwd: string): RoutingRequest | null {
  if (!input || typeof input !== "object") return null;
  const payload = input as { file_path?: unknown; command?: unknown; cwd?: unknown };
  const payloadCwd =
    typeof payload.cwd === "string" && payload.cwd !== "" ? payload.cwd : cwd;
  if (typeof payload.file_path === "string" && payload.file_path !== "") {
    return {
      kind: "file-read",
      path: payload.file_path,
      cwd: payloadCwd,
      sourceTool: "cursor:beforeReadFile",
    };
  }
  if (typeof payload.command === "string" && payload.command !== "") {
    return {
      kind: "shell",
      command: payload.command,
      cwd: payloadCwd,
      sourceTool: "cursor:beforeShellExecution",
    };
  }
  return null;
}

function cursorRespond(permission: string, reason?: string): HookResponse {
  const out: { permission: string; user_message?: string; agent_message?: string } = {
    permission,
  };
  if (reason) {
    const text = reason.split("\n").join(" ");
    out.user_message = text;
    out.agent_message = text;
  }
  return { stdout: `${JSON.stringify(out)}\n`, exit: 0 };
}

function cursorRender(verdict: Verdict): HookResponse {
  if (verdict.decision === "deny") {
    return cursorRespond("deny", verdict.reason);
  }
  return cursorRespond("allow");
}

/**
 * target 別の normalize → evaluate → render。
 * normalize/render のどの段でも失敗した場合は呼び出し側の catch-all
 * が fallback に倒す。
 */
export async function handleHook(
  target: HookTarget,
  event: string,
  input: unknown,
  deps: HookDeps,
): Promise<HookResponse> {
  switch (target) {
    case "windsurf": {
      const request = windsurfNormalize(event, input, deps.cwd);
      if (!request) return SILENT_ALLOW;
      return windsurfRender(await evaluate(request, deps));
    }
    case "copilot": {
      const normalized = copilotHandle(input, deps.cwd);
      if (normalized === null) return SILENT_ALLOW;
      if ("exit" in normalized) return normalized;
      return copilotRender(await evaluate(normalized, deps));
    }
    case "pretooluse": {
      const request = pretooluseNormalize(input, deps.cwd);
      if (!request) return SILENT_ALLOW;
      return pretooluseRender(await evaluate(request, deps));
    }
    case "cursor": {
      const request = cursorNormalize(input, deps.cwd);
      if (!request) return cursorRespond("allow");
      let verdict: Verdict;
      try {
        verdict = await evaluate(request, deps);
      } catch {
        return cursorRespond("allow");
      }
      return cursorRender(verdict);
    }
  }
}

/** 最深部 catch-all が使う target 適合の allow 応答。 */
export function fallbackAllow(target: HookTarget | undefined): HookResponse {
  if (target === "cursor") {
    return cursorRespond("allow");
  }
  return SILENT_ALLOW;
}
