import { mkdir } from "node:fs/promises";
import path from "node:path";
import {
  CheaperError,
  type ModelConfig,
  type ModelInfo,
  resolveModel,
  resolveUserConfigDir,
  type Tier,
} from "@cheaper/core";
import type { Api, Model } from "@earendil-works/pi-ai";
import { type CreateModelRuntimeOptions, ModelRuntime } from "@earendil-works/pi-coding-agent";
import {
  bundledProviderPlugins,
  type CheaperProviderPlugin,
  type ProviderPluginContext,
} from "./provider-plugin.js";

/**
 * ModelRuntime のうち runtime-pi が必要とする最小面。
 * 実 ModelRuntime・テスト stub の双方を受けられる構造型。
 */
export interface ModelCatalog {
  getAvailable(providerId?: string): Promise<readonly Model<Api>[]>;
  getModel(providerId: string, modelId: string): Model<Api> | undefined;
}

/** Pi Model → core の最小 shape（DESIGN-0003 の ModelInfo）。 */
export function toModelInfo(m: Model<Api>): ModelInfo {
  return {
    provider: m.provider,
    id: m.id,
    cost: { input: m.cost.input, output: m.cost.output },
    maxTokens: m.maxTokens,
  };
}

/**
 * `createModelRuntime` の options。Pi の `CreateModelRuntimeOptions`
 * に cheaper 固有の `env` seam を加えたもの。
 */
export interface CreateCheaperModelRuntimeOptions extends CreateModelRuntimeOptions {
  /**
   * user-scope config dir 解決に使う env。未指定時は `process.env`。
   * test は `CHEAPER_CONFIG_DIR` を tmpdir に向けて注入する。
   */
  env?: NodeJS.ProcessEnv;
  /**
   * 登録する provider plugin。未指定時は `bundledProviderPlugins()`
   *（ADR-0028）。test はこの seam で fake plugin を注入する。
   */
  providerPlugins?: readonly CheaperProviderPlugin[];
}

/**
 * Pi 永続 state（auth.json / models.json / models-store.json）の
 * 保存先を組み立てる唯一の場所（ADR-0027）。
 * `<userConfigDir>/pi/` 配下に固定する。
 */
export function resolvePiStatePaths(userConfigDir: string): {
  piDir: string;
  authPath: string;
  modelsPath: string;
  modelsStorePath: string;
} {
  const piDir = path.join(userConfigDir, "pi");
  return {
    piDir,
    authPath: path.join(piDir, "auth.json"),
    modelsPath: path.join(piDir, "models.json"),
    modelsStorePath: path.join(piDir, "models-store.json"),
  };
}

/**
 * CLI 用の薄い ModelRuntime factory。cli が `@earendil-works/*` に
 * 直接依存しないよう、生成だけをここに閉じ込める。
 * 返り値は ModelCatalog を満たすので `runWorker` の catalog/modelRuntime
 * 双方にそのまま渡せる。
 *
 * Pi state は cheaper の user-scope config dir 配下（`<dir>/pi/`）に
 * 集約する（SPEC-0024 / ADR-0027）。`PI_CODING_AGENT_DIR` は使わない。
 * caller が `authPath` 等を明示した場合は既定より優先される。
 *
 * bundled provider plugin は最初の availability/model resolution
 * より前に登録するため（ADR-0028）、create 時 refresh を遅延し、
 * plugin 登録後に create と同値の `refresh()` を実行する。
 * caller が `refreshOnCreate: false` を明示した場合は manual
 * refresh を省略する。
 */
export async function createModelRuntime(
  options?: CreateCheaperModelRuntimeOptions,
): Promise<ModelRuntime> {
  const { env, providerPlugins, ...rest } = options ?? {};
  const effectiveEnv = env ?? process.env;
  const userConfigDir = resolveUserConfigDir({ env });
  const paths = resolvePiStatePaths(userConfigDir);
  await mkdir(paths.piDir, { recursive: true, mode: 0o700 });
  const runtime = await ModelRuntime.create({
    authPath: paths.authPath,
    modelsPath: paths.modelsPath,
    modelsStorePath: paths.modelsStorePath,
    ...rest,
    refreshOnCreate: false,
  });
  const context: ProviderPluginContext = { env: effectiveEnv, userConfigDir };
  for (const plugin of providerPlugins ?? bundledProviderPlugins()) {
    await plugin.register(runtime, context);
  }
  if (rest.refreshOnCreate === false) return runtime;
  // ModelRuntime.create の create 時 refresh と同値に mirror する
  //（allowModelNetwork && PI_OFFLINE 未設定、timeout + signal 合成）。
  const allowNetwork = rest.allowModelNetwork === true && effectiveEnv.PI_OFFLINE === undefined;
  const timeoutMs = rest.modelRefreshTimeoutMs;
  const controller = allowNetwork && timeoutMs !== undefined ? new AbortController() : undefined;
  const timeout = controller ? setTimeout(() => controller.abort(), timeoutMs) : undefined;
  const signal = controller
    ? rest.signal !== undefined
      ? AbortSignal.any([rest.signal, controller.signal])
      : controller.signal
    : rest.signal;
  try {
    await runtime.refresh({
      allowNetwork,
      ...(signal !== undefined ? { signal } : {}),
    });
  } finally {
    if (timeout !== undefined) clearTimeout(timeout);
  }
  return runtime;
}

export interface ResolvedPiModel {
  /** createAgentSession に渡す Pi Model。 */
  model: Model<Api>;
  /** 実際に選ばれた tier。 */
  tier: Tier;
  /** core 側の解決済み model 情報（meta.model 用）。 */
  info: ModelInfo;
}

/**
 * catalog.getAvailable() → ModelInfo 変換 → core resolveModel → getModel で実体解決。
 * available が 0 件なら resolveModel が MODEL_TIER_UNAVAILABLE
 * （details.noAuthenticatedModels）を投げる。
 */
export async function resolvePiModel(opts: {
  catalog: ModelCatalog;
  requestedTier: Tier;
  policy: { maxTier: Tier; tierFallback: "none" | "higher" };
  config: ModelConfig;
}): Promise<ResolvedPiModel> {
  const available = (await opts.catalog.getAvailable()).map(toModelInfo);
  const resolved = resolveModel({
    available,
    requestedTier: opts.requestedTier,
    policy: opts.policy,
    config: opts.config,
  });
  const model = opts.catalog.getModel(resolved.model.provider, resolved.model.id);
  if (model === undefined) {
    throw new CheaperError(
      "INTERNAL_ERROR",
      `resolved model disappeared: ${resolved.model.provider}/${resolved.model.id}`,
      { details: { provider: resolved.model.provider, id: resolved.model.id } },
    );
  }
  return { model, tier: resolved.tier, info: resolved.model };
}
