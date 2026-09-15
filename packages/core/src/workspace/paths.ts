import { lstat, realpath, stat } from "node:fs/promises";
import path from "node:path";
import { CheaperError } from "../errors.js";

export type SymlinkPolicy = "deny" | "within-workspace";

export interface ResolvedWorkspacePath {
  /** canonical 絶対パス（不存在 tail は最深存在祖先の realpath + 残り component）。 */
  real: string;
  /** workspace 相対（POSIX 区切り）。 */
  display: string;
}

/**
 * workspace escape を表す専用 Error。tool 層が catch して tool error result に変換するため、
 * CheaperError ではなく `input` を保持する plain Error とする。
 */
export class WorkspaceEscapeError extends Error {
  readonly input: string;

  constructor(input: string, reason: string) {
    super(`workspace escape: ${reason}`);
    this.name = "WorkspaceEscapeError";
    this.input = input;
  }
}

/**
 * workspace root の canonical 化（DESIGN-0002）。
 * 存在確認（不存在 → INVALID_REQUEST）→ fs.realpath。
 */
export async function canonicalizeWorkspaceRoot(cwd: string): Promise<string> {
  try {
    await stat(cwd);
  } catch (e) {
    throw new CheaperError("INVALID_REQUEST", `workspace root does not exist: ${cwd}`, {
      details: { cwd },
      cause: e,
    });
  }
  return realpath(cwd);
}

/** win32 の大小文字非依存を含む portable な root 配下判定（比較を toLowerCase で行う）。 */
function isInsideRoot(root: string, candidate: string): boolean {
  const r = root.toLowerCase();
  const c = candidate.toLowerCase();
  return c === r || c.startsWith(r + path.sep);
}

function failEscape(input: string, reason: string): never {
  throw new WorkspaceEscapeError(input, reason);
}

async function lstatOrNull(p: string) {
  try {
    return await lstat(p);
  } catch {
    return null;
  }
}

/**
 * DESIGN-0002 の resolveWorkspacePath アルゴリズム 1–5。
 * `~` 展開は行わない（Pi 側の resolveToCwd が行う。ここに届くのは絶対パスか workspace 相対）。
 */
export async function resolveWorkspacePath(
  root: string,
  input: string,
  symlinkPolicy: SymlinkPolicy,
): Promise<ResolvedWorkspacePath> {
  // 1. NUL 拒否
  if (input.includes("\0")) failEscape(input, "nul-character");

  // 2. resolve（絶対 input もそのまま扱う）
  const abs = path.resolve(root, input);

  // 3. root 配下かどうか（大小文字非依存比較で win32 を吸収）
  if (!isInsideRoot(root, abs)) failEscape(input, "outside-workspace");

  // 4. root から component 順に辿る。symlink は policy で判定。
  const rel = path.relative(root, abs);
  const components = rel === "" ? [] : rel.split(path.sep);
  let current = root;
  let i = 0;
  while (i < components.length) {
    const next = path.join(current, components[i] as string);
    const st = await lstatOrNull(next);
    if (st === null) break; // 不存在 tail は許容（create 用）
    if (st.isSymbolicLink()) {
      if (symlinkPolicy === "deny") failEscape(input, "symlink-denied");
      let target: string;
      try {
        target = await realpath(next);
      } catch {
        // dangling symlink: target の配下確認ができないため fail closed
        failEscape(input, "symlink-unresolvable");
      }
      if (!isInsideRoot(root, target)) failEscape(input, "symlink-outside-workspace");
      current = target; // realpath 側から辿り直す
      i += 1;
      continue;
    }
    current = next;
    i += 1;
  }
  // この時点で current は最深存在祖先（各 symlink は root 配下確認済み、
  // 非 symlink component は root からの相対辿りなので配下が保証される）。
  const real = path.join(current, ...components.slice(i));

  // 5. display は workspace 相対 POSIX 区切り
  const displayRel = path.relative(root, real);
  const display = displayRel === "" ? "." : displayRel.split(path.sep).join("/");
  return { real, display };
}
