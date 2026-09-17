import { spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import { cliRoot } from "./helpers.js";

const repoRoot = path.join(cliRoot, "..", "..");

export function apmAvailable(): boolean {
  const r = spawnSync("apm", ["--version"], { shell: true, encoding: "utf8" });
  return r.status === 0;
}

// local path dep として install できる最小 package（repo の .apm/ + 無依存
// apm.yml）を作る。repo 本体は transitive remote deps と node_modules を
// 含むため、そのままでは local dep に使えない（DESIGN-0014 / DISC-0006）。
export function makeConsumerLayout(
  root: string,
  target = "windsurf",
): { pkg: string; consumer: string } {
  const pkg = path.join(root, "pkg");
  const consumer = path.join(root, "consumer");
  mkdirSync(pkg, { recursive: true });
  mkdirSync(consumer, { recursive: true });
  cpSync(path.join(repoRoot, ".apm"), path.join(pkg, ".apm"), { recursive: true });
  writeFileSync(path.join(pkg, "apm.yml"), 'name: cheaper\nversion: "0.0.0"\n');
  writeFileSync(
    path.join(consumer, "apm.yml"),
    `name: consumer\nversion: "0.0.1"\ntargets:\n- ${target}\ndependencies:\n  apm:\n  - path: ../pkg\n`,
  );
  return { pkg, consumer };
}

export function apmInstall(consumer: string, home: string, target = "windsurf") {
  return spawnSync("apm", ["install", "--target", target], {
    cwd: consumer,
    shell: true,
    encoding: "utf8",
    env: { ...process.env, HOME: home, USERPROFILE: home },
  });
}

// target → deploy 済み hook bundle の dir（dep dir 名 = "pkg"）。
// agent-skills は hook deploy 対象外。
const BUNDLE_DIRS: Record<string, string[] | null> = {
  windsurf: [".windsurf", "hooks", "pkg"],
  claude: [".claude", "hooks", "pkg"],
  codex: [".codex", "hooks", "pkg"],
  cursor: [".cursor", "hooks", "pkg"],
  copilot: [".github", "hooks", "scripts", "pkg"],
  "agent-skills": null,
};

export function bundleRoot(consumer: string, target = "windsurf"): string | null {
  const segs = BUNDLE_DIRS[target];
  return segs ? path.join(consumer, ...segs) : null;
}

export function bundledCli(consumer: string, target = "windsurf"): string | null {
  const root = bundleRoot(consumer, target);
  return root ? path.join(root, "cli", "bin", "cheaper.mjs") : null;
}

// target → 生成される hook config file の consumer 相対 path。
// windsurf / codex / cursor は hooks.json（`hooks` 直下に event を
// nest）に加え、同じ登録の top-level event key shape の
// apm-hooks.json も生成される（apm 0.28.0 実測）。agent-skills は
// hook config を生成しない。
const GENERATED_CONFIGS: Record<string, string[][]> = {
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
 * target が生成する hook config の path 一覧（consumer 配下の絶対 path）。
 * copilot は file 名が dep 名由来のため `.github/hooks/` 配下の `*-hooks.json`
 * を readdirSync で解決し、見つからなければ空配列を返す。
 */
export function generatedConfigPaths(consumer: string, target: string): string[] {
  if (target === "copilot") {
    const dir = path.join(consumer, ".github", "hooks");
    if (!existsSync(dir)) return [];
    return readdirSync(dir)
      .filter((f) => f.endsWith("-hooks.json"))
      .map((f) => path.join(dir, f));
  }
  return (GENERATED_CONFIGS[target] ?? []).map((segs) => path.join(consumer, ...segs));
}
