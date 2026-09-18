/**
 * @cheaper/runtime-pi public API。
 * CLI（PLAN-0003）が consume する実行境界は runWorker(deps) 1 つ。
 * 残りは meta/結果型と model 解決 adapter、session seam の型のみ公開する。
 */

export type {
  AuthEvent,
  AuthInteraction,
  AuthPrompt,
  AuthRuntime,
  AuthType,
  ProviderAuthInfo,
} from "./auth.js";
export {
  describeProviderAuth,
  listProviderAuth,
  loginProvider,
  logoutProvider,
  registerBundledOAuthFlows,
} from "./auth.js";
export type { ChangesSummary, FileChange } from "./journal.js";
export { MutationJournal } from "./journal.js";
export {
  type CreateCheaperModelRuntimeOptions,
  createModelRuntime,
  type ModelCatalog,
  type ResolvedPiModel,
  resolvePiModel,
  resolvePiStatePaths,
  toModelInfo,
} from "./model-runtime.js";
export type { SchemaValidationError } from "./output.js";
export {
  bundledProviderPlugins,
  type CheaperProviderPlugin,
  type ProviderPluginContext,
} from "./provider-plugin.js";
export type { RunWorkerDeps, WorkerRunMeta } from "./run.js";
export { runWorker, WorkerRunError } from "./run.js";
export type {
  StreamFnLike,
  WorkerSessionHandle,
  WorkerSessionOptions,
} from "./session.js";
export { createWorkerSession } from "./session.js";
export type { UsageSnapshot, WorkerUsage } from "./usage.js";
export { UsageCollector } from "./usage.js";
