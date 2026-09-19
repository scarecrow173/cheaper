/** "<provider>/<id>" の canonical key。 */
export function modelKey(provider: string, id: string): string {
  return `${provider}/${id}`;
}

/**
 * "<provider>/<id>" exact と "<provider>/*" wildcard を解釈する
 * model 参照 pattern 一致。exclude・preferences・exceptions で共有する。
 */
export function matchModelPattern(provider: string, id: string, pattern: string): boolean {
  if (pattern.endsWith("/*")) return provider === pattern.slice(0, -2);
  return pattern === modelKey(provider, id);
}

/**
 * "<provider>/<id>" の直接指定を parse する。model id 自体に "/" を
 * 含みうるため最初の "/" でのみ分割する。形式不正なら `undefined`。
 */
export function parseModelRef(ref: string): { provider: string; id: string } | undefined {
  const i = ref.indexOf("/");
  if (i <= 0 || i === ref.length - 1) return undefined;
  return { provider: ref.slice(0, i), id: ref.slice(i + 1) };
}
