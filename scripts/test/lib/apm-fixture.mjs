// APM consumer fixture helpers — packages/cli/test/e2e/apm-fixture.ts の
// node:test port(Phase 6 Commit C)。実 `apm install` による consumer deploy
// 検証で共有する。

import { spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readdirSync, symlinkSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const repoRoot = path.resolve(fileURLToPath(new URL("../../..", import.meta.url)));

// 配布用 APM package(apm.yml + .apm/ primitives)の repo 内 path。
// consumer e2e は packages/ 配下を一時 workspace へコピーして install
// する — packages/apm は sibling apm-hook-* への transitive `path:` dep
// を持つため、単体コピーは解決不能(issue #118 / DESIGN-kXMpXYHxw0mt0AE0SeAHTM)。
// repo 本体(development project)は直接参照しない(DESIGN-0021)。
export const apmPackageDir = path.join(repoRoot, "packages", "apm");

export const HOOK_PACKAGE_NAMES = [
  "apm-hook-windsurf",
  "apm-hook-cursor",
  "apm-hook-copilot",
  "apm-hook-claude",
  "apm-hook-codex",
];

// target → その target の hook を配布する package 名。
// agent-skills は hook を持たない。
export const TARGET_HOOK_PACKAGE = {
  windsurf: "apm-hook-windsurf",
  cursor: "apm-hook-cursor",
  copilot: "apm-hook-copilot",
  claude: "apm-hook-claude",
  codex: "apm-hook-codex",
  "agent-skills": null,
};

// target が期待する event key(native のみ — foreign event は登録されない)。
export const TARGET_NATIVE_EVENTS = {
  windsurf: ["pre_read_code", "pre_run_command"],
  cursor: ["beforeReadFile", "beforeShellExecution", "preToolUse"],
  copilot: ["preToolUse"],
  claude: ["PreToolUse"],
  codex: ["PreToolUse"],
};

export function apmAvailable() {
  const r = spawnSync("apm", ["--version"], { shell: true, encoding: "utf8" });
  return r.status === 0;
}

// local path dep として install する実 distribution package の写しを作る。
// packages/ 配下の全 package(親 apm + apm-hook-* sibling)を `pkgs/<name>/`
// へコピーする — 親の transitive `path: ../apm-hook-*` を解決するには
// sibling が pkgs/ 直下に必要(APM は local dep の `source_path` を anchor
// に相対解決する、#857 / issue #118 で 0.28.0/0.30.0 実証済み)。
export function makeConsumerLayout(root, target = "windsurf") {
  const pkgs = path.join(root, "pkgs");
  const consumer = path.join(root, "consumer");
  mkdirSync(consumer, { recursive: true });
  for (const name of ["apm", ...HOOK_PACKAGE_NAMES]) {
    cpSync(path.join(repoRoot, "packages", name), path.join(pkgs, name), {
      recursive: true,
    });
  }
  writeFileSync(
    path.join(consumer, "apm.yml"),
    `name: consumer\nversion: "0.0.1"\ntargets:\n- ${target}\ndependencies:\n  apm:\n  - path: ../pkgs/apm\n`,
  );
  return { pkgs, consumer };
}

export function apmInstall(consumer, home, target = "windsurf") {
  return spawnSync("apm", ["install", "--target", target], {
    cwd: consumer,
    shell: true,
    encoding: "utf8",
    env: { ...process.env, HOME: home, USERPROFILE: home },
  });
}

// target → deploy 済み hook bundle の dir(dep dir 名 = hook package 名)。
// agent-skills は hook deploy 対象外。
const BUNDLE_DIRS = {
  windsurf: [".windsurf", "hooks", "apm-hook-windsurf"],
  claude: [".claude", "hooks", "apm-hook-claude"],
  codex: [".codex", "hooks", "apm-hook-codex"],
  cursor: [".cursor", "hooks", "apm-hook-cursor"],
  copilot: [".github", "hooks", "scripts", "apm-hook-copilot"],
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
    "sh",
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
