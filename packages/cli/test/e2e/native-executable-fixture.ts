import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, rmSync, symlinkSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../../..");

// /usr/bin 自体に node がある runner でも、launcher に必要な utility だけを公開する。
export function nodeFreePath(root: string): string {
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

// 各 test file が独立した SEA を作り、tracked APM bundle は再生成しない。
export function buildNativeExecutable() {
  const target = `${process.platform}-${process.arch}`;
  const assets: Record<string, string> = {
    "win32-x64": "cheaper-windows-x64.exe",
    "linux-x64": "cheaper-linux-x64",
    "linux-arm64": "cheaper-linux-arm64",
  };
  const asset = assets[target];
  if (!asset) throw new Error(`Unsupported native fixture target: ${target}`);
  mkdirSync(path.join(repo, "dist"), { recursive: true });
  const directory = mkdtempSync(path.join(repo, "dist", "native-fixture-"));
  const cleanup = () => rmSync(directory, { recursive: true, force: true });
  const result = spawnSync(
    process.execPath,
    ["scripts/build-executable.mjs", "--target", target, "--out-dir", directory],
    {
      cwd: repo,
      encoding: "utf8",
      timeout: 180_000,
    },
  );
  if (result.status !== 0) {
    cleanup();
    throw new Error(
      `Native fixture build failed: ${result.error?.message ?? ""}\n${result.stdout}\n${result.stderr}`,
    );
  }
  return { executable: path.join(directory, asset), asset, target, cleanup };
}
