import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import type { AuthInteraction } from "@earendil-works/pi-ai";
import { describe, expect, it } from "vitest";
import { describeProviderAuth, loginProvider, logoutProvider } from "../../../src/auth.js";
import { createModelRuntime } from "../../../src/model-runtime.js";
import type { DevinCliRunner } from "../../../src/providers/devin/cli.js";
import { devinCredentialsPath, devinDataDir } from "../../../src/providers/devin/credentials.js";
import { createDevinProviderPlugin } from "../../../src/providers/devin/plugin.js";

const INTERACTION: AuthInteraction = {
  notify() {},
  prompt: () => Promise.resolve(""),
};

interface Recorded {
  bin: string;
  args: string[];
  env: NodeJS.ProcessEnv;
  inheritStdio?: boolean | undefined;
}

/**
 * fake Devin CLI runner。`auth login` で isolated data dir へ
 * credentials.toml を生成し、`models list` は固定 catalog を返す。
 */
function fakeDevin(opts: {
  dataDirFor: (env: NodeJS.ProcessEnv) => string;
  catalogJson?: string;
  loginExitCode?: number;
}): { runner: DevinCliRunner; calls: Recorded[] } {
  const calls: Recorded[] = [];
  const catalog =
    opts.catalogJson ??
    JSON.stringify({
      families: [
        {
          family_label: "GPT Test",
          family_uid: "gpt-test-1",
          slug: "gpt-test",
          variants: [
            {
              model_uid: "gpt-test-1-high",
              label: "High",
              max_context_tokens: 400_000,
              max_output_tokens: 64_000,
              cost_summary: "$2.00 / MTok In · $8.00 / MTok Out",
            },
          ],
        },
      ],
    });
  return {
    calls,
    runner: {
      async run(bin, args, runOpts) {
        calls.push({ bin, args, env: runOpts.env, inheritStdio: runOpts.inheritStdio });
        if (args[0] === "auth" && args[1] === "login") {
          const code = opts.loginExitCode ?? 0;
          if (code === 0) {
            const credPath = devinCredentialsPath(opts.dataDirFor(runOpts.env));
            mkdirSync(path.dirname(credPath), { recursive: true });
            writeFileSync(credPath, 'api_key = "devin-fake-key"\n');
          }
          return { stdout: "", stderr: "", code };
        }
        if (args[0] === "models") {
          return { stdout: catalog, stderr: "", code: 0 };
        }
        return { stdout: "", stderr: "", code: 0 };
      },
    },
  };
}

async function makeRuntime(env: NodeJS.ProcessEnv, runner: DevinCliRunner) {
  return createModelRuntime({
    env,
    providerPlugins: [createDevinProviderPlugin({ runner })],
  });
}

