// ADR-034zThoekpLqCcHhYO4MPe: 同一 run の公開物を checkout 外の staging に組み立てる。
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  chmodSync,
  closeSync,
  copyFileSync,
  cpSync,
  existsSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  openSync,
  readFileSync,
  readSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const repoRoot = fileURLToPath(new URL("../../", import.meta.url));
export const releaseTargets = Object.freeze([
  Object.freeze({ target: "linux-x64", asset: "cheaper-linux-x64", executable: "cheaper" }),
  Object.freeze({ target: "linux-arm64", asset: "cheaper-linux-arm64", executable: "cheaper" }),
  Object.freeze({ target: "win32-x64", asset: "cheaper-windows-x64.exe", executable: "cheaper.exe" }),
]);

// #73-A launcherのsafe SemVer grammarをそのまま使用する。
const SEMVER = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z]+(?:[.-][0-9A-Za-z]+)*)?(?:\+[0-9A-Za-z]+(?:[.-][0-9A-Za-z]+)*)?$/;
const LFS_HEADER = "version https://git-lfs.github.com/spec/v1";

// Phase 5: the root Cargo.toml `[workspace.package].version` is the product
// binary's version declaration (crates/cheaper-cli inherits it). The
// canonical source stays packages/cli/package.json; this declaration is
// cross-checked for drift. No TOML library — the declaration is located
// inside the [workspace.package] section, which ends at the next section
// header or end of file.
const WORKSPACE_PACKAGE_SECTION = /(?:^|\n)\[workspace\.package\][ \t]*\r?\n([\s\S]*?)(?=\n\[|\n?$)/;
// `\s` の代わりに `[ \t]` を使い、version 行の次の行（例: 続く `#` comment
// 行）が match に巻き込まれて setCargoWorkspaceVersion の rewrite で消える
// ことを防ぐ。`\r?` で CRLF 行末を許容する。
const WORKSPACE_VERSION_LINE = /^version[ \t]*=[ \t]*"([^"\r\n]+)"(?:[ \t]*#[^\r\n]*)?[ \t]*\r?$/m;

export function cargoWorkspaceVersion(root) {
  const section = WORKSPACE_PACKAGE_SECTION.exec(readFileSync(path.join(root, "Cargo.toml"), "utf8"))?.[1] ?? "";
  const match = WORKSPACE_VERSION_LINE.exec(section);
  if (!match) throw new Error("no version declaration in Cargo.toml [workspace.package]");
  return match[1];
}

export function setCargoWorkspaceVersion(root, version) {
  const file = path.join(root, "Cargo.toml");
  const text = readFileSync(file, "utf8");
  const match = WORKSPACE_PACKAGE_SECTION.exec(text);
  const section = match?.[1];
  if (section === undefined || !WORKSPACE_VERSION_LINE.test(section)) throw new Error("no version declaration in Cargo.toml [workspace.package]");
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
  if (declarations.length !== 1) throw new Error(`expected exactly one version: declaration in ${file}`);
  return declarations[0][1] ?? declarations[0][2] ?? declarations[0][3];
}

export function verifyReleaseVersion(root, tag) {
  const version = JSON.parse(readFileSync(path.join(root, "packages/cli/package.json"), "utf8")).version;
  if (typeof version !== "string" || !SEMVER.test(version)) throw new Error("invalid canonical version");
  if (tag !== `v${version}`) throw new Error(`tag ${tag} does not match canonical version v${version}`);
  const bundled = JSON.parse(readFileSync(path.join(root, "packages/apm/.apm/hooks/cli/version.json"), "utf8")).version;
  const apmVersion = apmPackageVersion(path.join(root, "packages/apm/apm.yml"));
  if (bundled !== version || apmVersion !== version) throw new Error("APM / bundled version does not match canonical version");
  if (cargoWorkspaceVersion(root) !== version) throw new Error("Cargo.toml [workspace.package] version does not match canonical version");
  return version;
}

export function parseManifest(bytes) {
  if (bytes.length > 65536) throw new Error("checksum manifest exceeds 64KiB");
  if (bytes.some((byte) => byte > 127 || byte === 13)) throw new Error("manifest must be ASCII and LF only; CR is forbidden");
  if (bytes.length === 0 || bytes.at(-1) !== 10) throw new Error("manifest format requires a final LF");
  const entries = new Map();
  for (const line of bytes.toString("ascii").slice(0, -1).split("\n")) {
    if (line === "") throw new Error("manifest has a blank line");
    const match = /^([0-9A-Fa-f]{64}) {2}([A-Za-z0-9._-]+)$/.exec(line);
    if (!match || match[2] === "." || match[2] === "..") throw new Error("invalid checksum manifest format");
    if (entries.has(match[2])) throw new Error(`duplicate manifest basename ${match[2]}`);
    entries.set(match[2], match[1].toLowerCase());
  }
  for (const { asset } of releaseTargets) {
    if (!entries.has(asset)) throw new Error(`missing manifest entry ${asset}`);
  }
  return entries;
}

function regularFile(file) {
  if (!lstatSync(file).isFile()) throw new Error(`expected a regular file: ${file}`);
}

