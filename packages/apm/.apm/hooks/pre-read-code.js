// cheaper pre_read_code adapter (DESIGN-034rA79Uog7JOa7evw6rOL).
// Windsurf Cascade calls this before reading a code file. The adapter
// only normalizes the native payload into a canonical file-read
// request and converts the shared policy verdict into Windsurf's
// block contract (stderr guidance + exit 2). All large-read policy
// lives in routing-policy.js.
//
// Fail-open contract: anything except a measured large file exits 0 so
// a broken setup never breaks normal reads.

const policy = require("./routing-policy.js");

function main() {
  let input;
  try {
    input = JSON.parse(policy.readStdin());
  } catch {
    process.exit(0);
  }
  if (input?.agent_action_name !== "pre_read_code") {
    process.exit(0);
  }
  const filePath = input.tool_info?.file_path;
  if (typeof filePath !== "string" || filePath === "") {
    process.exit(0);
  }
  const verdict = policy.evaluate(
    {
      kind: "file-read",
      path: filePath,
      cwd: process.cwd(),
      sourceTool: "windsurf:pre_read_code",
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
