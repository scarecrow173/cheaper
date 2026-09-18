/**
 * Devin CLI credential 読み取り — cheaper 専用 data dir のみ参照。
 * upstream pi-devin（MIT）の src/credentials.ts を、global
 * `~/.local/share/devin` ではなく cheaper data dir を読む形に
 * 改変（ADR-0029）。global credential・Devin Desktop credential
 * は暗黙利用しない。
 */
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import type { DevinCli } from "./cli.js";

export interface DevinCredentials {
  apiKey: string;
  apiServerUrl: string;
  webappHost: string;
  apiUrl: string;
  path: string;
}

/**
 * Devin child process 専用の data dir。
 * `<userConfigDir>/providers/devin/data` — child env の
 * `XDG_DATA_HOME` にそのまま渡す（Issue #27 の想定構成）。
 */
export function devinDataDir(userConfigDir: string): string {
  return path.join(userConfigDir, "providers", "devin", "data");
}

/**
 * `XDG_DATA_HOME=<dataDir>` の Devin CLI が書く credential path。
 * `<dataDir>/devin/credentials.toml`。
 */
export function devinCredentialsPath(dataDir: string): string {
  return path.join(dataDir, "devin", "credentials.toml");
}

function parseTomlStrings(text: string): Record<string, string> {
  const out: Record<string, string> = {};
  for (const line of text.split("\n")) {
    const match = line.match(/^\s*([A-Za-z0-9_]+)\s*=\s*"(.*)"\s*$/);
    if (match?.[1] !== undefined && match[2] !== undefined) {
      out[match[1]] = match[2];
    }
  }
  return out;
}

/**
 * cheaper data dir 内の credentials.toml だけを読む。
 * `~/.local/share/devin` 等の global path は参照しない。
 */
export function readDevinCredentials(dataDir: string): DevinCredentials | null {
  const credPath = devinCredentialsPath(dataDir);
  if (!existsSync(credPath)) return null;
  const raw = parseTomlStrings(readFileSync(credPath, "utf8"));
  const apiKey = raw.windsurf_api_key || raw.api_key;
  if (!apiKey) return null;
  return {
    apiKey,
    apiServerUrl: (raw.api_server_url || "https://server.codeium.com").replace(/\/$/, ""),
    webappHost: raw.devin_webapp_host || "app.devin.ai",
    apiUrl: raw.devin_api_url || "https://api.devin.ai",
    path: credPath,
  };
}

export async function devinAuthStatus(
  cli: DevinCli,
  dataDir: string,
): Promise<{ loggedIn: boolean; summary: string }> {
  const creds = readDevinCredentials(dataDir);
  try {
    const { stdout, stderr, code } = await cli.run(["auth", "status"], {
      timeoutMs: 15_000,
    });
    const text = `${stdout}\n${stderr}`.trim();
    const loggedIn = code === 0 && /logged in/i.test(text);
    return {
      loggedIn: loggedIn || creds !== null,
      summary: text || (creds !== null ? "credentials.toml present" : "not signed in"),
    };
  } catch (error) {
    if (creds !== null) {
      return {
        loggedIn: true,
        summary: `Devin credentials present at ${devinCredentialsPath(dataDir)}`,
      };
    }
    return {
      loggedIn: false,
      summary: error instanceof Error ? error.message : String(error),
    };
  }
}

/**
 * `cheaper auth login devin` の本体。Devin CLI 自身の login flow
 * を isolated env の child process としてそのまま実行し、
 * 生成された credentials.toml（cheaper data dir 内）を読む。
 */
export async function loginWithDevinCli(cli: DevinCli, dataDir: string): Promise<DevinCredentials> {
  const existing = readDevinCredentials(dataDir);
  if (existing !== null) {
    const status = await devinAuthStatus(cli, dataDir);
    if (status.loggedIn) return existing;
  }

  const { code } = await cli.run(["auth", "login"], { inheritStdio: true });
  const creds = readDevinCredentials(dataDir);
  if (creds === null) {
    throw new Error(
      `\`devin auth login\` ${code === 0 ? "finished" : `exited ${code}`} but ` +
        `${devinCredentialsPath(dataDir)} is missing. ` +
        `Run \`devin auth login\` with XDG_DATA_HOME=${dataDir} yourself, ` +
        `then retry \`cheaper auth login devin\`.`,
    );
  }
  return creds;
}
