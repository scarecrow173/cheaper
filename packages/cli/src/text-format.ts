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

interface ModelRow {
  provider?: string;
  id?: string;
  excluded?: boolean;
  tier?: string;
  tierSource?: string;
  effectiveCost?: number;
  effectiveCostPercentile?: number;
  workload?: { outputPerInput?: number };
  pricing?: {
    input?: number;
    output?: number;
    source?: string;
    snapshotVersion?: string;
    updatedAt?: string;
  } | null;
}

function modelRowText(m: ModelRow): string {
  const key = `${m.provider ?? "?"}/${m.id ?? "?"}`;
  const state = m.excluded === true ? "excluded" : "available";
  const auto =
    typeof m.effectiveCost === "number" && typeof m.effectiveCostPercentile === "number"
      ? ` eff=$${m.effectiveCost}/1M p=${m.effectiveCostPercentile.toFixed(2)}` +
        ` opi=${m.workload?.outputPerInput ?? "?"}`
      : "";
  const tier = `tier=${m.tier ?? "?"} (${m.tierSource ?? "?"})${auto}`;
  const p = m.pricing;
  const price =
    p == null || typeof p.input !== "number" || typeof p.output !== "number"
      ? "unpriced"
      : `in=$${p.input} out=$${p.output}/1M (${p.source ?? "?"}` +
        `${p.snapshotVersion !== undefined ? `@${p.snapshotVersion}` : ""}` +
        `${p.updatedAt !== undefined ? `, ${p.updatedAt}` : ""})`;
  return `  ${key.padEnd(44)} ${state.padEnd(9)} ${tier.padEnd(28)} ${price}`;
}

function utilityText(env: UtilitySuccessEnvelope): string {
  const r = env.result;
  const lines: string[] = [`duration: ${env.meta.durationMs}ms`, "result:", pretty(r)];
  const models = r.models;
  if (Array.isArray(models)) {
    return [
      "ok",
      `models: ${models.length}`,
      ...(models as ModelRow[]).map(modelRowText),
      `duration: ${env.meta.durationMs}ms`,
      "",
    ].join("\n");
  }
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
