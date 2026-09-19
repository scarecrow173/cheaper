// cheaper preToolUse adapter for GitHub Copilot (DESIGN-0025,
// ADR-0023, DESIGN-034rA79Uog7JOa7evw6rOL). Copilot fires `preToolUse`
// before every tool call; we intercept `toolName == "view"` (the file
// read tool) and the shell tools (`bash` / `powershell` / `shell`),
// normalize them into canonical routing requests, and convert the
// shared policy verdict into Copilot's permission decision JSON.
//
// Fail-closed platform, fail-open hook: Copilot denies the tool call
// on ANY nonzero exit, so this script NEVER exits nonzero. Intended
// blocks are expressed as `{"permissionDecision":"deny",...}` on
// stdout with exit 0; every error/unexpected path exits 0 silently
// (the same fail-open semantics as the Windsurf pre_read_code hook).
// All large-read policy lives in routing-policy.js.

const policy = require("./routing-policy.js");

const READ_TOOLS = new Set(["view"]);
const SHELL_TOOLS = new Set(["bash", "powershell", "shell"]);

// toolArgs arrives as either an object or a JSON string depending on
// the Copilot host (official docs differ). Normalize both; individual
// keys have also drifted, so accept the documented variants.
function normalizeToolArgs(toolArgs) {
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
  return args;
}

function extractFilePath(args) {
  for (const key of ["path", "file_path", "filePath"]) {
    if (typeof args[key] === "string" && args[key] !== "") {
      return args[key];
    }
  }
  return null;
}

function extractCommand(args) {
  for (const key of ["command", "command_line", "commandLine", "cmd"]) {
    if (typeof args[key] === "string" && args[key] !== "") {
      return args[key];
    }
  }
  return null;
}

function deny(reason) {
  process.stdout.write(
    `${JSON.stringify({
      permissionDecision: "deny",
      permissionDecisionReason: reason.split("\n").join(" "),
    })}\n`,
  );
  process.exit(0);
}

function main() {
  let input;
  try {
    input = JSON.parse(policy.readStdin());
  } catch {
    process.exit(0);
  }
  if (!input || typeof input !== "object") {
    process.exit(0);
  }
  if (typeof input.toolName !== "string") {
    // Foreign payload shape (e.g. this hook entry cross-firing under a
    // target whose preToolUse uses snake_case fields). Emit a dual
    // schema allow so Cursor-style permission hooks do not treat a
    // silent exit 0 as a block; Copilot reads `permissionDecision`
    // and sees an explicit allow.
    if (input.tool_name || input.command || input.file_path) {
      process.stdout.write(
        `${JSON.stringify({ permission: "allow", permissionDecision: "allow" })}\n`,
      );
    }
    process.exit(0);
  }
  const args = normalizeToolArgs(input.toolArgs);
  let request = null;
  if (READ_TOOLS.has(input.toolName) && args) {
    const filePath = extractFilePath(args);
    if (filePath !== null) {
      request = {
        kind: "file-read",
        path: filePath,
        cwd: process.cwd(),
        sourceTool: `copilot:${input.toolName}`,
      };
    }
  } else if (SHELL_TOOLS.has(input.toolName) && args) {
    const command = extractCommand(args);
    if (command !== null) {
      request = {
        kind: "shell",
        command,
        cwd: process.cwd(),
        sourceTool: `copilot:${input.toolName}`,
      };
    }
  }
  if (!request) {
    process.exit(0);
  }
  const verdict = policy.evaluate(request, { cli: process.argv[2] });
  if (verdict.decision === "deny") {
    deny(verdict.reason);
  }
  process.exit(0);
}

main();
