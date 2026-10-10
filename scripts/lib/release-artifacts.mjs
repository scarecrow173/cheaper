// Version declaration helpers shared by check-version.mjs /
// set-version.mjs / check-apm-payload.mjs。release artifact・checksum・
// bundle helper は issue #85(cargo source install + slim APM payload)で
// 撤去済み — executable の staging/acquisition は存在しない。
import { readFileSync } from "node:fs";
import path from "node:path";

// Phase 6: the root Cargo.toml `[workspace.package].version` is the
// canonical product version declaration (crates/cheaper-cli inherits it
// via `version.workspace = true`, and build.rs embeds it into the
// binary). No TOML library — the declaration is located inside the
// [workspace.package] section, which ends at the next section header or
// end of file.
const WORKSPACE_PACKAGE_SECTION =
  /(?:^|\n)\[workspace\.package\][ \t]*\r?\n([\s\S]*?)(?=\n\[|\n?$)/;
// `\s` の代わりに `[ \t]` を使い、version 行の次の行（例: 続く `#` comment
// 行）が match に巻き込まれて setCargoWorkspaceVersion の rewrite で消える
// ことを防ぐ。`\r?` で CRLF 行末を許容する。
const WORKSPACE_VERSION_LINE = /^version[ \t]*=[ \t]*"([^"\r\n]+)"(?:[ \t]*#[^\r\n]*)?[ \t]*\r?$/m;

export function cargoWorkspaceVersion(root) {
  const section =
    WORKSPACE_PACKAGE_SECTION.exec(readFileSync(path.join(root, "Cargo.toml"), "utf8"))?.[1] ?? "";
  const match = WORKSPACE_VERSION_LINE.exec(section);
  if (!match) throw new Error("no version declaration in Cargo.toml [workspace.package]");
  return match[1];
}

export function setCargoWorkspaceVersion(root, version) {
  const file = path.join(root, "Cargo.toml");
  const text = readFileSync(file, "utf8");
  const match = WORKSPACE_PACKAGE_SECTION.exec(text);
  const section = match?.[1];
  if (section === undefined || !WORKSPACE_VERSION_LINE.test(section))
    throw new Error("no version declaration in Cargo.toml [workspace.package]");
  // match.index で splice する。`text.replace(section, …)` は index 0 から
  // 同一文字列を探して前方の出現を書き換え得る上、section 内の `$&` 等が
  // replacement pattern として展開されて壊れる。inner replace も function
  // replacement にして version 文字列を literal に扱う。
  const bodyStart = match.index + match[0].length - section.length;
  const body = section.replace(WORKSPACE_VERSION_LINE, () => `version = "${version}"`);
  writeFileSync(file, text.slice(0, bodyStart) + body + text.slice(bodyStart + section.length));
}

// apm.yml の product version は `version:` 行の一意抽出で検査する。
// check-version.mjs もこの extractor を共有する。
const APM_VERSION_LINE = /^version:\s*(?:"([^"\r\n]+)"|'([^'\r\n]+)'|([^\s#]+))\s*(?:#.*)?$/gm;

export function apmPackageVersion(file) {
  const declarations = [...readFileSync(file, "utf8").matchAll(APM_VERSION_LINE)];
  if (declarations.length !== 1)
    throw new Error(`expected exactly one version: declaration in ${file}`);
  return declarations[0][1] ?? declarations[0][2] ?? declarations[0][3];
}
