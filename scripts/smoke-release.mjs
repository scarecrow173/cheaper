// 同run archiveをAPM consumerへdeployして、Node-free launcherをnative実行する。
import { spawnSync } from "node:child_process";
import { cpSync, existsSync, lstatSync, mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { hashExecutable, parseArgs, readManifest, releaseTargets } from "./lib/release-artifacts.mjs";

export function assertSmokeResult(result, command, version) {
  if (result.error || result.status !== 0) throw new Error(`${command} smoke failed: exit ${result.status}: ${result.error?.message ?? result.stderr ?? ""}`);
  if (command === "--version" && result.stdout.trim() !== version) throw new Error("smoke version mismatch");
  if (command === "models" && JSON.parse(result.stdout).ok !== true) throw new Error("models smoke did not return ok:true");
}

export function verifyBundle(cli, manifestFile, version) {
  const manifest = readManifest(manifestFile);
  const internal = readManifest(path.join(cli, "SHA256SUMS"));
  if (!manifest.bytes.equals(internal.bytes)) throw new Error("internal/external checksum manifests differ");
  if (JSON.parse(readFileSync(path.join(cli, "version.json"), "utf8")).version !== version) throw new Error("bundle version mismatch");
  for (const { target, executable, asset } of releaseTargets) {
    if (hashExecutable(path.join(cli, "bin", target, executable)) !== manifest.entries.get(asset)) throw new Error(`bundle checksum mismatch: ${asset}`);
  }
}

function checked(command, args, options) {
  const result = spawnSync(command, args, { encoding: "utf8", timeout: 120000, ...options });
  if (result.error || result.status !== 0) throw new Error(`${command} failed: ${result.error?.message ?? result.stderr ?? result.stdout}`);
  return result;
}

const regularFile = (file) => existsSync(file) && lstatSync(file).isFile();

export function smokeBundle({ packageDir, manifestFile, version, dryRun = false }) {
  verifyBundle(path.join(packageDir, ".apm/hooks/cli"), manifestFile, version);
  const platformTarget = `${process.platform}-${process.arch}`;
  const native = releaseTargets.find(({ target }) => target === platformTarget);
  // sibling-only target(darwin-arm64 等): Release/manifest 非対象で launcher は
  // acquisition を拒否するため、deploy される sibling を検証する。manifest の
  // 対象外なので package 内の hash を基準に deploy 後の同一性を照合し、
  // 実行検証は末尾の launcher `--version`/`models` smoke が担う。
  const siblingName = process.platform === "win32" ? "cheaper.exe" : "cheaper";
  const sibling = native ? undefined : path.join(packageDir, ".apm/hooks/cli/bin", platformTarget, siblingName);
  if (sibling && !regularFile(sibling)) throw new Error(`sibling-only smoke target ${platformTarget} requires ${sibling}`);
  const siblingHash = sibling ? hashExecutable(sibling) : undefined;
  if (dryRun) return { target: platformTarget, version, verified: 3, dryRun: true };
  const owned = mkdtempSync(path.join(tmpdir(), "cheaper-release-smoke-"));
  try {
    const pkg = path.join(owned, "pkg");
    const consumer = path.join(owned, "consumer");
    const home = path.join(owned, "home");
    mkdirSync(consumer);
    mkdirSync(home);
    cpSync(packageDir, pkg, { recursive: true });
    writeFileSync(path.join(consumer, "apm.yml"), 'name: consumer\nversion: "0.0.1"\ntargets:\n- windsurf\ndependencies:\n  apm:\n  - path: ../pkg\n');
    const env = { ...process.env, HOME: home, USERPROFILE: home, CHEAPER_CONFIG_DIR: path.join(home, "config"), CHEAPER_CACHE_DIR: path.join(home, "cache") };
    checked("apm", ["install", "--target", "windsurf"], { cwd: consumer, env, shell: process.platform === "win32" });
    const deployed = path.join(consumer, ".windsurf/hooks/pkg");
    // deploy後にも全target hashを照合。launcherが取得fallbackへ進む欠損も拒否する。
    verifyBundle(path.join(deployed, "cli"), manifestFile, version);
    if (sibling) {
      const deployedSibling = path.join(deployed, "cli/bin", platformTarget, siblingName);
      if (!regularFile(deployedSibling)) throw new Error(`APM deploy dropped sibling-only executable for ${platformTarget}`);
      if (hashExecutable(deployedSibling) !== siblingHash) throw new Error(`sibling-only executable checksum changed by APM deploy for ${platformTarget}`);
    }
    let executable;
    let prefix;
    if (process.platform === "win32") {
      const system = path.join(process.env.SystemRoot ?? "C:\\Windows", "System32");
      executable = path.join(system, "WindowsPowerShell/v1.0/powershell.exe");
      prefix = ["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File", path.join(deployed, "cheaper.ps1")];
      for (const key of Object.keys(env)) if (key.toLowerCase() === "path") delete env[key];
      env.PATH = system;
    } else {
      const bin = path.join(owned, "system-bin");
      mkdirSync(bin);
      // /usr/binにはNodeがあるため、必要なOS utilityだけのPATHを組み立てる。
      for (const tool of ["dirname", "uname", "head", "grep"]) symlinkSync(`/usr/bin/${tool}`, path.join(bin, tool));
      env.PATH = bin;
      executable = "/bin/sh";
      prefix = [path.join(deployed, "cheaper")];
    }
    const node = spawnSync("node", ["--version"], { cwd: consumer, env, encoding: "utf8" });
    if (!node.error || node.error.code !== "ENOENT") throw new Error("Node is available on smoke PATH");
    for (const command of ["--version", "models"]) {
      assertSmokeResult(spawnSync(executable, [...prefix, command], { cwd: consumer, env, encoding: "utf8", timeout: 30000 }), command, version);
    }
    return { target: platformTarget, version, verified: 3, nodeFree: true };
  } finally {
    rmSync(owned, { recursive: true, force: true });
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  try {
    const args = parseArgs(process.argv.slice(2), ["package-dir", "manifest", "version"], ["dry-run"]);
    if (!args["package-dir"] || !args.manifest || !args.version) throw new Error("required: --package-dir --manifest --version");
    console.log(JSON.stringify(smokeBundle({ packageDir: path.resolve(args["package-dir"]), manifestFile: path.resolve(args.manifest), version: args.version, dryRun: args["dry-run"] })));
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
