// cheaper shared routing policy (DESIGN-034rA79Uog7JOa7evw6rOL),
// ported from packages/apm/.apm/hooks/routing-policy.js.
// Target adapters normalize native hook payloads into a canonical
// routing request and call evaluate(); this module owns every
// target-independent decision: workspace containment, the byte-size
// fast path, in-process token measurement, the token threshold, and
// the FILE_TOO_LARGE fail-closed verdict.
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

import fs from "node:fs";
import path from "node:path";

export const DEFAULT_THRESHOLD = 8192;

// Legacy subprocess timeout: the JS policy spawned `cheaper
// count-tokens` with a 30s limit. The in-process equivalent wraps the
// measured path so hung I/O (FIFO, dead network FS) still resolves.
const MEASURE_TIMEOUT_MS = 30_000;

export interface FileReadRequest {
  kind: "file-read";
  path: string;
  cwd?: string | undefined;
  sourceTool?: string | undefined;
}

export interface ShellRequest {
  kind: "shell";
  command: string;
  cwd?: string | undefined;
  sourceTool?: string | undefined;
}

export type RoutingRequest = FileReadRequest | ShellRequest;

export type Verdict = { decision: "allow" } | { decision: "deny"; reason: string };

export interface EvaluateOptions {
  env?: NodeJS.ProcessEnv;
  /** built-in resources root（SEA では VFS root）。 */
  builtinDir?: string | undefined;
}

export function threshold(env?: NodeJS.ProcessEnv): number {
  const raw = (env || process.env).CHEAPER_READ_TOKEN_THRESHOLD;
  const n = raw === undefined ? NaN : Number.parseInt(raw, 10);
  return Number.isInteger(n) && n > 0 ? n : DEFAULT_THRESHOLD;
}

async function withTimeout<T>(promise: Promise<T>): Promise<T> {
  const signal = AbortSignal.timeout(MEASURE_TIMEOUT_MS);
  let onAbort: (() => void) | undefined;
  const timeout = new Promise<never>((_, reject) => {
    onAbort = () => reject(new Error("token measurement timed out"));
    signal.addEventListener("abort", onAbort, { once: true });
  });
  try {
    return await Promise.race([promise, timeout]);
  } finally {
    if (onAbort) signal.removeEventListener("abort", onAbort);
  }
}

/**
 * token 計測を実行する。byte fast path 通過後にのみ呼ばれるため、
 * tokenizer/policy 層の import コストはここに閉じ込める。
 * error の種別は raw のまま呼び出し側へ伝播させる。
 */
async function measureTokens(
  resolvedPath: string,
  cwd: string,
  opts: EvaluateOptions | undefined,
): Promise<{ tokens: number } | { fileTooLarge: { bytes: number; maxBytes: number } } | null> {
  let FileTooLargeError: typeof import("@cheaper/core").FileTooLargeError;
  let countWorkspaceFileTokens: typeof import("../count.js").countWorkspaceFileTokens;
  let defaultBuiltinDir: string;
  try {
    ({ FileTooLargeError } = await import("@cheaper/core"));
    ({ countWorkspaceFileTokens, defaultBuiltinDir } = await import("../count.js"));
  } catch {
    return null;
  }
  try {
    const result = await withTimeout(
      countWorkspaceFileTokens({
        cwd,
        path: resolvedPath,
        builtinDir: opts?.builtinDir ?? defaultBuiltinDir,
        env: opts?.env ?? process.env,
      }),
    );
    return { tokens: result.count.tokens };
  } catch (e) {
    if (e instanceof FileTooLargeError) {
      return { fileTooLarge: { bytes: e.bytes, maxBytes: e.maxBytes } };
    }
    return null;
  }
}

