import type { ModelRuntime } from "@earendil-works/pi-coding-agent";
import { createDevinProviderPlugin } from "./providers/devin/plugin.js";

/**
 * plugin が provider 登録に使う最小 context（ADR-0028）。
 * credential 値は渡さず、config 解決に必要な情報のみを含む。
 */
export interface ProviderPluginContext {
  /** runtime 生成時に注入された env（既定は `process.env`）。 */
  readonly env: NodeJS.ProcessEnv;
  /**
   * cheaper の user-scope config dir。
   * plugin 固有の永続 data（child process の state 等）は
   * この配下に置く。project-local `.cheaper/` には置かない。
   */
  readonly userConfigDir: string;
}

/**
 * cheaper が明示的に bundle する provider plugin の contract
 *（SPEC-0025 / ADR-0028）。
 *
 * plugin がしてよいこと（register で `ModelRuntime` に登録する
 * 範囲）:
 * - model provider の登録（`registerProvider` /
 *   `registerNativeProvider`）
 * - provider 固有 auth flow の登録
 * - provider 固有 model catalog / refresh logic の登録
 * - provider 固有 `streamSimple` 等の model transport の登録
 *
 * plugin に持ち込まないこと（contract に面が存在しない）:
 * - Pi tool / slash command / keyboard shortcut の登録
 * - Worker prompt / skill / theme / AGENTS.md
 * - session event handler
 * - general-purpose Pi extension の discovery / loading
 */
export interface CheaperProviderPlugin {
  readonly id: string;
  register(runtime: ModelRuntime, context: ProviderPluginContext): void | Promise<void>;
}

/**
 * cheaper が明示的に bundle する provider plugin の一覧。
 * external discovery は行わない（ADR-0028）。
 */
export function bundledProviderPlugins(): readonly CheaperProviderPlugin[] {
  return [createDevinProviderPlugin()];
}
