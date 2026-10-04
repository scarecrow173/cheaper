import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  chmodSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it } from "vitest";
import { apmAvailable, apmInstall, bundleRoot, makeConsumerLayout } from "./apm-fixture.js";
import { buildNativeExecutable, nodeFreePath } from "./native-executable-fixture.js";

// APM install 後の deployed executable 経路を Node.js 非依存環境で検証する
// （issue #71 acceptance: "supported target で cheaper executable を実行できる"）。
// PATH から toolchain（mise/node）を外した最小 env で、
// launcher → SEA executable → hook 判定・builtin resource 解決まで通す。
// `node cli/bin/cheaper.mjs` の直接呼び出しでは検出できない
// payload 欠落・launcher 破綻を捉えるのが目的。

const BIG_TEXT = "lorem ipsum dolor sit amet ".repeat(4000);

// hook/executable が必要とするだけの最小 PATH。toolchain の node は
// 含まれない（linux CI では /usr/bin 系に uname/head/grep が揃う）。
// Windows も PowerShell launcher を通し、System32 のみを PATH に残す。

const HOOK_HOST_TARGETS: Record<string, string> = {
  "win32-x64": "cheaper.exe",
  "linux-x64": "cheaper",
  "linux-arm64": "cheaper",
};

describe("deployed executable runs without Node.js on PATH", () => {
  let root: string;
  let consumer: string;
  let native: ReturnType<typeof buildNativeExecutable>;

  beforeAll(() => {
    if (apmAvailable() && `${process.platform}-${process.arch}` in HOOK_HOST_TARGETS) {
      native = buildNativeExecutable();
    }
  }, 180_000);
  afterAll(() => native?.cleanup());

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
      expect(existsSync(path.join(bin, "cli", "bin", hostTarget))).toBe(false);
      const actualVersion = JSON.parse(readFileSync(path.join(bin, "cli/version.json"), "utf8"))
        .version as string;
      const bytes = readFileSync(native.executable);
      const cache = path.join(root, "cache");
      const key = path.join(cache, actualVersion, hostTarget);
      mkdirSync(key, { recursive: true });
      const cached = path.join(key, HOOK_HOST_TARGETS[hostTarget]);
      writeFileSync(cached, bytes);
      chmodSync(cached, 0o755);
      writeFileSync(
        path.join(key, "SHA256SUMS"),
        `${createHash("sha256").update(bytes).digest("hex")}  ${native.asset}\n`,
      );

      const env = {
        ...process.env,
        PATH: nodeFreePath(root),
        HOME: root,
        USERPROFILE: root,
        CHEAPER_CONFIG_DIR: path.join(root, "cheaper-config"),
        CHEAPER_CACHE_DIR: cache,
        CHEAPER_RELEASE_BASE_URL: "http://127.0.0.1:1",
        ...(process.platform === "win32"
          ? {
              PSModulePath: path.join(
                process.env.WINDIR ?? "C:/Windows",
                "System32/WindowsPowerShell/v1.0/Modules",
              ),
            }
          : {}),
      };
      // Vitest の uppercase env と Windows の case-insensitive env が衝突しないよう揃える。
      if (process.platform === "win32") {
        for (const key of Object.keys(env)) {
          if (/^psmodulepath$/i.test(key) && key !== "PSModulePath") delete env[key];
        }
      }
      expect(spawnSync("node", ["--version"], { env }).error?.code).toBe("ENOENT");

      // 両 OS とも deployed launcher 経由で verified cache を使う。
      const entry =
        process.platform === "win32"
          ? path.join(
              process.env.WINDIR ?? "C:/Windows",
              "System32/WindowsPowerShell/v1.0/powershell.exe",
            )
          : path.join(bin, "cheaper");
      const prefix =
        process.platform === "win32" ? ["-NoProfile", "-File", path.join(bin, "cheaper.ps1")] : [];

      const version = spawnSync(entry, [...prefix, "--version"], { env, encoding: "utf8" });
      expect(version.status, version.stderr).toBe(0);
      expect(version.stdout.trim()).toBe(actualVersion);

      // builtin resource（model catalog）解決も Node なしで通る
      const models = spawnSync(entry, [...prefix, "models"], { env, encoding: "utf8" });
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
      const hook = spawnSync(entry, [...prefix, "hook", "pre-read-code", "--target", "windsurf"], {
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
