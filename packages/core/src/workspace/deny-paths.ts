/**
 * Workspace 内 path に対する privacy deny list（SPEC-0010 / DESIGN-0015）。
 * credential 系 file（.env, *.pem, id_rsa 等）への worker tool アクセスを
 * path pattern で拒否する。boundary check（workspace 外への脱出防止）とは
 * 独立した第 2 層で、workspace 内の display path にのみ適用される。
 */

export const DEFAULT_DENY_PATHS: readonly string[] = [
  ".env",
  ".env.*",
  "*.pem",
  "*.key",
  "*.p12",
  "*.pfx",
  "*.keystore",
  "id_rsa",
  "id_ed25519",
  "id_ecdsa",
  "id_dsa",
  ".netrc",
  ".npmrc",
  ".pypirc",
  "credentials.json",
  "secrets.json",
  "secrets.yaml",
  "secrets.yml",
  ".ssh/**",
  ".aws/**",
  ".gnupg/**",
];

export interface PathDenylist {
  /** workspace 相対 POSIX display path が deny 対象か。 */
  isDenied(displayPath: string): boolean;
  /** 有効 pattern 列（built-in + policy）。 */
  readonly patterns: readonly string[];
}

function escapeRegExp(s: string): string {
  return s.replace(/[.+^${}()|[\]\\]/g, "\\$&");
}

function segmentToRegex(seg: string): string {
  let out = "";
  for (const ch of seg) {
    if (ch === "*") out += "[^/]*";
    else if (ch === "?") out += "[^/]";
    else out += escapeRegExp(ch);
  }
  return out;
}

// pattern を display path（workspace 相対 POSIX）に対する regex へ変換する。
// - `/` なし: basename 相当として任意深さに match
// - `/` あり・先頭 `/` なし: 任意深さに match（implicit globstar prefix）
// - 先頭 `/`: workspace root anchored
// - `**` segment: 0 個以上の segment
// - 末尾の globstar（例: `.ssh` 配下全て）は dir 自体にも match
export function denyPatternToRegex(pattern: string): RegExp {
  let anchored = false;
  let p = pattern;
  if (p.startsWith("/")) {
    anchored = true;
    p = p.slice(1);
  }
  let dirItself = false;
  if (p === "**" || p.endsWith("/**")) {
    dirItself = true;
    p = p === "**" ? "" : p.slice(0, -3);
  }
  const segs = p.split("/").filter((s) => s.length > 0);
  const body = segs
    .map((s) => (s === "**" ? "(?:[^/]+/)*" : `${segmentToRegex(s)}/`))
    .join("")
    .replace(/\/$/, "");
  const tail = dirItself ? "(?:/.*)?" : "";
  const prefix = anchored ? "^" : "^(?:.*/)?";
  // Windows の filesystem と同じ大小文字非依存比較で casing による deny 回避を防ぐ。
  return new RegExp(`${prefix}${body}${tail}$`, process.platform === "win32" ? "i" : "");
}

/**
 * pattern 列から denylist を構築する。`extra` は built-in の後に連結され、
 * built-in を解除する手段は存在しない（privacy floor）。
 */
export function createPathDenylist(extra: readonly string[] = []): PathDenylist {
  const patterns = [...DEFAULT_DENY_PATHS, ...extra];
  const regexes = patterns.map(denyPatternToRegex);
  return {
    patterns,
    isDenied(displayPath) {
      const p = displayPath.startsWith("./") ? displayPath.slice(2) : displayPath;
      return regexes.some((re) => re.test(p));
    },
  };
}
