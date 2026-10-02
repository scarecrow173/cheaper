import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  canonicalizeWorkspaceRoot,
  countFileTokens,
  type FileTokenCount,
  loadPolicy,
  resolveConfigDirs,
} from "@cheaper/core";

/** built-in resources の既定 root（このパッケージ root）。 */
export const defaultBuiltinDir = fileURLToPath(new URL("..", import.meta.url));

/**
 * `CHEAPER_COUNT_MAX_BYTES` の解釈（DESIGN-0017）。
 * 正の整数のみ有効: trim 後が 10 進数字列のみで構成され、
 * `Number.isSafeInteger` かつ `n > 0` を満たす値だけを受け付ける。
 * `parseInt` は使わない（`16MB` → 16、`1.5` → 1 のような部分 parse を
 * 許容しないため）。不正値・未設定は core の default に任せるため
 * `undefined` を返す。
 */
export function countMaxBytes(env: NodeJS.ProcessEnv): number | undefined {
  const raw = env.CHEAPER_COUNT_MAX_BYTES;
  if (raw === undefined) return undefined;
  const trimmed = raw.trim();
  if (!/^\d+$/.test(trimmed)) return undefined;
  const n = Number(trimmed);
  return Number.isSafeInteger(n) && n > 0 ? n : undefined;
}

export interface WorkspaceFileCount {
  count: FileTokenCount;
  /** canonicalize 済み workspace root。 */
  root: string;
  userConfigDir: string;
}

/**
 * `count-tokens` command と `cheaper hook` routing policy が共有する
 * workspace file の token 計測経路（DESIGN-0012）。
 * canonicalize workspace root → config dirs → policy（symlinks のみ使用）
 * → `countFileTokens`。error は呼び出し側が context に応じて
 * CheaperError（CLI envelope）または routing verdict（hook）へ変換する。
 */
export async function countWorkspaceFileTokens(opts: {
  cwd: string;
  path: string;
  builtinDir: string;
  env: NodeJS.ProcessEnv;
}): Promise<WorkspaceFileCount> {
  const root = await canonicalizeWorkspaceRoot(opts.cwd);
  const dirs = resolveConfigDirs({ cwd: root, builtinDir: opts.builtinDir, env: opts.env });
  const policy = await loadPolicy({
    builtinPolicyPath: path.join(opts.builtinDir, "policy.yaml"),
    userConfigDir: dirs.userConfigDir,
    projectDir: dirs.projectDir,
  });
  const maxBytes = countMaxBytes(opts.env);
  const count = await countFileTokens({
    root,
    path: opts.path,
    symlinkPolicy: policy.filesystem.symlinks,
    ...(maxBytes !== undefined ? { maxBytes } : {}),
  });
  return { count, root, userConfigDir: dirs.userConfigDir };
}
