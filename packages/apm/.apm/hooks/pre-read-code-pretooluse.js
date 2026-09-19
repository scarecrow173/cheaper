// cheaper PreToolUse adapter for Claude Code and Codex
// (DESIGN-034rA79Uog7JOa7evw6rOL). Both agents deliver the same
// `{tool_name, tool_input}` payload shape: `Read` carries
// `tool_input.file_path`, `Bash`/`PowerShell` carry
// `tool_input.command`. The adapter normalizes either into a
// canonical routing request and converts the shared policy verdict
// into the PreToolUse deny contract:
// `hookSpecificOutput.permissionDecision: "deny"` on stdout with
// exit 0.
//
// Fail-open contract: malformed payloads, unrelated tools, missing
// fields, and infrastructure failures all exit 0 silently so a broken
// setup never breaks normal tool calls. Codex does not emit PreToolUse
// for its file-read tools today (upstream gap), so in practice only
// the shell mapping fires there.

const policy = require("./routing-policy.js");

const READ_TOOLS = new Set(["Read"]);
const SHELL_TOOLS = new Set(["Bash", "PowerShell"]);

function deny(reason) {
  process.stdout.write(
    `${JSON.stringify({
      hookSpecificOutput: {
        hookEventName: "PreToolUse",
        permissionDecision: "deny",
        permissionDecisionReason: reason.split("\n").join(" "),
      },
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
  if (!input || typeof input.tool_name !== "string") {
    process.exit(0);
  }
  const toolInput =
    input.tool_input && typeof input.tool_input === "object" ? input.tool_input : {};
  let request = null;
  if (READ_TOOLS.has(input.tool_name) && typeof toolInput.file_path === "string") {
    request = {
      kind: "file-read",
      path: toolInput.file_path,
      cwd: process.cwd(),
      sourceTool: `pretooluse:${input.tool_name}`,
    };
  } else if (SHELL_TOOLS.has(input.tool_name) && typeof toolInput.command === "string") {
    request = {
      kind: "shell",
      command: toolInput.command,
      cwd: process.cwd(),
      sourceTool: `pretooluse:${input.tool_name}`,
    };
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
