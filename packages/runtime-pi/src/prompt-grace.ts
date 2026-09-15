import type { AgentSession } from "@earendil-works/pi-coding-agent";

export type PromptOutcome =
  | { kind: "settled" }
  | { kind: "rejected"; err: unknown }
  | { kind: "grace" };

export interface PromptGraceOptions {
  /** run 全体の集約 abort signal（runAbort.signal）。abort 時に session.abort() を呼ぶ。 */
  signal?: AbortSignal;
  /** abort 後に prompt() settle を待つ猶予 ms。 */
  graceMs: number;
}

/**
 * `session.prompt()` を abort grace 付きで実行する（DESIGN-0004/0010）。
 * signal abort 時に `session.abort()` を呼び、`graceMs` 以内に prompt() が
 * settle しなければ `"grace"` で待機を打ち切る（stream が signal を無視する
 * provider でのハング防止）。grace timer は prompt settle で clearTimeout し、
 * event loop を保持しない。main session と repair session で共用する。
 */
export async function promptWithAbortGrace(
  session: AgentSession,
  prompt: string,
  opts: PromptGraceOptions,
): Promise<PromptOutcome> {
  let resolveGrace: () => void = () => {};
  const gracePromise = new Promise<void>((r) => {
    resolveGrace = r;
  });
  let graceTimer: ReturnType<typeof setTimeout> | undefined;
  const onAbort = () => {
    void session.abort();
    graceTimer ??= setTimeout(resolveGrace, opts.graceMs);
  };
  const signal = opts.signal;
  // 既に abort 済みの signal に対して listener は発火しないため、
  // 登録後に明示的に onAbort を呼ぶ（2 回呼ばれても graceTimer は ??= で 1 つだけ）。
  signal?.addEventListener("abort", onAbort, { once: true });
  if (signal?.aborted) onAbort();

  const promptOutcome = session.prompt(prompt).then(
    () => ({ kind: "settled" as const }),
    (err: unknown) => ({ kind: "rejected" as const, err }),
  );
  try {
    return await Promise.race([
      promptOutcome,
      gracePromise.then(() => ({ kind: "grace" as const })),
    ]);
  } finally {
    if (graceTimer !== undefined) clearTimeout(graceTimer);
    signal?.removeEventListener("abort", onAbort);
  }
}
