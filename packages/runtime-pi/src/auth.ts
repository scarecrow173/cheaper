/**
 * Provider auth（login/status/logout）を cheaper CLI に吸収する adapter
 * （SPEC-0011 / DESIGN-0016）。standalone `pi` 実行ファイルは要求しない。
 * credential 値（access/refresh/api key）は一切返さず、
 * providerId / type / configured などの安全な metadata のみ公開する。
 */
import { CheaperError } from "@cheaper/core";
import type { AuthInteraction, AuthType } from "@earendil-works/pi-ai";
import { registerBunOAuthFlows } from "@earendil-works/pi-ai/bun-oauth";
import type { createModelRuntime } from "./model-runtime.js";

export type { AuthEvent, AuthInteraction, AuthPrompt, AuthType } from "@earendil-works/pi-ai";

export type AuthRuntime = Awaited<ReturnType<typeof createModelRuntime>>;

/**
 * pi-ai は provider 別 OAuth flow を bundler が辿れない可変 specifier の
 * dynamic import で読む（`dist/auth/oauth/load.js`）。esbuild の単一ファイル
 * bundle ではこの import が解決不能になるため、bun-oauth entry が用意する
 * static loader 登録で差し替える。非 bundle 環境で呼んでも无害（同じ flow
 * module を eager import するだけ）。
 */
export function registerBundledOAuthFlows(): void {
  registerBunOAuthFlows();
}

export interface ProviderAuthInfo {
  providerId: string;
  /** 人間向け display name（例 "OpenAI (ChatGPT Plus/Pro)"）。 */
  name: string;
  /** 対話 login を実行できる auth method。ambient-only provider では空/部分。 */
  methods: AuthType[];
  /** OAuth method が subscription 課金に紐づくか。 */
  subscription: boolean;
  /** 現在 credential が設定済みか。 */
  configured: boolean;
}

function invalidRequest(message: string, details?: Record<string, unknown>): CheaperError {
  return new CheaperError("INVALID_REQUEST", message, details === undefined ? {} : { details });
}

function requireProvider(runtime: AuthRuntime, providerId: string) {
  const provider = runtime.getProvider(providerId);
  if (provider === undefined) {
    throw invalidRequest(`unknown provider: ${providerId}`, { providerId });
  }
  return provider;
}

/** provider の auth capability と設定状態を集約する（credential 値は含まない）。 */
export async function describeProviderAuth(
  runtime: AuthRuntime,
  providerId: string,
): Promise<ProviderAuthInfo> {
  const provider = requireProvider(runtime, providerId);
  const methods: AuthType[] = [];
  if (provider.auth.apiKey?.login !== undefined) methods.push("api_key");
  if (provider.auth.oauth !== undefined) methods.push("oauth");
  const check = await runtime.checkAuth(providerId);
  return {
    providerId,
    name: provider.name,
    methods,
    subscription: provider.auth.oauth?.isSubscription === true,
    configured: check !== undefined,
  };
}

/** 全 provider の auth 状態一覧（login 可能 method 非表示の provider も含む）。 */
export async function listProviderAuth(runtime: AuthRuntime): Promise<ProviderAuthInfo[]> {
  const providers = runtime.getProviders();
  const out: ProviderAuthInfo[] = [];
  for (const p of providers) {
    out.push(await describeProviderAuth(runtime, p.id));
  }
  return out;
}

/**
 * 対話 login。`interaction` は CLI 層が terminal I/O を bridge したもの。
 * Credential は返さない（永続化は ModelRuntime が authPath に対して行う）。
 */
export async function loginProvider(
  runtime: AuthRuntime,
  providerId: string,
  type: AuthType,
  interaction: AuthInteraction,
): Promise<ProviderAuthInfo> {
  const provider = requireProvider(runtime, providerId);
  const supports =
    type === "oauth"
      ? provider.auth.oauth !== undefined
      : provider.auth.apiKey?.login !== undefined;
  if (!supports) {
    throw invalidRequest(`provider ${providerId} does not support ${type} login`, {
      providerId,
      type,
    });
  }
  await runtime.login(providerId, type, interaction);
  return describeProviderAuth(runtime, providerId);
}

/** credential 削除。未登録 provider でも冪等に成功扱いする。 */
export async function logoutProvider(
  runtime: AuthRuntime,
  providerId: string,
): Promise<ProviderAuthInfo> {
  requireProvider(runtime, providerId);
  await runtime.logout(providerId);
  return describeProviderAuth(runtime, providerId);
}
