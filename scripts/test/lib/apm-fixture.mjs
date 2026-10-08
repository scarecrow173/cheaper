// APM consumer fixture helpers — packages/cli/test/e2e/apm-fixture.ts の
// node:test port(Phase 6 Commit C)。実 `apm install` による consumer deploy
// 検証で共有する。

import { spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readdirSync, symlinkSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const repoRoot = path.resolve(fileURLToPath(new URL("../../..", import.meta.url)));

// 配布用 APM package(apm.yml + .apm/ primitives)の repo 内 path。
// consumer e2e はこの package だけを一時 workspace へコピーして install
// する。repo 本体(development project)は直接参照しない(DESIGN-0021)。
export const apmPackageDir = path.join(repoRoot, "packages", "apm");

export function apmAvailable() {
  const r = spawnSync("apm", ["--version"], { shell: true, encoding: "utf8" });
  return r.status === 0;
}

// local path dep として install する実 distribution package の写しを作る。
// packages/apm(apm.yml + .apm/ primitives)をそのままコピーする
// (DESIGN-0014 / DISC-0006 / DESIGN-0021)。
export function makeConsumerLayout(root, target = "windsurf") {
  const pkg = path.join(root, "pkg");
  const consumer = path.join(root, "consumer");
  mkdirSync(pkg, { recursive: true });
  mkdirSync(consumer, { recursive: true });
  cpSync(apmPackageDir, pkg, { recursive: true });
  writeFileSync(
    path.join(consumer, "apm.yml"),
    `name: consumer\nversion: "0.0.1"\ntargets:\n- ${target}\ndependencies:\n  apm:\n  - path: ../pkg\n`,
  );
  return { pkg, consumer };
}

export function apmInstall(consumer, home, target = "windsurf") {
  return spawnSync("apm", ["install", "--target", target], {
    cwd: consumer,
    shell: true,
    encoding: "utf8",
    env: { ...process.env, HOME: home, USERPROFILE: home },
  });
}

// target → deploy 済み hook bundle の dir(dep dir 名 = "pkg")。
// agent-skills は hook deploy 対象外。
const BUNDLE_DIRS = {
  windsurf: [".windsurf", "hooks", "pkg"],
  claude: [".claude", "hooks", "pkg"],
  codex: [".codex", "hooks", "pkg"],
  cursor: [".cursor", "hooks", "pkg"],
  copilot: [".github", "hooks", "scripts", "pkg"],
  "agent-skills": null,
};

export function bundleRoot(consumer, target = "windsurf") {
  const segs = BUNDLE_DIRS[target];
  return segs ? path.join(consumer, ...segs) : null;
}

// target → 生成される hook config file の consumer 相対 path。
// windsurf / codex / cursor は hooks.json(hooks 直下に event を nest)
// に加え、同じ登録の top-level event key shape の apm-hooks.json も
// 生成される(apm 0.28.0 実測)。agent-skills は hook config を生成しない。
const GENERATED_CONFIGS = {
  windsurf: [
    [".windsurf", "hooks.json"],
    [".windsurf", "apm-hooks.json"],
  ],
  claude: [
    [".claude", "apm-hooks.json"],
    [".claude", "settings.json"],
  ],
  codex: [
    [".codex", "hooks.json"],
    [".codex", "apm-hooks.json"],
  ],
  cursor: [
    [".cursor", "hooks.json"],
    [".cursor", "apm-hooks.json"],
  ],
  "agent-skills": [],
};

/**
 * target が生成する hook config の path 一覧(consumer 配下の絶対 path)。
 * copilot は file 名が dep 名由来のため `.github/hooks/` 配下の
 * `*-hooks.json` を readdirSync で解決し、見つからなければ空配列を返す。
 */
export function generatedConfigPaths(consumer, target) {
  if (target === "copilot") {
    const dir = path.join(consumer, ".github", "hooks");
    if (!existsSync(dir)) return [];
    return readdirSync(dir)
      .filter((f) => f.endsWith("-hooks.json"))
      .map((f) => path.join(dir, f));
  }
  return (GENERATED_CONFIGS[target] ?? []).map((segs) => path.join(consumer, ...segs));
}

// /usr/bin 自体に node がある runner でも、launcher に必要な utility だけを
// 公開する PATH を組み立てる(packages/cli/test/e2e/native-executable-
// fixture.ts の nodeFreePath)。
export function nodeFreePath(root) {
  if (process.platform === "win32")
    return path.join(process.env.WINDIR ?? "C:/Windows", "System32");
  const directory = path.join(root, "system-bin");
  mkdirSync(directory, { recursive: true });
  for (const tool of [
    "dirname",
    "uname",
    "head",
    "grep",
    "awk",
    "realpath",
    "rm",
    "mkdir",
    "sleep",
    "sha256sum",
    "shasum",
    "wc",
    "chmod",
    "curl",
    "mktemp",
    "mv",
    "cat",
  ]) {
    const source = [`/usr/bin/${tool}`, `/bin/${tool}`].find(existsSync);
    if (source) symlinkSync(source, path.join(directory, tool));
  }
  return directory;
}

// launcher 経由で取得する実 binary — SEA fixture builder は Phase 6 で撤去
// 済みのため、cargo build 出力(target/release 優先、debug fallback)を使う。
// どちらも無ければ null(binary-dependent test は skip)。
export function cargoBinary() {
  for (const profile of ["release", "debug"]) {
    const p = path.join(
      repoRoot,
      "target",
      profile,
      process.platform === "win32" ? "cheaper.exe" : "cheaper",
    );
    if (existsSync(p)) return p;
  }
  return null;
}
