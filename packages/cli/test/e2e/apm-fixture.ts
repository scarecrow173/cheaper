import { spawnSync } from "node:child_process";
import { cpSync, mkdirSync, writeFileSync } from "node:fs";
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
export function makeConsumerLayout(root: string): { pkg: string; consumer: string } {
  const pkg = path.join(root, "pkg");
  const consumer = path.join(root, "consumer");
  mkdirSync(pkg, { recursive: true });
  mkdirSync(consumer, { recursive: true });
  cpSync(path.join(repoRoot, ".apm"), path.join(pkg, ".apm"), { recursive: true });
  writeFileSync(path.join(pkg, "apm.yml"), 'name: cheaper\nversion: "0.0.0"\n');
  writeFileSync(
    path.join(consumer, "apm.yml"),
    'name: consumer\nversion: "0.0.1"\ntargets:\n- windsurf\ndependencies:\n  apm:\n  - path: ../pkg\n',
  );
  return { pkg, consumer };
}

export function apmInstall(consumer: string, home: string) {
  return spawnSync("apm", ["install", "--target", "windsurf"], {
    cwd: consumer,
    shell: true,
    encoding: "utf8",
    env: { ...process.env, HOME: home, USERPROFILE: home },
  });
}

/** deploy 済み hook bundle の root（dep dir 名 = "pkg"）。 */
export function bundleRoot(consumer: string): string {
  return path.join(consumer, ".windsurf", "hooks", "pkg");
}

export function bundledCli(consumer: string): string {
  return path.join(bundleRoot(consumer), "cli", "bin", "cheaper.mjs");
}