describe("devin provider plugin", () => {
  it("registers the devin provider with bootstrap models before auth", async () => {
    const dir = await mkdtemp(path.join(tmpdir(), "cheaper-devin-plugin-"));
    const { runner } = fakeDevin({ dataDirFor: () => "" });
    const runtime = await makeRuntime({ CHEAPER_CONFIG_DIR: dir }, runner);

    const provider = runtime.getProvider("devin");
    expect(provider).toBeDefined();
    expect(runtime.getModel("devin", "claude-opus-5")).toBeDefined();

    const auth = await describeProviderAuth(runtime, "devin");
    expect(auth.methods).toEqual(["oauth"]);
    // Devin CLI の account-backed session なので subscription provider
    expect(auth.subscription).toBe(true);
    expect(auth.configured).toBe(false);
  });

  it("logs in via the Devin CLI, persists the credential, and refreshes the catalog", async () => {
    const dir = await mkdtemp(path.join(tmpdir(), "cheaper-devin-plugin-"));
    const env = { CHEAPER_CONFIG_DIR: dir };
    const dataDir = devinDataDir(dir);
    const { runner, calls } = fakeDevin({ dataDirFor: () => dataDir });
    const runtime = await makeRuntime(env, runner);

    const info = await loginProvider(runtime, "devin", "oauth", INTERACTION);
    expect(info.configured).toBe(true);

    // `devin auth login` が isolated env（XDG_DATA_HOME=dataDir）の
    // inherit-stdio child process として走った
    const login = calls.find((c) => c.args[0] === "auth" && c.args[1] === "login");
    expect(login?.inheritStdio).toBe(true);
    expect(login?.env.XDG_DATA_HOME).toBe(dataDir);
    // global env は不変
    expect(process.env.XDG_DATA_HOME).not.toBe(dataDir);

    // login 直後に `devin models list` で live catalog 取得
    expect(
      calls.some((c) => c.args[0] === "models" && c.args[1] === "list" && c.args[2] === "--format"),
    ).toBe(true);
    expect(runtime.getModel("devin", "gpt-test-1-high")).toBeDefined();

    // credential は <config>/pi/auth.json に永続化（data dir 外）
    const authJson = path.join(dir, "pi", "auth.json");
    const persisted = readFileSync(authJson, "utf8");
    expect(persisted).toContain("devin");

    // 認証済み devin model が availability に参加する
    const available = await runtime.getAvailable("devin");
    expect(available.length).toBeGreaterThan(0);
  });

  it("fails login with an actionable error when the Devin CLI is missing", async () => {
    const dir = await mkdtemp(path.join(tmpdir(), "cheaper-devin-plugin-"));
    const runner: DevinCliRunner = {
      run: () => {
        const err = new Error("spawn devin ENOENT") as Error & { code: string };
        err.code = "ENOENT";
        return Promise.reject(err);
      },
    };
    const runtime = await makeRuntime({ CHEAPER_CONFIG_DIR: dir }, runner);

    await expect(loginProvider(runtime, "devin", "oauth", INTERACTION)).rejects.toThrow(
      /Devin CLI not found|DEVIN_CLI/,
    );
  });

  it("fails login when `devin auth login` produces no credentials", async () => {
    const dir = await mkdtemp(path.join(tmpdir(), "cheaper-devin-plugin-"));
    const { runner } = fakeDevin({
      dataDirFor: () => devinDataDir(dir),
      loginExitCode: 1,
    });
    const runtime = await makeRuntime({ CHEAPER_CONFIG_DIR: dir }, runner);

    await expect(loginProvider(runtime, "devin", "oauth", INTERACTION)).rejects.toThrow(
      /credentials\.toml/,
    );
  });

  it("removes devin availability on logout", async () => {
    const dir = await mkdtemp(path.join(tmpdir(), "cheaper-devin-plugin-"));
    const { runner } = fakeDevin({ dataDirFor: () => devinDataDir(dir) });
    const runtime = await makeRuntime({ CHEAPER_CONFIG_DIR: dir }, runner);

    await loginProvider(runtime, "devin", "oauth", INTERACTION);
    expect((await describeProviderAuth(runtime, "devin")).configured).toBe(true);

    const info = await logoutProvider(runtime, "devin");
    expect(info.configured).toBe(false);
    expect(await runtime.getAvailable("devin")).toHaveLength(0);
  });

  it("closes the cheaper-owned Devin session on logout and re-runs `devin auth login`", async () => {
    const dir = await mkdtemp(path.join(tmpdir(), "cheaper-devin-plugin-"));
    const dataDir = devinDataDir(dir);
    const { runner, calls } = fakeDevin({ dataDirFor: () => dataDir });
    const runtime = await makeRuntime({ CHEAPER_CONFIG_DIR: dir }, runner);

    await loginProvider(runtime, "devin", "oauth", INTERACTION);
    const credPath = devinCredentialsPath(dataDir);
    expect(existsSync(credPath)).toBe(true);

    await logoutProvider(runtime, "devin");
    // isolated env で `devin auth logout` が走り、cheaper-owned の
    // credentials.toml が削除される（残ると次回 login が stale token を再利用する）
    expect(calls.some((c) => c.args[0] === "auth" && c.args[1] === "logout")).toBe(true);
    expect(existsSync(credPath)).toBe(false);

    calls.length = 0;
    await loginProvider(runtime, "devin", "oauth", INTERACTION);
    // stale credential 再利用ではなく `devin auth login` が再実行される
    expect(calls.some((c) => c.args[0] === "auth" && c.args[1] === "login")).toBe(true);
    expect((await describeProviderAuth(runtime, "devin")).configured).toBe(true);
  });
});
