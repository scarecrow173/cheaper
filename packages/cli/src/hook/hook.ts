// `cheaper hook <event> --target <agent>` entry point.
// This path is intentionally decoupled from runCli's envelope /
// recordUsage / generic-catch machinery: hook invocations render
// target-native output only, and every failure resolves to the
// target's fail-open form so a broken setup never blocks tool calls.
// Copilot treats ANY nonzero exit as a deny, so this command never
// returns a nonzero exit code.

import type { CliHooks, CliIo } from "../cli.js";
import {
  fallbackAllow,
  handleHook,
  HOOK_TARGETS,
  type HookResponse,
  type HookTarget,
} from "./targets.js";

const KNOWN_EVENTS = new Set([
  "pre-read-code",
  "pre-run-command",
  "pre-tool-use",
  "before-read-file",
  "before-shell-execution",
]);

interface HookArgv {
  event: string | undefined;
  target: HookTarget | undefined;
}

/** 緩い引数解析: 不正入力でも throw せず undefined を返す。 */
function parseHookArgv(argv: readonly string[]): HookArgv {
  let event: string | undefined;
  let target: string | undefined;
  const positional: string[] = [];
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === "--target") {
      target = argv[i + 1];
      i++;
      continue;
    }
    if (arg !== undefined && arg.startsWith("--target=")) {
      target = arg.slice("--target=".length);
      continue;
    }
    if (arg !== undefined && arg.startsWith("-")) {
      continue; // 未知 option は無視（fail-open）
    }
    if (arg !== undefined) positional.push(arg);
  }
  event = positional[0];
  return {
    event,
    target: (HOOK_TARGETS as readonly string[]).includes(target ?? "")
      ? (target as HookTarget)
      : undefined,
  };
}

async function readStdinText(io: CliIo): Promise<string> {
  try {
    if (io.stdin.isTTY === true) return "";
    const chunks: Buffer[] = [];
    for await (const chunk of io.stdin) {
      chunks.push(typeof chunk === "string" ? Buffer.from(chunk) : (chunk as Buffer));
    }
    return Buffer.concat(chunks).toString("utf8");
  } catch {
    return "";
  }
}

function emit(io: CliIo, response: HookResponse): number {
  try {
    if (response.stdout !== undefined) io.stdout.write(response.stdout);
    if (response.stderr !== undefined) io.stderr.write(response.stderr);
  } catch {
    // 出力失敗でも exit code は維持（fail-open）
  }
  return response.exit;
}

/**
 * `cheaper hook` の dispatch。全 error を target 適合の allow +
 * exit 0 に変換する catch-all を持ち、runCli の汎用 catch には
 * 到達しない（呼び出し側は早期 return で dispatch する）。
 */
export async function executeHook(
  argv: readonly string[],
  io: CliIo,
  hooks: CliHooks,
): Promise<number> {
  const { event, target } = parseHookArgv(argv);
  if (event === undefined || target === undefined || !KNOWN_EVENTS.has(event)) {
    return emit(io, fallbackAllow(target));
  }
  try {
    const text = await readStdinText(io);
    let input: unknown;
    try {
      input = JSON.parse(text);
    } catch {
      // malformed stdin: cursor は schema-valid JSON 必須、他は silent
      return emit(io, fallbackAllow(target));
    }
    const response = await handleHook(target, event, input, {
      env: io.env,
      builtinDir: hooks.builtinDir,
      cwd: io.cwd,
    });
    return emit(io, response);
  } catch {
    return emit(io, fallbackAllow(target));
  }
}
