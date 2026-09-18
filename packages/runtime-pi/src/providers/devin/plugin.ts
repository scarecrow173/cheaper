/**
 * Devin Local provider plugin（SPEC-0026 / DESIGN-0031）。
 * `registerProvider("devin", ...)` で auth（Devin CLI 委譲）・
 * catalog（`devin models list`）・stream（vendored Connect
 * transport）を登録する。Pi extension・slash command・event
 * handler は持ち込まない（ADR-0028/0029）。
 */
import type { OAuthCredentials } from "@earendil-works/pi-ai";
import type { ProviderModelConfig } from "@earendil-works/pi-coding-agent";
import type { CheaperProviderPlugin } from "../../provider-plugin.js";
import { createDevinCli, type DevinCliRunner } from "./cli.js";
import { devinDataDir, loginWithDevinCli, readDevinCredentials } from "./credentials.js";
import { FALLBACK_MODELS, loadCliCatalog, modelsFromCatalog } from "./models.js";
import { streamDevin } from "./stream.js";

const PROVIDER_ID = "devin";
/** protocol 固定の Connect エンドポイント（catalog 非依存）。 */
const BASE_URL = "https://server.codeium.com";

function toOAuthCredentials(apiKey: string): OAuthCredentials {
  return {
    refresh: "",
    access: apiKey,
    expires: Date.now() + 365 * 24 * 60 * 60 * 1000,
  };
}

export interface DevinProviderPluginDeps {
  /** test が注入する child-process runner seam。 */
  runner?: DevinCliRunner;
}

export function createDevinProviderPlugin(deps?: DevinProviderPluginDeps): CheaperProviderPlugin {
  return {
    id: PROVIDER_ID,
    register(runtime, context) {
      const dataDir = devinDataDir(context.userConfigDir);
      const cli = createDevinCli({
        env: context.env,
        dataDir,
        ...(deps?.runner !== undefined ? { runner: deps.runner } : {}),
      });

      const registerModels = (models: ProviderModelConfig[]): void => {
        runtime.registerProvider(PROVIDER_ID, {
          name: "Devin Local",
          api: "devin-local",
          baseUrl: BASE_URL,
          models,
          oauth: {
            name: "Devin CLI",
            async login(): Promise<OAuthCredentials> {
              const creds = await loginWithDevinCli(cli, dataDir);
              // login 直後に live catalog を反映（失敗しても login 自体は成功）。
              try {
                registerModels(modelsFromCatalog(await loadCliCatalog(cli)));
              } catch {
                // bootstrap model list のまま
              }
              return toOAuthCredentials(creds.apiKey);
            },
            async refreshToken(credentials): Promise<OAuthCredentials> {
              const creds = readDevinCredentials(dataDir);
              return creds === null ? credentials : toOAuthCredentials(creds.apiKey);
            },
            getApiKey(credentials): string {
              // credentials.toml（rotated token 拾い）優先、stored credential に fallback。
              return readDevinCredentials(dataDir)?.apiKey ?? credentials.access;
            },
          },
          streamSimple: streamDevin,
          async refreshModels({ credential }) {
            if (credential === undefined) return FALLBACK_MODELS;
            return modelsFromCatalog(await loadCliCatalog(cli));
          },
        });
      };

      registerModels(FALLBACK_MODELS);
    },
  };
}
