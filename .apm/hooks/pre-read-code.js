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
  try {
    if (!fs.statSync(filePath).isFile()) {
      process.exit(0);
    }
  } catch {
    process.exit(0);
  }

  const [cmd, prefix] = cheaperCommand();
  const run = spawnSync(
    cmd,
    [...prefix, "count-tokens", filePath, "--cwd", process.cwd()],
    { encoding: "utf8", timeout: 30_000 },
  );
  if (run.error || run.status !== 0) {
    process.exit(0);
  }
  let envelope;
  try {
    envelope = JSON.parse(run.stdout);
  } catch {
    process.exit(0);
  }
  if (!envelope.ok || !envelope.result || typeof envelope.result.tokens !== "number") {
    process.exit(0);
  }

  const limit = threshold();
  const { tokens } = envelope.result;
  if (tokens <= limit) {
    process.exit(0);
  }
  process.stderr.write(
    `cheaper: '${envelope.result.path || filePath}' has ${tokens} tokens (threshold ${limit}). ` +
      "Do not read it directly. Delegate to the cheaper bulk-reader instead " +
      "(see the cheaper-bulk-read skill):\n" +
      `echo '{"task":"<what you need from the file>","paths":["${filePath}"]}' | cheaper bulk-read\n`,
  );
  process.exit(2);
}

main();
