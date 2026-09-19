// cheaper pre_run_command adapter (DESIGN-034rA79Uog7JOa7evw6rOL).
// Windsurf Cascade calls this before running a shell command. The
// adapter normalizes the native payload into a canonical shell
// request and hands it to the shared routing policy. The policy is
// currently an allow seam; issue #42 connects shell read candidate
// extraction there without touching this adapter.
//
// Fail-open contract: anything except an explicit policy deny exits 0
// so a broken setup never breaks normal commands.

const policy = require("./routing-policy.js");

function main() {
  let input;
  try {
    input = JSON.parse(policy.readStdin());
  } catch {
    process.exit(0);
  }
  if (!input || input.agent_action_name !== "pre_run_command") {
    process.exit(0);
  }
  const command = input.tool_info && input.tool_info.command_line;
  if (typeof command !== "string" || command === "") {
    process.exit(0);
  }
  const verdict = policy.evaluate(
    {
      kind: "shell",
      command,
      cwd: process.cwd(),
      sourceTool: "windsurf:pre_run_command",
    },
    { cli: process.argv[2] },
  );
  if (verdict.decision === "deny") {
    process.stderr.write(`${verdict.reason}\n`);
    process.exit(2);
  }
  process.exit(0);
}

main();
