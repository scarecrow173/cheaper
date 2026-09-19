// cheaper shared routing policy (DESIGN-034rA79Uog7JOa7evw6rOL).
// Target adapters normalize native hook payloads into a canonical
// routing request and call evaluate(); this module owns every
// target-independent decision: workspace containment, the byte-size
// fast path, `cheaper count-tokens` measurement, the token threshold,
// and the FILE_TOO_LARGE fail-closed verdict.
//
// Canonical requests:
//   { kind: "file-read", path: string, cwd: string, sourceTool?: string }
//   { kind: "shell", command: string, cwd: string, sourceTool?: string }
//
// Verdicts:
//   { decision: "allow" }
//   { decision: "deny", reason: string }   // multi-line guidance text
//
// `shell` requests currently always allow; issue #42 connects read
// candidate extraction here without touching any adapter.

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

function threshold(env) {
  const raw = (env || process.env).CHEAPER_READ_TOKEN_THRESHOLD;
  const n = raw === undefined ? NaN : Number.parseInt(raw, 10);
  return Number.isInteger(n) && n > 0 ? n : DEFAULT_THRESHOLD;
}

// Returns [cmd, argsPrefix]: how to invoke the cheaper CLI.
// Resolution order: explicit cli (adapter argv[2], bundled CLI path
// from hooks.json) > CHEAPER_CLI env > `cheaper` on PATH.
// .js/.mjs paths run via node.
function cheaperCommand(cli, env) {
  const explicit = cli || (env || process.env).CHEAPER_CLI;
  if (explicit && /\.m?js$/.test(explicit)) {
    return [process.execPath, [explicit]];
  }
  return [explicit || "cheaper", []];
}

// The measured verdict is the only fail-closed outcome; every
// infrastructure failure resolves to allow so a broken setup never
// breaks normal tool calls.
function evaluateFileRead(request, opts) {
  const cwd = request.cwd || process.cwd();
  const resolved = path.resolve(cwd, request.path);
  const rel = path.relative(cwd, resolved);
  // ".." alone or a "../" prefix means outside; a name merely starting
  // with ".." (e.g. "..foo.ts") is still inside the workspace.
  if (rel === "" || rel === ".." || rel.startsWith(`..${path.sep}`) || path.isAbsolute(rel)) {
    return { decision: "allow" };
  }
  const relPosix = rel.split(path.sep).join("/");
  let st;
  try {
    st = fs.statSync(resolved);
    if (!st.isFile()) {
      return { decision: "allow" };
    }
  } catch {
    return { decision: "allow" };
  }
  // Fast path (DESIGN-0017): tokens <= bytes always holds, so a file
  // whose size fits the threshold can never exceed it — skip the CLI
  // spawn.
  const limit = threshold(opts && opts.env);
  if (st.size <= limit) {
    return { decision: "allow" };
  }

  const [cmd, prefix] = cheaperCommand(opts && opts.cli, opts && opts.env);
  const run = spawnSync(cmd, [...prefix, "count-tokens", resolved, "--cwd", cwd], {
    encoding: "utf8",
    timeout: 30_000,
  });
  if (run.error) {
    return { decision: "allow" };
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
    const capText =
      typeof det.maxBytes === "number" ? ` (exceeds the ${det.maxBytes}-byte measurement cap)` : "";
    return {
      decision: "deny",
      reason:
        `cheaper: '${relPosix}' is ${bytes} bytes${capText}.\n` +
        "Do not read it directly. Delegate this file through the cheaper-bulk-read skill.\n" +
        `workspace-relative path: ${relPosix}`,
    };
  }
  if (run.status !== 0 || !envelope) {
    return { decision: "allow" };
  }
  if (!envelope.ok || !envelope.result || typeof envelope.result.tokens !== "number") {
    return { decision: "allow" };
  }

  const { tokens } = envelope.result;
  if (tokens <= limit) {
    return { decision: "allow" };
  }
  // Non-executable guidance only: never emit a shell command with the
  // file path interpolated (quoting would break on adversarial names).
  // The skill owns JSON/stdin assembly; we hand over the
  // workspace-relative path.
  return {
    decision: "deny",
    reason:
      `cheaper: '${relPosix}' has ${tokens} tokens (threshold ${limit}).\n` +
      "Do not read it directly. Delegate this file through the cheaper-bulk-read skill.\n" +
      `workspace-relative path: ${relPosix}`,
  };
}

function evaluateShell(_request, _opts) {
  // Seam for issue #42: shell read candidate extraction plugs in here.
  return { decision: "allow" };
}

function evaluate(request, opts) {
  if (!request || typeof request !== "object") {
    return { decision: "allow" };
  }
  if (request.kind === "file-read" && typeof request.path === "string" && request.path !== "") {
    return evaluateFileRead(request, opts);
  }
  if (request.kind === "shell" && typeof request.command === "string" && request.command !== "") {
    return evaluateShell(request, opts);
  }
  return { decision: "allow" };
}

module.exports = {
  DEFAULT_THRESHOLD,
  readStdin,
  threshold,
  cheaperCommand,
  evaluate,
};
