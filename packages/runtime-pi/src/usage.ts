import type { AgentSessionEvent } from "@earendil-works/pi-coding-agent";

export interface WorkerUsage {
  input: number;
  output: number;
  cacheRead: number;
  cacheWrite: number;
  totalTokens: number;
  costUsd: number;
}

/**
 * 親コンテキストからの offload 実測（実際値。推定ではない）。
 * - `sourceTokens`: 全 tool output text の canonical token 和
 * - `returnedTokens`: 成功時は `JSON.stringify(result)` の token 数。
 *   失敗で result が無いときは 0
 * - `avoidedContextTokens`: `max(0, sourceTokens - returnedTokens)`
 */
export interface WorkerSavings {
  sourceTokens: number;
  returnedTokens: number;
  avoidedContextTokens: number;
  /**
   * source↔returned を親 model の直接実行コストと直接比較できる
   * workload か（read-only worker のみ true。core の
   * `isDirectlyComparableWorker` で判定）。false の record は
   * `cheaper usage` の estimated 算出の母数に含めない。
   * 旧 schema の record には存在しない（非 comparable 扱い）。
   */
  directlyComparable: boolean;
}

export interface UsageSnapshot {
  usage: WorkerUsage;
  turns: number;
  transientRetries: number;
}

/**
 * session イベントから usage/turn/retry を逐次集計する（DESIGN-0004）。
 * - message_end（assistant）の message.usage を加算
 * - turn_end の数を turns に
 * - auto_retry_start の数を transientRetries に（cheaper 独自 retry は持たない）
 */
export class UsageCollector {
  private input = 0;
  private output = 0;
  private cacheRead = 0;
  private cacheWrite = 0;
  private totalTokens = 0;
  private costUsd = 0;
  private turns = 0;
  private transientRetries = 0;

  handleEvent(event: { type: string; message?: unknown }): void {
    switch (event.type) {
      case "turn_end":
        this.turns += 1;
        break;
      case "auto_retry_start":
        this.transientRetries += 1;
        break;
      case "message_end": {
        const m = event.message;
        if (typeof m === "object" && m !== null && (m as { role?: unknown }).role === "assistant") {
          const u = (m as { usage?: Record<string, unknown> }).usage;
          if (u) {
            this.input += num(u.input);
            this.output += num(u.output);
            this.cacheRead += num(u.cacheRead);
            this.cacheWrite += num(u.cacheWrite);
            this.totalTokens += num(u.totalTokens);
            const cost = (u as { cost?: { total?: unknown } }).cost;
            this.costUsd += num(cost?.total);
          }
        }
        break;
      }
    }
  }

  /** AgentSessionEvent をそのまま渡せる型付き入口。 */
  handleSessionEvent(event: AgentSessionEvent): void {
    this.handleEvent(event);
  }

  snapshot(): UsageSnapshot {
    return {
      usage: {
        input: this.input,
        output: this.output,
        cacheRead: this.cacheRead,
        cacheWrite: this.cacheWrite,
        totalTokens: this.totalTokens,
        costUsd: this.costUsd,
      },
      turns: this.turns,
      transientRetries: this.transientRetries,
    };
  }
}

function num(v: unknown): number {
  return typeof v === "number" && Number.isFinite(v) ? v : 0;
}
