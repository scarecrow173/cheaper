/**
 * `cheaper auth` — provider 認証を CLI に吸収する（SPEC-0011 / DESIGN-0016）。
 * standalone `pi` は要求しない。ModelRuntime が authPath へ credential を
 * 永続化する。token 等の secret 値は stdout/stderr いずれにも出さない。
 */
import { spawn } from "node:child_process";
import { createInterface, type Interface } from "node:readline/promises";
import { CheaperError } from "@cheaper/core";
import {
  type AuthEvent,
  type AuthInteraction,
  type AuthPrompt,
  type AuthRuntime,
  createModelRuntime,
  describeProviderAuth,
  listProviderAuth,
  loginProvider,
  logoutProvider,
} from "@cheaper/runtime-pi";
import type { AuthCliArgs } from "./args.js";
import type { CliIo } from "./cli.js";
import { type CliEnvelope, utilitySuccessEnvelope } from "./envelope.js";

function invalidRequest(message: string, details?: Record<string, unknown>): CheaperError {
  return new CheaperError("INVALID_REQUEST", message, details === undefined ? {} : { details });
}

/** auth_url を browser で開く best-effort。失敗しても flow は継続する。 */
function openUrlBestEffort(url: string): void {
  const cmd =
    process.platform === "win32"
      ? { file: "cmd", args: ["/c", "start", "", url] }
      : process.platform === "darwin"
        ? { file: "open", args: [url] }
        : { file: "xdg-open", args: [url] };
  try {
    const child = spawn(cmd.file, cmd.args, { detached: true, stdio: "ignore" });
    child.on("error", () => {});
    child.unref();
  } catch {
    // URL は stderr に表示済みなので起動失敗は無視する
  }
}

/**
 * terminal I/O → AuthInteraction の bridge。
 * `select` prompt は `--method` が option id に一致すれば非対話で即答する。
 * stdin が非 TTY のとき対話 prompt は INVALID_REQUEST で失敗する。
 */
export function createTerminalInteraction(
  io: CliIo,
  opts: { method?: string | undefined; signal?: AbortSignal | undefined },
): AuthInteraction {
  let rl: Interface | undefined;
  const ask = async (message: string, signal?: AbortSignal): Promise<string> => {
    rl ??= createInterface({ input: io.stdin, output: process.stderr });
    const signals = [opts.signal, signal].filter((s): s is AbortSignal => s !== undefined);
    const combined =
      signals.length === 0
        ? undefined
        : signals.length === 1
          ? signals[0]
          : AbortSignal.any(signals);
    try {
      return (
        await rl.question(message, combined === undefined ? {} : { signal: combined })
      ).trim();
    } finally {
      rl.close();
      rl = undefined;
    }
  };

  return {
    ...(opts.signal !== undefined ? { signal: opts.signal } : {}),
    notify(event: AuthEvent): void {
      switch (event.type) {
        case "auth_url":
          io.stderr.write(`\nOpen this URL to sign in:\n  ${event.url}\n\n`);
          if (event.instructions !== undefined) io.stderr.write(`${event.instructions}\n`);
          openUrlBestEffort(event.url);
          break;
        case "device_code":
          io.stderr.write(`\nGo to ${event.verificationUri} and enter code: ${event.userCode}\n`);
          if (event.expiresInSeconds !== undefined) {
            io.stderr.write(`(expires in ${event.expiresInSeconds}s)\n`);
          }
          break;
        case "info":
        case "progress":
          io.stderr.write(`${event.message}\n`);
          if (event.type === "info" && event.links !== undefined) {
            for (const link of event.links) {
              io.stderr.write(`  ${link.label ?? "link"}: ${link.url}\n`);
            }
          }
          break;
      }
    },
    async prompt(p: AuthPrompt): Promise<string> {
      if (p.type === "select") {
        if (opts.method !== undefined) {
          const hit = p.options.find((o) => o.id === opts.method);
          if (hit !== undefined) return hit.id;
          io.stderr.write(
            `note: --method ${opts.method} did not match options (${p.options.map((o) => o.id).join(", ")})\n`,
          );
        }
        if (io.stdin.isTTY !== true) {
          throw invalidRequest(
            `interactive selection required; re-run with --method <${p.options.map((o) => o.id).join("|")}>`,
            { options: p.options.map((o) => o.id) },
          );
        }
        io.stderr.write(`${p.message}\n`);
        p.options.forEach((o, i) => {
          io.stderr.write(
            `  ${i + 1}) ${o.label}${o.description !== undefined ? ` — ${o.description}` : ""}\n`,
          );
        });
        const answer = await ask(`select [1-${p.options.length}]: `, p.signal);
        const n = Number.parseInt(answer, 10);
        const opt = Number.isInteger(n) ? p.options[n - 1] : undefined;
        if (opt === undefined) {
          throw invalidRequest(`invalid selection: ${answer}`, {
            options: p.options.map((o) => o.id),
          });
        }
        return opt.id;
      }
      if (io.stdin.isTTY !== true) {
        throw invalidRequest(`auth ${p.type} prompt requires an interactive terminal`, {
          promptType: p.type,
        });
      }
      return ask(`${p.message}: `, p.signal);
    },
  };
}

type Runtime = AuthRuntime;

/**
 * `auth` サブコマンド実行。stdout contract は既存と同じ envelope 1 つ。
 * login は SIGINT/SIGTERM を AbortSignal に中継して OAuth flow を中断する。
 */
export async function executeAuth(
  args: AuthCliArgs,
  io: CliIo,
  hooks: { createModelRuntime?: () => Promise<Runtime> | Runtime; signal?: AbortSignal },
  startedAt: number,
): Promise<{ envelope: CliEnvelope; exitCode: number }> {
  const runtime = await (hooks.createModelRuntime ?? createModelRuntime)();
  const now = Date.now;

  if (args.action === "status") {
    const providers =
      args.provider !== undefined
        ? [await describeProviderAuth(runtime, args.provider)]
        : await listProviderAuth(runtime);
    const credentials = (await runtime.listCredentials()).map((c) => ({
      providerId: c.providerId,
      type: c.type,
    }));
    return {
      envelope: utilitySuccessEnvelope({
        result: { credentials, providers },
        durationMs: now() - startedAt,
      }),
      exitCode: 0,
    };
  }

  const provider = args.provider as string;

  if (args.action === "logout") {
    await logoutProvider(runtime, provider);
    return {
      envelope: utilitySuccessEnvelope({
        result: { provider, loggedOut: true },
        durationMs: now() - startedAt,
      }),
      exitCode: 0,
    };
  }

  const ac = new AbortController();
  const onAbort = () => ac.abort();
  process.on("SIGINT", onAbort);
  process.on("SIGTERM", onAbort);
  hooks.signal?.addEventListener("abort", onAbort, { once: true });
  if (hooks.signal?.aborted === true) ac.abort();
  try {
    const interaction = createTerminalInteraction(io, { method: args.method, signal: ac.signal });
    const info = await loginProvider(runtime, provider, "oauth", interaction);
    return {
      envelope: utilitySuccessEnvelope({
        result: {
          provider: info.providerId,
          authenticated: info.configured,
          method: "oauth",
          subscription: info.subscription,
        },
        durationMs: now() - startedAt,
      }),
      exitCode: 0,
    };
  } finally {
    process.off("SIGINT", onAbort);
    process.off("SIGTERM", onAbort);
    hooks.signal?.removeEventListener("abort", onAbort);
  }
}
