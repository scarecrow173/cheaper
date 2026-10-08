// Launcher acquisition contract — packages/cli/test/e2e/launcher-acquisition.test.ts
// の node:test port(Phase 6 Commit C)。packages/apm/.apm/hooks/ の
// launcher script(`cheaper`/`cheaper.ps1`)が version 固定 Release の
// SHA256SUMS + asset pair を取得・検証・cache・exec する contract を、
// stub Release HTTP server 越しに検証する。
//
// TS 版の `buildNativeExecutable`(SEA fixture)は撤去済み — 「実 CLI」が
// 必要な test は `cargoBinary()`(target/release → target/debug)を使い、
// cargo build 済みでなければ skip する。

import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  chmodSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import path from "node:path";
import { after, afterEach, before, beforeEach, describe, it } from "node:test";
import {
  apmAvailable,
  apmInstall,
  bundleRoot,
  cargoBinary,
  makeConsumerLayout,
  nodeFreePath,
  repoRoot as repo,
} from "./lib/apm-fixture.mjs";

const hooks = path.join(repo, "packages/apm/.apm/hooks");
const windows = process.platform === "win32";
const platform = windows ? "win32-x64" : `${process.platform}-${process.arch}`;
const filename = windows ? "cheaper.exe" : "cheaper";
const asset = windows ? "cheaper-windows-x64.exe" : `cheaper-${platform}`;
const supportedPlatform = ["win32-x64", "linux-x64", "linux-arm64"].includes(platform);
const sh = windows ? "C:/Program Files/Git/usr/bin/sh.exe" : "/bin/sh";
const psProbe = spawnSync("pwsh", ["-NoProfile", "-Command", "(Get-Process -Id $PID).Path"], {
  encoding: "utf8",
});
const pwshPath = psProbe.stdout?.trim();
const pwsh = windows && psProbe.status === 0;
const runners = [
  { name: "sh", available: existsSync(sh) },
  { name: "pwsh", available: pwsh },
];
const digest = (bytes) => createHash("sha256").update(bytes).digest("hex");

let fixtureRoot;
let binary;

before(() => {
  fixtureRoot = mkdtempSync(path.join(tmpdir(), "cheaper-native-fixture-"));
  if (windows) {
    const compiler = path.join(
      process.env.WINDIR ?? "C:/Windows",
      "Microsoft.NET/Framework64/v4.0.30319/csc.exe",
    );
    const target = path.join(fixtureRoot, filename);
    const compile = spawnSync(
      compiler,
      ["/nologo", `/out:${target}`, path.join(repo, "scripts/test/fixtures/launcher-fixture.cs")],
      { encoding: "utf8" },
    );
    assert.strictEqual(compile.status, 0, compile.stderr + compile.stdout);
    binary = readFileSync(target);
  } else {
    binary = Buffer.from(
      '#!/bin/sh\nif [ "$1" = "--version" ]; then cat >/dev/null; echo 1.0.0; exit 0; fi\nprintf "ARGS:\\n"\nprintf "%s\\n" "$@"\nprintf "INPUT:"\ncat\necho "fixture stderr" >&2\n[ "$1" != hook ] || exit 2\n',
    );
  }
});
after(() => {
  if (fixtureRoot) rmSync(fixtureRoot, { recursive: true, force: true });
});

