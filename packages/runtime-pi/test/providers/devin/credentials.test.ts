import { mkdirSync, writeFileSync } from "node:fs";
import { mkdir, mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { describe, expect, it } from "vitest";
import type { DevinCli } from "../../../src/providers/devin/cli.js";
import {
  devinAuthStatus,
  devinCredentialsPath,
  devinDataDir,
  loginWithDevinCli,
  readDevinCredentials,
} from "../../../src/providers/devin/credentials.js";

function tmpdir_() {
  return mkdtemp(path.join(tmpdir(), "cheaper-devin-creds-"));
}

async function writeCredentials(dataDir: string, extra = ""): Promise<string> {
  const credPath = devinCredentialsPath(dataDir);
  await mkdir(path.dirname(credPath), { recursive: true });
  await writeFile(credPath, `api_key = "devin-test-key"\n${extra}`, { mode: 0o600 });
  return credPath;
}

function cliStub(handler: (args: string[]) => { stdout: string; stderr: string; code: number }): {
  cli: DevinCli;
  calls: string[][];
} {
  const calls: string[][] = [];
  return {
    calls,
    cli: {
      bin: "devin",
      async run(args) {
        calls.push(args);
        return handler(args);
      },
      async version() {
        return null;
      },
    },
  };
}

describe("devinDataDir / devinCredentialsPath", () => {
  it("places credentials under <userConfigDir>/providers/devin/data", () => {
    expect(devinDataDir("/cfg")).toBe(path.join("/cfg", "providers", "devin", "data"));
    expect(devinCredentialsPath("/data")).toBe(path.join("/data", "devin", "credentials.toml"));
  });
});

describe("readDevinCredentials", () => {
  it("returns null when credentials.toml is absent", async () => {
    const dir = await tmpdir_();
    expect(readDevinCredentials(dir)).toBeNull();
  });

  it("parses api_key with default endpoints", async () => {
    const dir = await tmpdir_();
    const credPath = await writeCredentials(dir);
    const creds = readDevinCredentials(dir);

    expect(creds?.apiKey).toBe("devin-test-key");
    expect(creds?.apiServerUrl).toBe("https://server.codeium.com");
    expect(creds?.webappHost).toBe("app.devin.ai");
    expect(creds?.path).toBe(credPath);
  });

  it("accepts windsurf_api_key and honors api_server_url override", async () => {
    const dir = await tmpdir_();
    const credPath = devinCredentialsPath(dir);
    await mkdir(path.dirname(credPath), { recursive: true });
    await writeFile(
      credPath,
      [
        'windsurf_api_key = "wsk-123"',
        'api_server_url = "https://server.example.com/"',
        'devin_webapp_host = "devin.example.com"',
        "",
      ].join("\n"),
    );

    const creds = readDevinCredentials(dir);
    expect(creds?.apiKey).toBe("wsk-123");
    expect(creds?.apiServerUrl).toBe("https://server.example.com");
    expect(creds?.webappHost).toBe("devin.example.com");
  });

  it("returns null when no key field is present", async () => {
    const dir = await tmpdir_();
    const credPath = devinCredentialsPath(dir);
    await mkdir(path.dirname(credPath), { recursive: true });
    await writeFile(credPath, 'api_server_url = "https://x"\n');
    expect(readDevinCredentials(dir)).toBeNull();
  });

  it("never reads the global ~/.local/share/devin path", async () => {
    // cheaper data dir のみ参照する。global path への依存があれば
    // このテスト環境では存在しないため null になるはず。
    const dir = await tmpdir_();
    expect(readDevinCredentials(dir)).toBeNull();
    await writeCredentials(dir);
    expect(readDevinCredentials(dir)?.apiKey).toBe("devin-test-key");
  });
});

describe("devinAuthStatus", () => {
  it("reports logged in when the CLI says so", async () => {
    const dir = await tmpdir_();
    const { cli } = cliStub(() => ({
      stdout: "Logged in as user@example.com",
      stderr: "",
      code: 0,
    }));
    const status = await devinAuthStatus(cli, dir);
    expect(status.loggedIn).toBe(true);
  });

  it("reports logged in when credentials.toml exists even if status fails", async () => {
    const dir = await tmpdir_();
    await writeCredentials(dir);
    const { cli } = cliStub(() => ({ stdout: "", stderr: "not signed in", code: 1 }));
    const status = await devinAuthStatus(cli, dir);
    expect(status.loggedIn).toBe(true);
  });

  it("reports logged out without credentials and failing status", async () => {
    const dir = await tmpdir_();
    const { cli } = cliStub(() => ({ stdout: "", stderr: "nope", code: 1 }));
    const status = await devinAuthStatus(cli, dir);
    expect(status.loggedIn).toBe(false);
  });
});

describe("loginWithDevinCli", () => {
  it("returns existing credentials when the CLI session is still valid", async () => {
    const dir = await tmpdir_();
    await writeCredentials(dir);
    const { cli, calls } = cliStub(() => ({
      stdout: "Logged in",
      stderr: "",
      code: 0,
    }));

    const creds = await loginWithDevinCli(cli, dir);
    expect(creds.apiKey).toBe("devin-test-key");
    // `devin auth status` のみで `auth login` は再実行しない
    expect(calls).toEqual([["auth", "status"]]);
  });

  it("runs `devin auth login` with inherited stdio then reads the new file", async () => {
    const dir = await tmpdir_();
    const { cli, calls } = cliStub((args) => {
      if (args[0] === "auth" && args[1] === "login") {
        // fake CLI: login 完了で credentials.toml を生成した扱いにする
        const credPath = devinCredentialsPath(dir);
        mkdirSync(path.dirname(credPath), { recursive: true });
        writeFileSync(credPath, 'api_key = "devin-test-key"\n');
        return { stdout: "", stderr: "", code: 0 };
      }
      return { stdout: "", stderr: "", code: 1 };
    });

    const creds = await loginWithDevinCli(cli, dir);
    expect(calls).toEqual([["auth", "login"]]);
    expect(creds.apiKey).toBe("devin-test-key");
  });

  it("fails with an actionable error when login produces no credentials", async () => {
    const dir = await tmpdir_();
    const { cli } = cliStub(() => ({ stdout: "", stderr: "", code: 0 }));

    await expect(loginWithDevinCli(cli, dir)).rejects.toThrow(/credentials\.toml/);
  });
});
