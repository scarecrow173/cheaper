// cheaper Cursor adapter (DESIGN-034rA79Uog7JOa7evw6rOL).
// Cursor fires `beforeReadFile` (payload carries `file_path`) and
// `beforeShellExecution` (payload carries `command` and `cwd`). The
// adapter distinguishes the event by payload keys, normalizes into a
// canonical routing request, and converts the shared policy verdict
// into Cursor's permission response:
// `{"permission":"deny",...}` blocks, `{"permission":"allow"}` allows.
//
// Cursor treats exit 0 with a schema-invalid response as a block, so
// this script ALWAYS emits a schema-valid permission JSON — including
// on malformed stdin and infrastructure failures (fail-open via
// explicit allow, never silent).

const policy = require("./routing-policy.js");

function respond(permission, reason) {
  const out = { permission };
  if (reason) {
    const text = reason.split("\n").join(" ");
    out.user_message = text;
    out.agent_message = text;
  }
  process.stdout.write(`${JSON.stringify(out)}\n`);
  process.exit(0);
}

function main() {
  let input;
  try {
    input = JSON.parse(policy.readStdin());
  } catch {
    respond("allow");
  }
  let request = null;
  if (input && typeof input.file_path === "string" && input.file_path !== "") {
    request = {
      kind: "file-read",
      path: input.file_path,
      cwd: typeof input.cwd === "string" && input.cwd !== "" ? input.cwd : process.cwd(),
      sourceTool: "cursor:beforeReadFile",
    };
  } else if (input && typeof input.command === "string" && input.command !== "") {
    request = {
      kind: "shell",
      command: input.command,
      cwd: typeof input.cwd === "string" && input.cwd !== "" ? input.cwd : process.cwd(),
      sourceTool: "cursor:beforeShellExecution",
    };
  }
  if (!request) {
    respond("allow");
  }
  let verdict;
  try {
    verdict = policy.evaluate(request, { cli: process.argv[2] });
  } catch {
    respond("allow");
  }
  if (verdict.decision === "deny") {
    respond("deny", verdict.reason);
  }
  respond("allow");
}

main();
