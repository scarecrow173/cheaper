import type {
  CliEnvelope,
  ErrorEnvelope,
  SuccessEnvelope,
  UtilitySuccessEnvelope,
} from "./envelope.js";

function pretty(v: unknown): string {
  return JSON.stringify(v, null, 2);
}

function successText(env: SuccessEnvelope): string {
  const m = env.meta;
  const lines: string[] = [
    "ok",
    `worker:   ${env.worker}`,
    `model:    ${m.model.provider}/${m.model.id} (tier ${m.model.tier})`,
    `usage:    in=${m.usage.input} out=${m.usage.output} cacheRead=${m.usage.cacheRead} cacheWrite=${m.usage.cacheWrite} total=${m.usage.totalTokens} cost=$${m.usage.costUsd.toFixed(6)}`,
    `turns:    ${m.turns}  toolCalls=${m.toolCalls} rejected=${m.rejectedToolCalls} repairs=${m.repairs} retries=${m.transientRetries}`,
    `duration: ${m.durationMs}ms`,
    `changes:  ${m.changes.totals.files} files (+${m.changes.totals.linesAdded}/-${m.changes.totals.linesDeleted})`,
  ];
  for (const f of m.changes.files) {
    lines.push(`  ${f.kind.padEnd(8)} ${f.path} (+${f.linesAdded}/-${f.linesDeleted})`);
  }
  lines.push("result:", pretty(env.result));
  return `${lines.join("\n")}\n`;
}

function utilityText(env: UtilitySuccessEnvelope): string {
  const r = env.result;
  const lines: string[] = [`duration: ${env.meta.durationMs}ms`, "result:", pretty(r)];
  const t = r.tokenizer as { name?: string; implementation?: string; version?: string } | undefined;
  if (
    typeof r.path === "string" &&
    typeof r.tokens === "number" &&
    t !== undefined &&
    typeof t.name === "string"
  ) {
    lines.unshift(
      "ok",
      `path:      ${r.path}`,
      `tokens:    ${r.tokens}`,
      `bytes:     ${typeof r.bytes === "number" ? r.bytes : "-"}`,
      `tokenizer: ${t.name} (${t.implementation ?? "?"} ${t.version ?? "?"})`,
    );
  } else {
    lines.unshift("ok");
  }
  return `${lines.join("\n")}\n`;
}

function errorText(env: ErrorEnvelope): string {
  const e = env.error;
  const m = env.meta;
  const lines: string[] = [
    `error ${e.code}`,
    `worker:   ${env.worker ?? "-"}`,
    `message:  ${e.message}`,
    `exit:     ${e.exitCode}  retryable=${e.retryable}`,
    `duration: ${m.durationMs ?? "-"}ms`,
  ];
  if (m.changes !== null) {
    lines.push(
      `changes:  ${m.changes.totals.files} files (+${m.changes.totals.linesAdded}/-${m.changes.totals.linesDeleted})`,
    );
    for (const f of m.changes.files) {
      lines.push(`  ${f.kind.padEnd(8)} ${f.path} (+${f.linesAdded}/-${f.linesDeleted})`);
    }
  }
  lines.push("error:", pretty(e));
  return `${lines.join("\n")}\n`;
}

/** `--format text` の人間向け出力（DESIGN-0005）。機械可読ではない。 */
export function formatText(env: CliEnvelope): string {
  if (!env.ok) return errorText(env);
  if (env.worker === null) return utilityText(env);
  return successText(env);
}
