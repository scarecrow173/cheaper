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
// `shell` requests extract read candidates (cat/head/tail/less/more
// file operands) via a conservative tokenizer and evaluate each
// through the same file-read policy — no adapter changes needed.

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
  const limit = threshold(opts?.env);
  if (st.size <= limit) {
    return { decision: "allow" };
  }

  const [cmd, prefix] = cheaperCommand(opts?.cli, opts?.env);
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
  if (envelope?.error && envelope.error.code === "FILE_TOO_LARGE") {
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

// Conservative shell tokenizer for issue #42 (ADR-034rBhRB7QyWkAxRiTDXXq).
// It never executes or expands anything: quoting only delimits tokens, and
// a token containing shell metacharacters is flagged `unsafe` so its whole
// segment is skipped (fail-open — a missed detection, never a false block).
// Returns null on unrecoverable input (e.g. an unterminated quote).

const SEGMENT_UNSAFE = /[<>]/;
const TOKEN_UNSAFE = /[$`*?[\]~(){}!]/;
const ENV_PREFIX = /^[A-Za-z_][A-Za-z0-9_]*=/;
// A backslash escapes the next character only when that character is a
// shell metachar or whitespace; otherwise it stays literal so Windows-style
// paths such as `src\dir\file.txt` survive tokenization.
const ESCAPABLE = /[\s'"\\|&;<>$`*?[\](){}!#~]/;

function tokenizeShell(command) {
  const segments = [];
  let tokens = [];
  let unsafe = false;
  let text = "";
  let tokenUnsafe = false;
  let quote = null; // null | "'" | '"'
  let hasToken = false;

  const endToken = () => {
    if (hasToken) {
      tokens.push({ text, unsafe: tokenUnsafe });
      text = "";
      tokenUnsafe = false;
      hasToken = false;
    }
  };
  const endSegment = () => {
    endToken();
    segments.push({ tokens, unsafe });
    tokens = [];
    unsafe = false;
  };

  for (let i = 0; i < command.length; i++) {
    const ch = command[i];
    if (quote === "'") {
      if (ch === "'") {
        quote = null;
      } else {
        text += ch;
        hasToken = true;
      }
      continue;
    }
    if (quote === '"') {
      if (ch === '"') {
        quote = null;
      } else if (ch === "\\" && i + 1 < command.length && ESCAPABLE.test(command[i + 1])) {
        text += command[++i];
        hasToken = true;
      } else {
        text += ch;
        hasToken = true;
        if (ch === "$" || ch === "`") tokenUnsafe = true;
      }
      continue;
    }
    // Unquoted context.
    if (ch === "#" && !hasToken) {
      // Comment: skip to the next line.
      while (i < command.length && command[i] !== "\n") i++;
      continue;
    }
    if (ch === " " || ch === "\t" || ch === "\r") {
      endToken();
      continue;
    }
    if (ch === "\n" || ch === ";") {
      endSegment();
      continue;
    }
    if (ch === "|" || ch === "&") {
      if (command[i + 1] === ch) i++; // || or &&
      endSegment();
      continue;
    }
    if (SEGMENT_UNSAFE.test(ch)) {
      // Redirection makes the whole segment uninterpretable for us.
      unsafe = true;
      continue;
    }
    if (ch === "'" || ch === '"') {
      quote = ch;
      hasToken = true;
      continue;
    }
    if (ch === "\\") {
      if (i + 1 < command.length && ESCAPABLE.test(command[i + 1])) {
        text += command[++i];
      } else {
        text += ch;
      }
      hasToken = true;
      continue;
    }
    if (TOKEN_UNSAFE.test(ch)) tokenUnsafe = true;
    text += ch;
    hasToken = true;
  }
  if (quote !== null) return null; // unterminated quote
  endSegment();
  return segments;
}

// Read commands and the options that consume a separate value token.
// `plusOptions` marks commands whose `+`-prefixed tokens are options too
// (less `+F` / `+123`, more `+/<pattern>` / `+<line>`).
const READ_COMMANDS = {
  cat: { valueOptions: new Set(), plusOptions: false },
  head: { valueOptions: new Set(["-n", "-c", "--lines", "--bytes"]), plusOptions: false },
  tail: {
    valueOptions: new Set([
      "-n",
      "-c",
      "-s",
      "--lines",
      "--bytes",
      "--sleep-interval",
      "--pid",
      "--max-unchanged-stats",
    ]),
    plusOptions: false,
  },
  less: {
    valueOptions: new Set([
      "-o",
      "-O",
      "-p",
      "-k",
      "-P",
      "-t",
      "-T",
      "-D",
      "--log-file",
      "--pattern",
      "--tags",
      "--tag-file",
      "--prompt",
    ]),
    plusOptions: true,
  },
  more: { valueOptions: new Set(["-n", "-p", "-t"]), plusOptions: true },
};

function commandName(token) {
  const base = token.replace(/^.*[\\/]/, "");
  return base.replace(/\.(exe|bat|cmd)$/i, "");
}

// Commands that change the effective cwd for later segments (POSIX
// builtins plus cmd/PowerShell aliases). We do not track where they go —
// a read candidate after one resolves against a different cwd than the
// hook's, so it is ambiguous and must fail open rather than risk a
// false block (or a false pass) from the wrong base directory.
const CWD_CHANGING_COMMANDS = new Set(["cd", "chdir", "pushd", "popd", "set-location", "sl"]);

// Extracts file operands of read commands from a shell command string.
// Returns [] when the command is unparseable or nothing safely readable
// is referenced; callers treat [] as fail-open.
function extractReadCandidates(command) {
  const segments = tokenizeShell(command);
  if (!segments) return [];
  const candidates = [];
  let cwdAmbiguous = false;
  for (const segment of segments) {
    const { tokens } = segment;
    let i = 0;
    while (i < tokens.length && ENV_PREFIX.test(tokens[i].text)) i++;
    const head = tokens[i];
    if (!head || head.unsafe) continue;
    if (CWD_CHANGING_COMMANDS.has(commandName(head.text).toLowerCase())) {
      cwdAmbiguous = true;
      continue;
    }
    if (cwdAmbiguous || segment.unsafe) continue;
    const spec = READ_COMMANDS[commandName(head.text)];
    if (!spec) continue;
    const operands = [];
    let optionsEnded = false;
    let skipValue = false;
    let abandoned = false;
    for (let j = i + 1; j < tokens.length; j++) {
      const tok = tokens[j];
      if (skipValue) {
        skipValue = false;
        continue;
      }
      if (tok.unsafe) {
        abandoned = true;
        break;
      }
      const t = tok.text;
      if (!optionsEnded) {
        if (t === "--") {
          optionsEnded = true;
          continue;
        }
        if (t.startsWith("-") || (spec.plusOptions && t.startsWith("+"))) {
          if (spec.valueOptions.has(t)) skipValue = true;
          continue;
        }
      }
      operands.push(t);
    }
    if (!abandoned) candidates.push(...operands);
  }
  return [...new Set(candidates)];
}

function evaluateShell(request, opts) {
  const candidates = extractReadCandidates(request.command);
  for (const candidate of candidates) {
    const verdict = evaluateFileRead(
      {
        kind: "file-read",
        path: candidate,
        cwd: request.cwd,
        sourceTool: request.sourceTool,
      },
      opts,
    );
    if (verdict.decision === "deny") {
      return {
        decision: "deny",
        reason:
          `cheaper: blocked shell command '${request.command.replace(/\s+/g, " ").trim()}'.\n` +
          verdict.reason,
      };
    }
  }
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