// The measured verdict is the only fail-closed outcome; every
// infrastructure failure resolves to allow so a broken setup never
// breaks normal tool calls.
async function evaluateFileRead(
  request: FileReadRequest,
  opts: EvaluateOptions | undefined,
): Promise<Verdict> {
  const cwd = request.cwd || process.cwd();
  const resolved = path.resolve(cwd, request.path);
  const rel = path.relative(cwd, resolved);
  // ".." alone or a "../" prefix means outside; a name merely starting
  // with ".." (e.g. "..foo.ts") is still inside the workspace.
  if (rel === "" || rel === ".." || rel.startsWith(`..${path.sep}`) || path.isAbsolute(rel)) {
    return { decision: "allow" };
  }
  const relPosix = rel.split(path.sep).join("/");
  let st: fs.Stats;
  try {
    st = fs.statSync(resolved);
    if (!st.isFile()) {
      return { decision: "allow" };
    }
  } catch {
    return { decision: "allow" };
  }
  // Fast path (DESIGN-0017): tokens <= bytes always holds, so a file
  // whose size fits the threshold can never exceed it — skip the
  // measurement entirely (no tokenizer/policy import).
  const limit = threshold(opts?.env);
  if (st.size <= limit) {
    return { decision: "allow" };
  }

  const measured = await measureTokens(resolved, cwd, opts);
  if (measured === null) {
    return { decision: "allow" };
  }
  if ("fileTooLarge" in measured) {
    const { bytes, maxBytes } = measured.fileTooLarge;
    return {
      decision: "deny",
      reason:
        `cheaper: '${relPosix}' is ${bytes} bytes (exceeds the ${maxBytes}-byte measurement cap).\n` +
        "Do not read it directly. Delegate this file through the cheaper-bulk-read skill.\n" +
        `workspace-relative path: ${relPosix}`,
    };
  }
  const { tokens } = measured;
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

interface ShellToken {
  text: string;
  unsafe: boolean;
}

interface ShellSegment {
  tokens: ShellToken[];
  unsafe: boolean;
}

export function tokenizeShell(command: string): ShellSegment[] | null {
  const segments: ShellSegment[] = [];
  let tokens: ShellToken[] = [];
  let unsafe = false;
  let text = "";
  let tokenUnsafe = false;
  let quote: "'" | '"' | null = null;
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
    const ch = command.charAt(i);
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
      } else if (ch === "\\" && i + 1 < command.length && ESCAPABLE.test(command.charAt(i + 1))) {
        text += command.charAt(++i);
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
      while (i < command.length && command.charAt(i) !== "\n") i++;
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
      if (command.charAt(i + 1) === ch) i++; // || or &&
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
      if (i + 1 < command.length && ESCAPABLE.test(command.charAt(i + 1))) {
        text += command.charAt(++i);
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
const READ_COMMANDS: Record<string, { valueOptions: Set<string>; plusOptions: boolean }> = {
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

function commandName(token: string): string {
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
export function extractReadCandidates(command: string): string[] {
  const segments = tokenizeShell(command);
  if (!segments) return [];
  const candidates: string[] = [];
  let cwdAmbiguous = false;
  for (const segment of segments) {
    const { tokens } = segment;
    let i = 0;
    while (i < tokens.length) {
      const t = tokens[i];
      if (t === undefined || !ENV_PREFIX.test(t.text)) break;
      i++;
    }
    const head = tokens[i];
    if (!head || head.unsafe) continue;
    if (CWD_CHANGING_COMMANDS.has(commandName(head.text).toLowerCase())) {
      cwdAmbiguous = true;
      continue;
    }
    if (cwdAmbiguous || segment.unsafe) continue;
    const spec = READ_COMMANDS[commandName(head.text)];
    if (!spec) continue;
    const operands: string[] = [];
    let optionsEnded = false;
    let skipValue = false;
    let abandoned = false;
    for (let j = i + 1; j < tokens.length; j++) {
      const tok = tokens[j];
      if (tok === undefined) break;
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

async function evaluateShell(
  request: ShellRequest,
  opts: EvaluateOptions | undefined,
): Promise<Verdict> {
  const candidates = extractReadCandidates(request.command);
  for (const candidate of candidates) {
    const verdict = await evaluateFileRead(
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

export async function evaluate(request: unknown, opts?: EvaluateOptions): Promise<Verdict> {
  if (!request || typeof request !== "object") {
    return { decision: "allow" };
  }
  const req = request as { kind?: unknown; path?: unknown; command?: unknown };
  if (req.kind === "file-read" && typeof req.path === "string" && req.path !== "") {
    return evaluateFileRead(request as FileReadRequest, opts);
  }
  if (req.kind === "shell" && typeof req.command === "string" && req.command !== "") {
    return evaluateShell(request as ShellRequest, opts);
  }
  return { decision: "allow" };
}
