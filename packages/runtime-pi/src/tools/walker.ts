import type { Dirent, Stats } from "node:fs";
import { type FileHandle, open, readdir, realpath, stat } from "node:fs/promises";
import path from "node:path";
import {
  isInsideWorkspaceRoot,
  type PathDenylist,
  resolveWorkspacePath,
  type SymlinkPolicy,
  WorkspaceEscapeError,
} from "@cheaper/core";
import { SNIFF_BYTES } from "./limits.js";

export const ACCESS_DENIED_MESSAGE = "Access denied: path is outside the workspace";
export const POLICY_DENIED_MESSAGE = "Access denied: path is denied by policy";
export const SCOPE_DENIED_MESSAGE = "Access denied: path is outside the write scope";

/** tool 層の escape → error result 変換（operations.ts と同一メッセージ）。 */
export function rethrowAsDenied(e: unknown): never {
  if (e instanceof WorkspaceEscapeError) {
    throw new Error(ACCESS_DENIED_MESSAGE);
  }
  throw e;
}

const DENIED_MESSAGES = new Set([
  ACCESS_DENIED_MESSAGE,
  POLICY_DENIED_MESSAGE,
  SCOPE_DENIED_MESSAGE,
]);

/** boundary/policy/scope 拒否の error result か（rejectedToolCalls 計上判定に使用）。 */
export function isAccessDeniedError(e: unknown): boolean {
  return e instanceof Error && DENIED_MESSAGES.has(e.message);
}

const SKIP_DIRS = new Set([".git", "node_modules"]);

export interface WalkOptions {
  /** canonical 化済み workspace root。 */
  root: string;
  symlinkPolicy: SymlinkPolicy;
  /** privacy deny list。deny file は yield せず deny dir には潜らない。 */
  denylist?: PathDenylist | undefined;
}

export interface WalkedFile {
  /** I/O に使う実 path（symlink 解決済み）。 */
  real: string;
  /** workspace 相対 POSIX 区切り（model 向け表示）。 */
  display: string;
}

/** 列挙失敗の通知。deny 対象は filter 段で消えるためここには載らない（存在秘匿）。 */
export interface WalkIssue {
  /** workspace 相対 POSIX 区切り（model 向け表示）。 */
  display: string;
  reason: "unreadable";
}

export type WalkIssueHandler = (issue: WalkIssue) => void;

function toDisplay(root: string, logicalAbs: string): string {
  const rel = path.relative(root, logicalAbs);
  return rel === "" ? "." : rel.split(path.sep).join("/");
}

/**
 * 先頭 `SNIFF_BYTES` の NUL 走査による text/binary 判定（DESIGN-0019）。
 * bounded 判定で size 判定は持たない。open/read 失敗は `"unreadable"` を返す
 * （fail closed だが理由を区別する）。
 */
export async function sniffTextFile(real: string): Promise<"text" | "binary" | "unreadable"> {
  let fh: FileHandle | undefined;
  try {
    fh = await open(real, "r");
    const buf = Buffer.alloc(SNIFF_BYTES);
    const { bytesRead } = await fh.read(buf, 0, SNIFF_BYTES, 0);
    return buf.subarray(0, bytesRead).includes(0) ? "binary" : "text";
  } catch {
    return "unreadable";
  } finally {
    await fh?.close().catch(() => {});
  }
}

/** win32 の FS 大小文字非依存を boundary 比較に反映する。POSIX では厳密比較。 */
const CASE_INSENSITIVE_FS = process.platform === "win32";

/**
 * 自前 grep/find 共通の sandboxed walker（DESIGN-0002 / DESIGN-0019）。
 * 許可された通常 file を size・内容に関係なく path 列挙する。text eligibility は
 * 呼び出し側が `sniffTextFile` で判定する。`.git`/`node_modules` skip、
 * symlink policy、denylist、祖先チェーンによる cycle 防止を維持し、
 * 列挙失敗（stat/readdir/realpath 失敗・dangling symlink）は `onIssue` に流す。
 * deny 対象は結果にも `onIssue` にも出さない（存在秘匿）。
 * 開始 path は `resolveWorkspacePath` で boundary 判定し、失敗は従来どおり throw する。
 * cycle 防止は祖先チェーン（canonical dir path）のみで判定し、同一実 dir の
 * 別名（symlink）経由の重複結果は許容する。
 */
export async function* walkFiles(
  opts: WalkOptions,
  startAbs: string,
  onIssue?: WalkIssueHandler,
): AsyncGenerator<WalkedFile> {
  const start = await resolveWorkspacePath(opts.root, startAbs, opts.symlinkPolicy);
  if (opts.denylist?.isDenied(start.display)) return;
  yield* walkDir(opts, start.real, start.real, new Set(), onIssue);
}

async function* walkDir(
  opts: WalkOptions,
  fsPath: string,
  logicalPath: string,
  ancestors: Set<string>,
  onIssue: WalkIssueHandler | undefined,
): AsyncGenerator<WalkedFile> {
  const display = () => toDisplay(opts.root, logicalPath);
  let st: Stats | undefined;
  try {
    st = await stat(fsPath);
  } catch {
    onIssue?.({ display: display(), reason: "unreadable" });
    return;
  }
  if (st.isFile()) {
    if (opts.denylist?.isDenied(display())) return;
    yield { real: fsPath, display: display() };
    return;
  }
  if (!st.isDirectory()) return;

  let canonical: string;
  try {
    // 生の realpath を cycle キーに使う。POSIX では大小文字違いは別 dir なので
    // toLowerCase すると誤って同一視して walk 漏れが起きる。
    canonical = await realpath(fsPath);
  } catch {
    onIssue?.({ display: display(), reason: "unreadable" });
    return;
  }
  if (ancestors.has(canonical)) return; // symlink cycle 防止
  ancestors.add(canonical);
  try {
    let entries: Dirent[] | undefined;
    try {
      entries = await readdir(fsPath, { withFileTypes: true });
    } catch {
      onIssue?.({ display: display(), reason: "unreadable" });
      return;
    }
    for (const entry of entries) {
      if (SKIP_DIRS.has(entry.name)) continue;
      const childFs = path.join(fsPath, entry.name);
      const childLogical = path.join(logicalPath, entry.name);
      // deny path（file/dir 両方）は walk 結果にも潜行にも onIssue にも出さない。
      if (opts.denylist?.isDenied(toDisplay(opts.root, childLogical))) continue;
      if (entry.isSymbolicLink()) {
        if (opts.symlinkPolicy === "deny") continue;
        let target: string;
        try {
          target = await realpath(childFs);
        } catch {
          onIssue?.({ display: toDisplay(opts.root, childLogical), reason: "unreadable" });
          continue; // dangling symlink → 列挙失敗として通知
        }
        if (!isInsideWorkspaceRoot(opts.root, target, { caseInsensitive: CASE_INSENSITIVE_FS })) {
          continue;
        }
        // realpath 側を辿り、表示は論理 path（symlink 名）を維持
        yield* walkDir(opts, target, childLogical, ancestors, onIssue);
      } else if (entry.isDirectory()) {
        yield* walkDir(opts, childFs, childLogical, ancestors, onIssue);
      } else if (entry.isFile()) {
        yield { real: childFs, display: toDisplay(opts.root, childLogical) };
      }
    }
  } finally {
    ancestors.delete(canonical);
  }
}