for (const runner of runners) {
  describe(`${runner.name} version-pinned launcher`, {
    skip: !(runner.available && supportedPlatform),
  }, () => {
    let root;
    let launch;
    let cache;
    let env;
    let server;
    let requests;
    let manifest;
    let body;
    let releaseVersion;
    let behavior;

    beforeEach(async () => {
      root = mkdtempSync(path.join(tmpdir(), "cheaper launch "));
      launch = path.join(root, "hooks");
      cache = path.join(root, "cache");
      mkdirSync(path.join(launch, "cli"), { recursive: true });
      for (const file of ["cheaper", "cheaper.ps1", "hooks.json"]) {
        copyFileSync(path.join(hooks, file), path.join(launch, file));
      }
      writeFileSync(path.join(launch, "cli/version.json"), '{"version":"1.0.0"}\n');
      body = binary;
      releaseVersion = "1.0.0";
      manifest = `${digest(binary)}  ${asset}\n`;
      behavior = "normal";
      requests = [];
      server = createServer((req, res) => {
        requests.push(req.url ?? "");
        if (behavior === "redirect") {
          res.writeHead(302, { location: "http://example.invalid/forbidden" }).end();
          return;
        }
        if (req.url === `/v${releaseVersion}/SHA256SUMS`) {
          if (behavior === "missing") {
            res.writeHead(404).end();
            return;
          }
          res.end(manifest);
        } else if (req.url === `/v${releaseVersion}/${asset}`) {
          if (behavior === "interrupt") {
            res.writeHead(200, { "Content-Length": body.length + 100 });
            res.write(body.subarray(0, 10));
            setTimeout(() => res.destroy(), 20);
          } else {
            setTimeout(() => res.end(body), 100);
          }
        } else {
          res.writeHead(404).end();
        }
      });
      await new Promise((resolve) => server?.listen(0, "127.0.0.1", resolve));
      const address = server.address();
      if (!address || typeof address === "string") throw new Error("No stub port");
      const base = `http://127.0.0.1:${address.port}`;
      env = {
        ...process.env,
        PATH: windows
          ? "C:/Program Files/Git/usr/bin;C:/Windows/System32;C:/Windows/System32/WindowsPowerShell/v1.0"
          : nodeFreePath(root),
        CHEAPER_RELEASE_BASE_URL: base,
        CHEAPER_CACHE_DIR: cache,
        CHEAPER_CONFIG_DIR: path.join(root, "config"),
        HOME: root,
        USERPROFILE: root,
        XDG_CACHE_HOME: path.join(root, "xdg"),
        LOCALAPPDATA: path.join(root, "local"),
        HTTP_PROXY: "",
        HTTPS_PROXY: "",
        ALL_PROXY: "",
        NO_PROXY: "127.0.0.1,localhost",
      };
    });
    afterEach(async () => {
      if (server) {
        server.closeAllConnections();
        await new Promise((resolve) => server?.close(() => resolve()));
        server = undefined;
      }
      rmSync(root, { recursive: true, force: true });
    });

    function run(argv = ["--version"], input = "", extra = {}, command) {
      const executable = runner.name === "sh" ? sh : pwshPath;
      const args =
        runner.name === "sh"
          ? [path.join(launch, "cheaper"), ...argv]
          : command
            ? ["-NoProfile", "-Command", command]
            : ["-NoProfile", "-File", path.join(launch, "cheaper.ps1"), ...argv];
      return new Promise((resolve, reject) => {
        const child = spawn(executable, args, { cwd: launch, env: { ...env, ...extra } });
        let stdout = "";
        let stderr = "";
        const timer = setTimeout(() => {
          child.kill();
          reject(new Error("launcher timed out"));
        }, 30_000);
        child.stdout.on("data", (chunk) => {
          stdout += chunk;
        });
        child.stderr.on("data", (chunk) => {
          stderr += chunk;
        });
        child.on("error", reject);
        child.on("close", (code) => {
          clearTimeout(timer);
          resolve({ code, stdout, stderr });
        });
        child.stdin.on("error", () => {});
        child.stdin.end(input);
      });
    }
    function preseed(bytes = binary, sums = manifest) {
      const key = path.join(cache, "1.0.0", platform);
      mkdirSync(key, { recursive: true });
      writeFileSync(path.join(key, filename), bytes);
      chmodSync(path.join(key, filename), 0o755);
      writeFileSync(path.join(key, "SHA256SUMS"), sums);
      return key;
    }
    function clean(version = "1.0.0") {
      const parent = path.join(cache, version);
      assert.deepStrictEqual(
        existsSync(parent) ? readdirSync(parent).filter((p) => p.startsWith(".")) : [],
        [],
      );
    }
    function failed(result, message) {
      assert.notStrictEqual(result.code, 0, result.stdout + result.stderr);
      assert.strictEqual(result.stdout, "");
      assert.match(result.stderr, message);
      clean();
    }
    function copyDirectoryPair(from, to) {
      const target = path.join(to, platform);
      mkdirSync(target);
      for (const f of [filename, "SHA256SUMS"])
        copyFileSync(path.join(from, platform, f), path.join(target, f));
      chmodSync(path.join(target, filename), 0o755);
    }

    it("uses sibling without consulting version, cache, or network", async () => {
      const dir = path.join(launch, "cli/bin", platform);
      mkdirSync(dir, { recursive: true });
      writeFileSync(path.join(dir, filename), binary);
      chmodSync(path.join(dir, filename), 0o755);
      rmSync(path.join(launch, "cli/version.json"));
      const result = await run();
      assert.strictEqual(result.code, 0, result.stderr);
      assert.strictEqual(result.stdout.trim(), "1.0.0");
      assert.deepStrictEqual(requests, []);
      assert.strictEqual(existsSync(cache), false);
    });
    it("acquires the pinned pair and executes with preserved hook stdin, argv, and exit", async () => {
      const argv = ["hook", "pre-tool-use", "space arg", "", 'quote";$()\\end'];
      const input = '{"tool_name":"Read","path":"space file"}';
      const result = await run(argv, input);
      assert.strictEqual(result.code, 2, result.stderr);
      assert.strictEqual(
        result.stdout.replace(/\r/g, ""),
        `ARGS:\n${argv.join("\n")}\nINPUT:${input}`,
      );
      assert.match(result.stderr, /fixture stderr/);
      assert.deepStrictEqual(
        [...requests].sort(),
        [`/v1.0.0/${asset}`, "/v1.0.0/SHA256SUMS"].sort(),
      );
      assert.deepStrictEqual(
        readdirSync(path.join(cache, "1.0.0", platform)).sort(),
        ["SHA256SUMS", filename].sort(),
      );
      clean();
    });
    it("executes an offline verified preseed without a download", async () => {
      preseed();
      await new Promise((resolve) => server?.close(() => resolve()));
      server = undefined;
      const result = await run();
      assert.strictEqual(result.code, 0, result.stderr);
      assert.strictEqual(result.stdout.trim(), "1.0.0");
      assert.deepStrictEqual(requests, []);
      clean();
    });
    it("diagnoses a cold offline cache with release, proxy and preseed guidance", async () => {
      await new Promise((resolve) => server?.close(() => resolve()));
      server = undefined;
      failed(await run(), /download.*failed/i);
      const result = await run();
      failed(result, /tagged Release\/SHA256SUMS/);
      assert.match(result.stderr, /proxy.*mirror.*offline preseed/i);
    });
    it("downloads only once under concurrent first-run", async () => {
      const results = await Promise.all([run(), run()]);
      for (const r of results) assert.strictEqual(r.code, 0, r.stderr);
      assert.strictEqual(requests.filter((p) => p.endsWith("SHA256SUMS")).length, 1);
      assert.strictEqual(requests.filter((p) => p.endsWith(asset)).length, 1);
      clean();
    });
    for (const failure of [
      "corrupt",
      "interrupted",
      "missing-manifest",
      "bad-manifest",
      "duplicate",
      "wrong-version",
      "broken-executable",
      "redirect",
    ]) {
      it(`fails closed for ${failure} without leaving partial files`, {
        timeout: 35_000,
      }, async () => {
        if (failure === "corrupt") body = Buffer.from("corrupt");
        if (failure === "interrupted") behavior = "interrupt";
        if (failure === "missing-manifest") behavior = "missing";
        if (failure === "bad-manifest") manifest = "invalid checksum\n";
        if (failure === "duplicate") manifest += manifest;
        if (failure === "wrong-version") {
          writeFileSync(path.join(launch, "cli/version.json"), '{"version":"1.0.1"}');
          preseed();
          const oldParent = path.join(cache, "1.0.0");
          const newParent = path.join(cache, "1.0.1");
          mkdirSync(newParent);
          copyDirectoryPair(oldParent, newParent);
        }
        if (failure === "broken-executable") {
          body = Buffer.from("not executable");
          manifest = `${digest(body)}  ${asset}\n`;
        }
        if (failure === "redirect") behavior = "redirect";
        failed(await run(), /cheaper launcher:|version|checksum|download|manifest/i);
        if (failure !== "wrong-version")
          assert.strictEqual(existsSync(path.join(cache, "1.0.0", platform)), false);
      });
    }
    for (const failure of ["binary-only", "manifest-only", "hash-change"]) {
      it(`rejects ${failure} cache without repairing it`, { timeout: 35_000 }, async () => {
        const key = preseed();
        if (failure === "binary-only") rmSync(path.join(key, "SHA256SUMS"));
        if (failure === "manifest-only") rmSync(path.join(key, filename));
        if (failure === "hash-change") writeFileSync(path.join(key, filename), "changed");
        failed(await run(), /manifest|checksum|cache/i);
        assert.deepStrictEqual(requests, []);
      });
    }
    it("reports unwritable cache boundary without downloading", async () => {
      writeFileSync(cache, "regular file");
      failed(await run(), /cache|writ|directory/i);
      assert.deepStrictEqual(requests, []);
    });
    it("does not write a cache under deployed hooks", async () => {
      failed(
        await run(["--version"], "", { CHEAPER_CACHE_DIR: path.join(launch, "bad-cache") }),
        /cache|hook/i,
      );
      assert.strictEqual(existsSync(path.join(launch, "bad-cache")), false);
    });
    it("rejects missing or malformed version metadata before downloading", async () => {
      rmSync(path.join(launch, "cli/version.json"));
      failed(await run(), /version/i);
      writeFileSync(path.join(launch, "cli/version.json"), '{"version":"../latest"}');
      failed(await run(), /version/i);
      assert.deepStrictEqual(requests, []);
    });
    it("rejects LFS pointers rather than acquiring", async () => {
      const dir = path.join(launch, "cli/bin", platform);
      mkdirSync(dir, { recursive: true });
      writeFileSync(
        path.join(dir, filename),
        "version https://git-lfs.github.com/spec/v1\noid sha256:0000\n",
      );
      failed(await run(), /LFS pointer/i);
      assert.deepStrictEqual(requests, []);
    });
    it("rejects Windows ARM64 acquisition", { skip: runner.name !== "pwsh" }, async () => {
      failed(
        await run(["--version"], "", {
          PROCESSOR_ARCHITEW6432: "ARM64",
          PROCESSOR_ARCHITECTURE: "ARM64",
        }),
        /unsupported/i,
      );
      assert.deepStrictEqual(requests, []);
    });
    it("uses Invoke-WebRequest when curl.exe is unavailable", {
      skip: runner.name !== "pwsh",
    }, async () => {
      const result = await run(["--version"], "", {
        PATH: "C:/Windows/System32/WindowsPowerShell/v1.0",
      });
      assert.strictEqual(result.code, 0, result.stderr);
      assert.strictEqual(requests.length, 2);
      clean();
    });
    it("rejects plain HTTP redirects in Invoke-WebRequest fallback", {
      skip: runner.name !== "pwsh",
    }, async () => {
      behavior = "redirect";
      failed(
        await run(["--version"], "", {
          PATH: "C:/Windows/System32/WindowsPowerShell/v1.0",
        }),
        /download/i,
      );
      assert.strictEqual(requests.length, 1);
    });
    it("rejects unsupported acquisition but retains unsupported-host sibling compatibility", {
      skip: runner.name !== "sh",
    }, async () => {
      const tools = path.join(root, "tools");
      mkdirSync(tools);
      writeFileSync(
        path.join(tools, "uname"),
        '#!/bin/sh\ncase "$1" in -s) echo Darwin;; -m) echo x86_64;; esac\n',
      );
      chmodSync(path.join(tools, "uname"), 0o755);
      const extra = { PATH: `${tools}${path.delimiter}${env.PATH}` };
      failed(await run(["--version"], "", extra), /unsupported/i);
      const dir = path.join(launch, "cli/bin/darwin-x64");
      mkdirSync(dir, { recursive: true });
      writeFileSync(path.join(dir, "cheaper"), binary);
      chmodSync(path.join(dir, "cheaper"), 0o755);
      const result = await run(["--version"], "", extra);
      assert.strictEqual(result.code, 0, result.stderr);
      assert.deepStrictEqual(requests, []);
    });
    it("runs declared PowerShell hooks and preserves their fallback policies", {
      skip: runner.name !== "pwsh",
    }, async () => {
      preseed();
      const declared = JSON.parse(readFileSync(path.join(launch, "hooks.json"), "utf8"));
      for (const entries of Object.values(declared.hooks)) {
        for (const entry of entries) {
          if (!entry.powershell) continue;
          assert.match(entry.powershell, /cheaper\.ps1/);
          assert.doesNotMatch(entry.powershell, /cli\/bin/);
        }
      }
      const direct = await run(
        [],
        '{"tool_name":"Read"}',
        {},
        declared.hooks.pre_read_code[0].powershell,
      );
      assert.strictEqual(direct.code, 2, direct.stderr);
      assert.match(direct.stdout, /INPUT:\{"tool_name":"Read"\}/);
      const copilot = await run([], "{}", {}, declared.hooks.preToolUse[0].powershell);
      assert.strictEqual(copilot.code, 0);
      const cursor = await run([], "{}", {}, declared.hooks.beforeReadFile[0].powershell);
      assert.strictEqual(cursor.code, 0);
      assert.match(cursor.stdout, /\{"permission":"allow"\}/);
    });
    it("runs a real CLI hook via cached executable with original stdin and exit 2", {
      timeout: 35_000,
    }, async (t) => {
      const actual = cargoBinary();
      if (!actual) {
        t.skip("cargo binary not found; run `mise exec -- cargo build -p cheaper-cli`");
        return;
      }
      const bytes = readFileSync(actual);
      const versionProbe = spawnSync(actual, ["--version"], {
        encoding: "utf8",
        env,
        timeout: 30_000,
      });
      assert.strictEqual(versionProbe.status, 0, versionProbe.stderr);
      const actualVersion = versionProbe.stdout.trim();
      writeFileSync(
        path.join(launch, "cli/version.json"),
        JSON.stringify({ version: actualVersion }),
      );
      const key = path.join(cache, actualVersion, platform);
      mkdirSync(key, { recursive: true });
      writeFileSync(path.join(key, filename), bytes);
      chmodSync(path.join(key, filename), 0o755);
      writeFileSync(path.join(key, "SHA256SUMS"), `${digest(bytes)}  ${asset}\n`);
      writeFileSync(path.join(launch, "big file.txt"), "lorem ipsum ".repeat(20_000));
      const input = JSON.stringify({
        agent_action_name: "pre_read_code",
        tool_info: { file_path: "big file.txt" },
      });
      const result = await run(["hook", "pre-read-code", "--target", "windsurf"], input);
      assert.strictEqual(result.code, 2, result.stdout + result.stderr);
      assert.match(result.stderr, /bulk-read/);
      assert.deepStrictEqual(requests, []);
    });
    // 配布 binary が Rust 実装であることも含め、launcher の
    // 取得・検証・exec contract が不変であることを cargo build の実
    // artifact で確認する。cargo binary が無い環境では skip。
    it("acquires a cargo-built binary through the unmodified launcher", {
      timeout: 35_000,
    }, async (t) => {
      const actual = cargoBinary();
      if (!actual) {
        t.skip("cargo binary not found; run `mise exec -- cargo build -p cheaper-cli`");
        return;
      }
      body = readFileSync(actual);
      const versionProbe = spawnSync(actual, ["--version"], {
        encoding: "utf8",
        env,
        timeout: 30_000,
      });
      assert.strictEqual(versionProbe.status, 0, versionProbe.stderr);
      releaseVersion = versionProbe.stdout.trim();
      assert.match(releaseVersion, /^\d+\.\d+\.\d+$/);
      writeFileSync(
        path.join(launch, "cli/version.json"),
        `${JSON.stringify({ version: releaseVersion })}\n`,
      );
      manifest = `${digest(body)}  ${asset}\n`;
      const probe = await run(["--version"]);
      assert.strictEqual(probe.code, 0, probe.stderr);
      assert.strictEqual(probe.stdout.trim(), releaseVersion);
      assert.deepStrictEqual(
        [...requests].sort(),
        [`/v${releaseVersion}/${asset}`, `/v${releaseVersion}/SHA256SUMS`].sort(),
      );
      assert.deepStrictEqual(
        readdirSync(path.join(cache, releaseVersion, platform)).sort(),
        ["SHA256SUMS", filename].sort(),
      );
      writeFileSync(path.join(launch, "big.txt"), "lorem ipsum ".repeat(20_000));
      const hook = await run(
        ["hook", "pre-read-code", "--target", "windsurf"],
        JSON.stringify({
          agent_action_name: "pre_read_code",
          tool_info: { file_path: "big.txt" },
        }),
      );
      assert.strictEqual(hook.code, 2, hook.stdout + hook.stderr);
      assert.match(hook.stderr, /bulk-read/);
      clean(releaseVersion);
    });
    for (const target of ["windsurf", "claude", "codex", "cursor"]) {
      it(`installs binary-free source for ${target} and acquires a real Node-free hook from local HTTP`, {
        timeout: 60_000,
      }, async (t) => {
        const actual = cargoBinary();
        if (!apmAvailable()) {
          t.skip("apm CLI unavailable");
          return;
        }
        if (!actual) {
          t.skip("cargo binary not found; run `mise exec -- cargo build -p cheaper-cli`");
          return;
        }
        const { consumer } = makeConsumerLayout(root, target);
        const installed = apmInstall(consumer, root, target);
        assert.strictEqual(installed.status, 0, installed.stderr);
        launch = bundleRoot(consumer, target);
        assert.strictEqual(existsSync(path.join(launch, "cli/bin", platform)), false);
        assert.strictEqual(spawnSync("node", ["--version"], { env }).error?.code, "ENOENT");
        releaseVersion = JSON.parse(
          readFileSync(path.join(launch, "cli/version.json"), "utf8"),
        ).version;
        body = readFileSync(actual);
        manifest = `${digest(body)}  ${asset}\n`;
        writeFileSync(path.join(launch, "big.txt"), "lorem ipsum ".repeat(20_000));
        const result = await run(
          ["hook", "pre-read-code", "--target", "windsurf"],
          JSON.stringify({
            agent_action_name: "pre_read_code",
            tool_info: { file_path: "big.txt" },
          }),
        );
        assert.strictEqual(result.code, 2, result.stdout + result.stderr);
        assert.match(result.stderr, /bulk-read/);
        assert.deepStrictEqual(
          [...requests].sort(),
          [`/v${releaseVersion}/${asset}`, `/v${releaseVersion}/SHA256SUMS`].sort(),
        );
        assert.strictEqual(existsSync(path.join(cache, releaseVersion, platform, filename)), true);
        assert.strictEqual(existsSync(path.join(launch, "cli/bin", platform)), false);
      });
    }
  });
}
