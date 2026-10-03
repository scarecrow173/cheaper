import { spawnSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { apmAvailable, apmInstall, bundleRoot, makeConsumerLayout } from "./apm-fixture.js";

// APM install 後の deployed executable 経路を Node.js 非依存環境で検証する
// （issue #71 acceptance: "supported target で cheaper executable を実行できる"）。
// PATH から toolchain（mise/node）を外した最小 env で、
// launcher → SEA executable → hook 判定・builtin resource 解決まで通す。
// `node cli/bin/cheaper.mjs` の直接呼び出しでは検出できない
// payload 欠落・launcher 破綻を捉えるのが目的。

const BIG_TEXT = "lorem ipsum dolor sit amet ".repeat(4000);

// hook/executable が必要とするだけの最小 PATH。toolchain の node は
// 含まれない（linux CI では /usr/bin 系に uname/head/grep が揃う）。
// win32 では exe を直接 spawn するため PATH はほぼ不要だが、
// System32 を残して子 process 起動を安定させる。
function minimalPath(): string {
  return process.platform === "win32" ? "C:\\Windows\\System32" : "/usr/bin:/bin";
}

const HOOK_HOST_TARGETS: Record<string, string> = {
  "win32-x64": "cheaper.exe",
  "linux-x64": "cheaper",
  "linux-arm64": "cheaper",
};

describe("deployed executable runs without Node.js on PATH", () => {
  let root: string;
  let consumer: string;

  beforeEach(() => {
    if (!apmAvailable()) return;
    root = mkdtempSync(path.join(tmpdir(), "cheaper-apm-exe-"));
    ({ consumer } = makeConsumerLayout(root, "windsurf"));
  });
  afterEach(() => {
    if (root) rmSync(root, { recursive: true, force: true });
  });

  const hostTarget = `${process.platform}-${process.arch}`;
  const hasHostBinary = hostTarget in HOOK_HOST_TARGETS;

  it.runIf(hasHostBinary)(
    "deployed launcher/exe answers --version, models, and a hook deny without node",
    (t) => {
      if (!apmAvailable()) {
        t.skip("apm CLI not available on PATH");
        return;
      }
      const install = apmInstall(consumer, root, "windsurf");
      expect(install.status, install.stderr).toBe(0);

      const bundle = bundleRoot(consumer, "windsurf");
      expect(bundle).not.toBeNull();
      const bin = bundle as string;

      const env = {
        PATH: minimalPath(),
        HOME: root,
        USERPROFILE: root,
        CHEAPER_CONFIG_DIR: path.join(root, "cheaper-config"),
      };

      // POSIX では deployed launcher（#!/bin/sh）経由、win32 では
      // hooks.json の powershell entry と同じ exe 直呼び経路を使う。
      const entry =
        process.platform === "win32"
          ? path.join(bin, "cli", "bin", "win32-x64", "cheaper.exe")
          : path.join(bin, "cheaper");

      const version = spawnSync(entry, ["--version"], { env, encoding: "utf8" });
      expect(version.status, version.stderr).toBe(0);
      expect(version.stdout.trim()).toMatch(/^\d+\.\d+\.\d+/);

      // builtin resource（model catalog）解決も Node なしで通る
      const models = spawnSync(entry, ["models"], { env, encoding: "utf8" });
      expect(models.status, models.stderr).toBe(0);
      expect(models.stdout).toContain('"ok":true');

      // hook 判定経路: workspace 内の閾値超過 file で deny。
      // 判定が動いたことを stderr 文言で確認する（exit 2 だけだと
      // launcher 失敗と区別がつかない）。
      writeFileSync(path.join(consumer, "big.txt"), BIG_TEXT, "utf8");
      const payload = JSON.stringify({
        agent_action_name: "pre_read_code",
        tool_info: { file_path: "big.txt" },
      });
      const hook = spawnSync(entry, ["hook", "pre-read-code", "--target", "windsurf"], {
        cwd: consumer,
        env,
        input: payload,
        encoding: "utf8",
      });
      expect(hook.status, hook.stdout + hook.stderr).toBe(2);
      expect(hook.stderr).toContain("bulk-read");
    },
    120_000,
  );

  it.runIf(!hasHostBinary)(
    "host platform has no payload executable — covered by other targets",
    () => {
      // darwin-* 等 payload 未同梱の host ではこの describe をスキップ。
      expect(true).toBe(true);
    },
  );
});
