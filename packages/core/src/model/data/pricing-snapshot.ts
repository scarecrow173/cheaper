import type { PricingSnapshot } from "../pricing.js";

/**
 * bundled 補完 pricing snapshot（ADR-034rzI0Yqw3L8AFM9diZct）。
 * runtime catalog（Pi `Model.cost`、models.dev 由来）に pricing が
 * 無い model だけを補完する。exhaustive catalog ではなく、欠損が
 * 判明した model を随時追加する。version は手編集の release 単位。
 * esbuild bundle に埋め込むため runtime file IO を伴わない。
 */
export const PRICING_SNAPSHOT: PricingSnapshot = {
  version: "2026-09-19",
  updatedAt: "2026-09-19",
  models: {},
};
