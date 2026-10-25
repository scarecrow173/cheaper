// Committed-state lockfile contract — issue #85 / ADR-034Dmb3kNAO4Ub6FxuQCtC。
// consumer は `cargo install --git … --locked cheaper-cli` で install する
// ため、committed Cargo.lock が manifests と整合していることが配布契約の
// 一部になる。check pipeline で check-apm-payload の直前に置き、
// 先行する build/test が lockfile を再生成してしまった場合の
// committed-state drift を検出する(working-tree のみの check では
// cargo が書き直した後では no-op になる)。
//
//   1. cargo metadata --locked … committed lockfile が manifest と不整合
//      なら失敗する(lockfile の再解決を要求しない)。
//   2. git diff --exit-code -- Cargo.lock … 先行 step が lockfile を再生成
//      した場合に fail。
import { execFileSync, spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const repoRoot = fileURLToPath(new URL("..", import.meta.url));

// stdout(JSON blob)は捨てる — workspace の metadata は 1MiB を超えて
// spawnSync の default maxBuffer に収まらない。Windows では cargo が
// shim(.bat)なので shell 経由で起動する。
const metadata = spawnSync("cargo metadata --locked --format-version 1", {
  cwd: repoRoot,
  encoding: "utf8",
  shell: true,
  stdio: ["ignore", "ignore", "pipe"],
});
if (metadata.status !== 0) {
  console.error(
    "cargo metadata --locked failed — committed Cargo.lock is inconsistent " +
      `with the workspace manifests:\n${metadata.stderr.trim()}`,
  );
  process.exit(1);
}

try {
  execFileSync("git", ["diff", "--exit-code", "--", "Cargo.lock"], {
    cwd: repoRoot,
    stdio: ["ignore", "pipe", "pipe"],
  });
} catch {
  console.error(
    "Cargo.lock drifted in the working tree — an earlier build step " +
      "regenerated it. Commit the regenerated lockfile so `cargo install " +
      "--locked` resolves the same graph for consumers.",
  );
  process.exit(1);
}

console.log("lockfile check OK: committed Cargo.lock is consistent and undrifted");
