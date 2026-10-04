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
import { createServer, type Server } from "node:http";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it } from "vitest";

const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../../..");
const hooks = path.join(repo, "packages/apm/.apm/hooks");
const windows = process.platform === "win32";
const platform = windows ? "win32-x64" : `${process.platform}-${process.arch}`;
const filename = windows ? "cheaper.exe" : "cheaper";
const asset = windows ? "cheaper-windows-x64.exe" : `cheaper-${platform}`;
const sh = windows ? "C:/Program Files/Git/usr/bin/sh.exe" : "/bin/sh";
const psProbe = spawnSync("pwsh", ["-NoProfile", "-Command", "(Get-Process -Id $PID).Path"], {
  encoding: "utf8",
});
const pwshPath = psProbe.stdout?.trim();
const pwsh = psProbe.status === 0;
const runners = [
  { name: "sh", available: existsSync(sh) },
  { name: "pwsh", available: windows && pwsh },
];
const digest = (bytes: Buffer) => createHash("sha256").update(bytes).digest("hex");

type Result = { code: number | null; stdout: string; stderr: string };
let fixtureRoot: string;
let binary: Buffer;

beforeAll(() => {
  fixtureRoot = mkdtempSync(path.join(tmpdir(), "cheaper-native-fixture-"));
  if (windows) {
    const compiler = path.join(
      process.env.WINDIR ?? "C:/Windows",
      "Microsoft.NET/Framework64/v4.0.30319/csc.exe",
    );
    const target = path.join(fixtureRoot, filename);
    const compile = spawnSync(
      compiler,
      [
        "/nologo",
        `/out:${target}`,
        path.join(repo, "packages/cli/test/e2e/fixtures/launcher-fixture.cs"),
      ],
      { encoding: "utf8" },
    );
    expect(compile.status, compile.stderr + compile.stdout).toBe(0);
    binary = readFileSync(target);
  } else {
    binary = Buffer.from(
      '#!/bin/sh\nif [ "$1" = "--version" ]; then cat >/dev/null; echo 1.0.0; exit 0; fi\nprintf "ARGS:\\n"\nprintf "%s\\n" "$@"\nprintf "INPUT:"\ncat\necho "fixture stderr" >&2\n[ "$1" != hook ] || exit 2\n',
    );
  }
});
afterAll(() => {
  if (fixtureRoot) rmSync(fixtureRoot, { recursive: true, force: true });
});

