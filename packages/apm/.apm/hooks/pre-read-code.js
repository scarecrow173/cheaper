"use strict";

// cheaper pre_read_code hook (DESIGN-0013).
// Windsurf Cascade calls this before reading a code file. We measure the
// file's canonical token count via `cheaper count-tokens`; if it exceeds the
// threshold we exit 2 with guidance so the agent delegates to bulk-read
// instead of loading the file into context.
//
// Fail-open contract: anything except a measured large file exits 0 so a
// broken setup never breaks normal reads.

const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

const DEFAULT_THRESHOLD = 8192;

function readStdin() {
  try {
    return fs.readFileSync(0, "utf8");
  } catch {
    return "";
  }
}

function threshold() {
  const raw = process.env.CHEAPER_READ_TOKEN_THRESHOLD;
  const n = raw === undefined ? NaN : Number.parseInt(raw, 10);
  return Number.isInteger(n) && n > 0 ? n : DEFAULT_THRESHOLD;
}

// Returns [cmd, argsPrefix]: how to invoke the cheaper CLI.
// Resolution order: argv[2] (bundled CLI path from hooks.json) >
// CHEAPER_CLI env > `cheaper` on PATH. .js/.mjs paths run via node.
function cheaperCommand() {
  const explicit = process.argv[2] || process.env.CHEAPER_CLI;
  if (explicit && /\.m?js$/.test(explicit)) {
    return [process.execPath, [explicit]];
  }
  return [explicit || "cheaper", []];
}

function main() {
  let input;
  try {
    input = JSON.parse(readStdin());
  } catch {
    process.exit(0);
  }
  if (!input || input.agent_action_name !== "pre_read_code") {
    process.exit(0);
  }
  const filePath = input.tool_info && input.tool_info.file_path;
  if (typeof filePath !== "string" || filePath === "") {
    process.exit(0);
  }
  // Windsurf passes an absolute path; bulk-reader's input schema requires a
  // workspace-relative path. Normalize against process.cwd() (the workspace
  // root) and fail open on anything outside it.
  const resolved = path.resolve(filePath);
  const rel = path.relative(process.cwd(), resolved);
  // ".." alone or a "../" prefix means outside; a name merely starting with
  // ".." (e.g. "..foo.ts") is still inside the workspace.
  if (rel === "" || rel === ".." || rel.startsWith(`..${path.sep}`) || path.isAbsolute(rel)) {
    process.exit(0);
  }
  const relPosix = rel.split(path.sep).join("/");
  let st;
  try {
    st = fs.statSync(resolved);
    if (!st.isFile()) {
      process.exit(0);
    }
  } catch {
    process.exit(0);
  }
  // Fast path (DESIGN-0017): tokens <= bytes always holds, so a file whose
  // size fits the threshold can never exceed it — skip the CLI spawn.
  const limit = threshold();
  if (st.size <= limit) {
    process.exit(0);
  }

  const [cmd, prefix] = cheaperCommand();
  const run = spawnSync(
    cmd,
    [...prefix, "count-tokens", resolved, "--cwd", process.cwd()],
    { encoding: "utf8", timeout: 30_000 },
  );
  if (run.error) {
    process.exit(0);
  }
  let envelope;
  try {
    envelope = JSON.parse(run.stdout);
  } catch {
    envelope = null;
  }
  // A parseable envelope is authoritative even on non-zero exit: a
  // FILE_TOO_LARGE refusal is a measured verdict (fail-closed), unlike
  // infra failures which stay fail-open.
  if (envelope && envelope.error && envelope.error.code === "FILE_TOO_LARGE") {
    const det = envelope.error.details || {};
    const bytes = typeof det.bytes === "number" ? det.bytes : st.size;
    const capText = typeof det.maxBytes === "number" ? ` (exceeds the ${det.maxBytes}-byte measurement cap)` : "";
    process.stderr.write(
      `cheaper: '${relPosix}' is ${bytes} bytes${capText}.\n` +
        "Do not read it directly. Delegate this file through the cheaper-bulk-read skill.\n" +
        `workspace-relative path: ${relPosix}\n`,
    );
    process.exit(2);
  }
  if (run.status !== 0 || !envelope) {
    process.exit(0);
  }
  if (!envelope.ok || !envelope.result || typeof envelope.result.tokens !== "number") {
    process.exit(0);
  }

  const { tokens } = envelope.result;
  if (tokens <= limit) {
    process.exit(0);
  }
  // Non-executable guidance only: never emit a shell command with the file
  // path interpolated (quoting would break on adversarial names). The skill
  // owns JSON/stdin assembly; we hand over the workspace-relative path.
  process.stderr.write(
    `cheaper: '${relPosix}' has ${tokens} tokens (threshold ${limit}).\n` +
      "Do not read it directly. Delegate this file through the cheaper-bulk-read skill.\n" +
      `workspace-relative path: ${relPosix}\n`,
  );
  process.exit(2);
}

main();