export function readManifest(file) {
  regularFile(file);
  if (lstatSync(file).size > 65536) throw new Error("checksum manifest exceeds 64KiB");
  const bytes = readFileSync(file);
  return { bytes, entries: parseManifest(bytes) };
}

export function hashExecutable(file) {
  regularFile(file);
  const fd = openSync(file, "r");
  try {
    const buffer = Buffer.alloc(65536);
    const hash = createHash("sha256");
    let first = true;
    for (;;) {
      const length = readSync(fd, buffer, 0, buffer.length, null);
      if (length === 0) {
        if (first) throw new Error(`empty executable: ${file}`);
        break;
      }
      const chunk = buffer.subarray(0, length);
      if (first && chunk.toString("utf8", 0, LFS_HEADER.length).startsWith(LFS_HEADER)) throw new Error(`LFS pointer is not an executable: ${file}`);
      first = false;
      hash.update(chunk);
    }
    return hash.digest("hex");
  } finally {
    closeSync(fd);
  }
}

export function generateManifest(assetDir) {
  const lines = releaseTargets.map(({ asset }) => `${hashExecutable(path.join(assetDir, asset))}  ${asset}\n`);
  const bytes = Buffer.from(lines.join(""), "ascii");
  parseManifest(bytes);
  return bytes;
}

export function verifyAssets(assetDir) {
  const manifest = readManifest(path.join(assetDir, "SHA256SUMS"));
  for (const { asset } of releaseTargets) {
    if (hashExecutable(path.join(assetDir, asset)) !== manifest.entries.get(asset)) throw new Error(`checksum mismatch: ${asset}`);
  }
  return manifest;
}

function contained(parent, child) {
  const relative = path.relative(path.resolve(parent), path.resolve(child));
  return relative === "" || (!relative.startsWith(`..${path.sep}`) && relative !== ".." && !path.isAbsolute(relative));
}

function inputs(root, assetDir, tag) {
  return { root, assetDir, version: verifyReleaseVersion(root, tag), manifest: verifyAssets(assetDir), source: path.join(root, "packages/apm") };
}

function copySnapshot(input, stageDir) {
  if (contained(input.source, stageDir) || contained(stageDir, input.source) || existsSync(stageDir)) throw new Error("stage/output must be new and separate from source package");
  const pkg = path.join(stageDir, "cheaper");
  const sourceBin = path.join(input.source, ".apm/hooks/cli/bin");
  cpSync(input.source, pkg, {
    recursive: true,
    filter: (entry) => !releaseTargets.some(({ target }) => contained(path.join(sourceBin, target), entry)),
  });
  const cli = path.join(pkg, ".apm/hooks/cli");
  for (const { target, asset, executable } of releaseTargets) {
    const dir = path.join(cli, "bin", target);
    mkdirSync(dir, { recursive: true });
    copyFileSync(path.join(input.assetDir, asset), path.join(dir, executable));
    if (target !== "win32-x64") chmodSync(path.join(dir, executable), 0o755);
  }
  writeFileSync(path.join(cli, "SHA256SUMS"), input.manifest.bytes);
  return pkg;
}

export function stagePackage({ root, assetDir, stageDir, tag }) {
  return copySnapshot(inputs(root, assetDir, tag), path.resolve(stageDir));
}

export function assembleBundle({ root, assetDir, outDir, tag, dryRun = false }) {
  const input = inputs(root, assetDir, tag);
  outDir = path.resolve(outDir);
  if (contained(input.source, outDir) || contained(outDir, input.source) || contained(assetDir, outDir) || contained(outDir, assetDir)) throw new Error("output must be separate from source package and raw assets");
  const archive = path.join(outDir, `cheaper-apm-v${input.version}.tar.gz`);
  if (existsSync(archive)) throw new Error(`archive already exists: ${archive}`);
  const result = { version: input.version, archive, manifest: path.join(outDir, "SHA256SUMS") };
  if (dryRun) return result;
  mkdirSync(outDir, { recursive: true });
  const owned = mkdtempSync(path.join(outDir, ".release-stage-"));
  try {
    const stage = path.join(owned, "package");
    copySnapshot(input, stage);
    const tar = spawnSync("tar", ["-czf", archive, "-C", stage, "cheaper"], { encoding: "utf8" });
    if (tar.error || tar.status !== 0) throw new Error(`tar assembly failed: ${tar.error?.message ?? tar.stderr}`);
    writeFileSync(result.manifest, input.manifest.bytes);
    return result;
  } catch (error) {
    rmSync(archive, { force: true });
    throw error;
  } finally {
    rmSync(owned, { recursive: true, force: true });
  }
}

export function parseArgs(argv, values, flags = []) {
  const result = {};
  for (let i = 0; i < argv.length; i += 1) {
    const key = argv[i].startsWith("--") ? argv[i].slice(2) : "";
    if (Object.hasOwn(result, key)) throw new Error(`duplicate argument: --${key}`);
    if (flags.includes(key)) result[key] = true;
    else if (values.includes(key) && argv[i + 1] && !argv[i + 1].startsWith("--")) result[key] = argv[++i];
    else throw new Error(`unknown or incomplete argument: ${argv[i]}`);
  }
  return result;
}