for (const runner of runners) {
  describe.runIf(runner.available && ["win32-x64", "linux-x64", "linux-arm64"].includes(platform))(
    `${runner.name} version-pinned launcher`,
    { timeout: 35_000 },
    () => {
      let root: string;
      let launch: string;
      let cache: string;
      let env: NodeJS.ProcessEnv;
      let server: Server | undefined;
      let base: string;
      let requests: string[];
      let manifest: string;
      let body: Buffer;
      let behavior: "normal" | "interrupt" | "missing" | "redirect";

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
        manifest = `${digest(binary)}  ${asset}\n`;
        behavior = "normal";
        requests = [];
        server = createServer((req, res) => {
          requests.push(req.url ?? "");
          if (behavior === "redirect") {
            res.writeHead(302, { location: "http://example.invalid/forbidden" }).end();
            return;
          }
          if (req.url === "/v1.0.0/SHA256SUMS") {
            if (behavior === "missing") {
              res.writeHead(404).end();
              return;
            }
            res.end(manifest);
          } else if (req.url === `/v1.0.0/${asset}`) {
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
        await new Promise<void>((resolve) => server?.listen(0, "127.0.0.1", resolve));
        const address = server.address();
        if (!address || typeof address === "string") throw new Error("No stub port");
        base = `http://127.0.0.1:${address.port}`;
        env = {
          ...process.env,
          PATH: windows
            ? "C:/Program Files/Git/usr/bin;C:/Windows/System32;C:/Windows/System32/WindowsPowerShell/v1.0"
            : "/usr/bin:/bin",
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
          await new Promise<void>((resolve) => server?.close(() => resolve()));
        }
        rmSync(root, { recursive: true, force: true });
      });

      function run(
        argv = ["--version"],
        input = "",
        extra: NodeJS.ProcessEnv = {},
        command?: string,
      ) {
        const executable = runner.name === "sh" ? sh : pwshPath;
        const args =
          runner.name === "sh"
            ? [path.join(launch, "cheaper"), ...argv]
            : command
              ? ["-NoProfile", "-Command", command]
              : ["-NoProfile", "-File", path.join(launch, "cheaper.ps1"), ...argv];
        return new Promise<Result>((resolve, reject) => {
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
      function clean() {
        const parent = path.join(cache, "1.0.0");
        expect(
          existsSync(parent) ? readdirSync(parent).filter((p) => p.startsWith(".")) : [],
        ).toEqual([]);
      }
      function failed(result: Result, message: RegExp) {
        expect(result.code, result.stdout + result.stderr).not.toBe(0);
        expect(result.stdout).toBe("");
        expect(result.stderr).toMatch(message);
        clean();
      }

      it("uses sibling without consulting version, cache, or network", async () => {
        const dir = path.join(launch, "cli/bin", platform);
        mkdirSync(dir, { recursive: true });
        writeFileSync(path.join(dir, filename), binary);
        chmodSync(path.join(dir, filename), 0o755);
        rmSync(path.join(launch, "cli/version.json"));
        const result = await run();
        expect(result.code, result.stderr).toBe(0);
        expect(result.stdout.trim()).toBe("1.0.0");
        expect(requests).toEqual([]);
        expect(existsSync(cache)).toBe(false);
      });
      it("acquires the pinned pair and executes with preserved hook stdin, argv, and exit", async () => {
        const argv = ["hook", "pre-tool-use", "space arg", "", 'quote";$()\\end'];
        const input = '{"tool_name":"Read","path":"space file"}';
        const result = await run(argv, input);
        expect(result.code, result.stderr).toBe(2);
        expect(result.stdout.replace(/\r/g, "")).toBe(`ARGS:\n${argv.join("\n")}\nINPUT:${input}`);
        expect(result.stderr).toContain("fixture stderr");
        expect(requests.sort()).toEqual([`/v1.0.0/${asset}`, "/v1.0.0/SHA256SUMS"].sort());
        expect(readdirSync(path.join(cache, "1.0.0", platform)).sort()).toEqual(
          ["SHA256SUMS", filename].sort(),
        );
        clean();
      });
      it("executes an offline verified preseed without a download", async () => {
        preseed();
        const result = await run(["--version"], "", {
          CHEAPER_RELEASE_BASE_URL: "https://example.invalid/releases",
        });
        expect(result.code, result.stderr).toBe(0);
        expect(result.stdout.trim()).toBe("1.0.0");
        expect(requests).toEqual([]);
        clean();
      });
      it("downloads only once under concurrent first-run", async () => {
        const results = await Promise.all([run(), run()]);
        for (const r of results) expect(r.code, r.stderr).toBe(0);
        expect(requests.filter((p) => p.endsWith("SHA256SUMS"))).toHaveLength(1);
        expect(requests.filter((p) => p.endsWith(asset))).toHaveLength(1);
        clean();
      });
      it.each([
        "corrupt",
        "interrupted",
        "missing-manifest",
        "bad-manifest",
        "duplicate",
        "wrong-version",
        "broken-executable",
        "redirect",
      ])(
        "fails closed for %s without leaving partial files",
        async (failure) => {
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
            expect(existsSync(path.join(cache, "1.0.0", platform))).toBe(false);
        },
        35_000,
      );
      function copyDirectoryPair(from: string, to: string) {
        const target = path.join(to, platform);
        mkdirSync(target);
        for (const f of [filename, "SHA256SUMS"])
          copyFileSync(path.join(from, platform, f), path.join(target, f));
        chmodSync(path.join(target, filename), 0o755);
      }
      it.each(["binary-only", "manifest-only", "hash-change"])(
        "rejects %s cache without repairing it",
        async (failure) => {
          const key = preseed();
          if (failure === "binary-only") rmSync(path.join(key, "SHA256SUMS"));
          if (failure === "manifest-only") rmSync(path.join(key, filename));
          if (failure === "hash-change") writeFileSync(path.join(key, filename), "changed");
          failed(await run(), /manifest|checksum|cache/i);
          expect(requests).toEqual([]);
        },
      );
      it("reports unwritable cache boundary without downloading", async () => {
        writeFileSync(cache, "regular file");
        failed(await run(), /cache|writ|directory/i);
        expect(requests).toEqual([]);
      });
      it("does not write a cache under deployed hooks", async () => {
        failed(
          await run(["--version"], "", { CHEAPER_CACHE_DIR: path.join(launch, "bad-cache") }),
          /cache|hook/i,
        );
        expect(existsSync(path.join(launch, "bad-cache"))).toBe(false);
      });
      it("rejects missing or malformed version metadata before downloading", async () => {
        rmSync(path.join(launch, "cli/version.json"));
        failed(await run(), /version/i);
        writeFileSync(path.join(launch, "cli/version.json"), '{"version":"../latest"}');
        failed(await run(), /version/i);
        expect(requests).toEqual([]);
      });
      it("rejects LFS pointers rather than acquiring", async () => {
        const dir = path.join(launch, "cli/bin", platform);
        mkdirSync(dir, { recursive: true });
        writeFileSync(
          path.join(dir, filename),
          "version https://git-lfs.github.com/spec/v1\noid sha256:0000\n",
        );
        failed(await run(), /LFS pointer/i);
        expect(requests).toEqual([]);
      });
      it.runIf(runner.name === "pwsh")("rejects Windows ARM64 acquisition", async () => {
        failed(
          await run(["--version"], "", {
            PROCESSOR_ARCHITEW6432: "ARM64",
            PROCESSOR_ARCHITECTURE: "ARM64",
          }),
          /unsupported/i,
        );
        expect(requests).toEqual([]);
      });
      it.runIf(runner.name === "pwsh")(
        "uses Invoke-WebRequest when curl.exe is unavailable",
        async () => {
          const result = await run(["--version"], "", {
            PATH: "C:/Windows/System32/WindowsPowerShell/v1.0",
          });
          expect(result.code, result.stderr).toBe(0);
          expect(requests).toHaveLength(2);
          clean();
        },
      );
      it.runIf(runner.name === "pwsh")(
        "rejects plain HTTP redirects in Invoke-WebRequest fallback",
        async () => {
          behavior = "redirect";
          failed(
            await run(["--version"], "", {
              PATH: "C:/Windows/System32/WindowsPowerShell/v1.0",
            }),
            /download/i,
          );
          expect(requests).toHaveLength(1);
        },
      );
      it.runIf(runner.name === "sh")(
        "rejects unsupported acquisition but retains unsupported-host sibling compatibility",
        async () => {
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
          expect(result.code, result.stderr).toBe(0);
          expect(requests).toEqual([]);
        },
      );
      it.runIf(runner.name === "pwsh")(
        "runs declared PowerShell hooks and preserves their fallback policies",
        async () => {
          preseed();
          const declared = JSON.parse(readFileSync(path.join(launch, "hooks.json"), "utf8")) as {
            hooks: Record<string, { powershell?: string }[]>;
          };
          for (const entries of Object.values(declared.hooks)) {
            for (const entry of entries) {
              if (!entry.powershell) continue;
              expect(entry.powershell).toContain("cheaper.ps1");
              expect(entry.powershell).not.toContain("cli/bin");
            }
          }
          const direct = await run(
            [],
            '{"tool_name":"Read"}',
            {},
            declared.hooks.pre_read_code[0].powershell,
          );
          expect(direct.code, direct.stderr).toBe(2);
          expect(direct.stdout).toContain('INPUT:{"tool_name":"Read"}');
          const copilot = await run([], "{}", {}, declared.hooks.preToolUse[0].powershell);
          expect(copilot.code).toBe(0);
          const cursor = await run([], "{}", {}, declared.hooks.beforeReadFile[0].powershell);
          expect(cursor.code).toBe(0);
          expect(cursor.stdout).toContain('{"permission":"allow"}');
        },
      );
      it("runs a real CLI hook via cached executable with original stdin and exit 2", async () => {
        const actual = path.join(hooks, "cli/bin", platform, filename);
        if (!existsSync(actual)) throw new Error(`Missing setup executable ${actual}`);
        const bytes = readFileSync(actual);
        const actualVersion = spawnSync(actual, ["--version"], {
          encoding: "utf8",
          env,
        }).stdout.trim();
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
        expect(result.code, result.stdout + result.stderr).toBe(2);
        expect(result.stderr).toContain("bulk-read");
        expect(requests).toEqual([]);
      }, 35_000);
    },
  );
}
