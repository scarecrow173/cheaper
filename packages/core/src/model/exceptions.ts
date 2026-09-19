import { matchModelPattern } from "./ref.js";
import type { Tier } from "./tier.js";

/**
 * built-in curated exception entry。pricing metadata だけでは扱えない
 * model の tier を理由・provenance 付きで固定する
 * （exhaustive な tier list ではない。issue #45 / ADR-034rzI0Yqw3L8AFM9diZct）。
 */
export interface ModelException {
  tier: Tier;
  /** 適用理由（なぜ pricing/auto 分類で扱えないか） */
  reason: string;
  /** 根拠となる情報源（issue / doc / 外部 URL） */
  source: string;
  /** entry を最後に確認した日（ISO date） */
  checkedAt: string;
}

/**
 * bundled curated exceptions。key は "<provider>/<id>" exact または
 * "<provider>/*" wildcard。
 */
export const MODEL_EXCEPTIONS: Readonly<Record<string, ModelException>> = {
  // local model は price=0 が API 価格を意味しないため auto 分類に任せず
  // low に固定する（issue #45 非目標「price=0 だけで low にしない」の例外側）
  "ollama/*": {
    tier: "low",
    reason: "local Ollama model; zero price は API 価格を意味しないため curated で low に固定",
    source: "https://github.com/scarecrow173/cheaper/issues/45",
    checkedAt: "2026-09-19",
  },
};

/**
 * exception table から model の entry を返す。exact key を wildcard
 * より優先し、wildcard 間は key の辞書順で最初の一致を返す
 * （決定的にするため）。
 */
export function findModelException(
  provider: string,
  id: string,
  exceptions: Readonly<Record<string, ModelException>> = MODEL_EXCEPTIONS,
): ModelException | undefined {
  const exact = exceptions[`${provider}/${id}`];
  if (exact !== undefined) return exact;
  for (const key of Object.keys(exceptions).sort()) {
    if (key.endsWith("/*") && matchModelPattern(provider, id, key)) {
      return exceptions[key];
    }
  }
  return undefined;
}
