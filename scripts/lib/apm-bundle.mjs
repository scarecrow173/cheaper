// Shared assembly of packages/apm/.apm/hooks/cli/ — the self-contained
// cheaper CLI tree shipped inside the APM hook bundle (issue #71).
// Used by scripts/bundle-cli.mjs (canonical `pnpm bundle` entrypoint) and
// scripts/build-apm-bundle.mjs (adds platform executables).
//
// Layout produced:
//   cli/bin/<target>/cheaper[.exe]  native executables (build-apm-bundle only)
//   cli/version.json                {version} derived from Cargo workspace version
//   cli/workers/                    worker definitions + schemas
//   cli/policy.yaml                 built-in policy
//
// Small generated assets are committed. Git/local-path hook consumers acquire
// a pinned executable into user cache; release archives/local builds carry
// ignored platform executables instead of adding them to Git.

import { cpSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";

import { cargoWorkspaceVersion, repoRoot as repoRootDir } from "./release-artifacts.mjs";

export function repoRoot() {
  return repoRootDir;
}

export function apmHooksDir(root = repoRoot()) {
  return path.join(root, "packages", "apm", ".apm", "hooks");
}

// Rebuilds cli/ generated assets. version.json・workers/・policy.yaml
// のみを削除対象にし、bin/<target>/ 配下の platform executable（local
// build/release 用、Git ignore 対象）は保持する。旧 `bin/cheaper.mjs`
// （esbuild JS fallback、廃止済み）も再生成のたびに除去する。
//
// workers/・policy.yaml の複写元は cheaper-cli crate の bundled resource
// source dir — `src/embedded.rs` の `include_dir!`/`include_str!` 対象と
// 同一 path（現状 `packages/cli/`）であり、Rust binary に埋め込まれる
// byte と payload の同梱物が一致することを保証する。
export function buildApmCliTree(root = repoRoot()) {
  const resourceSrc = path.join(root, "packages", "cli");
  const outDir = path.join(apmHooksDir(root), "cli");

  for (const rel of [path.join("bin", "cheaper.mjs"), "version.json", "workers", "policy.yaml"]) {
    rmSync(path.join(outDir, rel), { recursive: true, force: true });
  }

  // version metadata は Cargo.toml `[workspace.package].version`（canonical
  // source）から生成する。binary の埋込 `version.json`（build.rs が
  // `CARGO_PKG_VERSION` から生成）と同じ宣言に由来する。
  writeFileSync(
    path.join(outDir, "version.json"),
    `${JSON.stringify({ version: cargoWorkspaceVersion(root) }, null, 2)}\n`,
  );
  cpSync(path.join(resourceSrc, "workers"), path.join(outDir, "workers"), {
    recursive: true,
  });
  cpSync(path.join(resourceSrc, "policy.yaml"), path.join(outDir, "policy.yaml"));

  return { outDir };
}
