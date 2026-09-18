import { Readable } from "node:stream";
import type { AuthInteraction, AuthPrompt } from "@cheaper/runtime-pi";
import { describe, expect, it } from "vitest";
import { type CliHooks, type CliIo, runCli } from "../src/cli.js";

function makeIo(stdinText = "", isTTY = false) {
  let stdout = "";
  let stderr = "";
  const stdin = Readable.from([stdinText]);
  (stdin as Readable & { isTTY: boolean }).isTTY = isTTY;
  const io: CliIo = {
    stdin,
    stdout: { write: (s: string) => (stdout += s) },
    stderr: { write: (s: string) => (stderr += s) },
    env: {},
    cwd: process.cwd(),
  };
  return { io, out: () => stdout, err: () => stderr };
}

const SECRET = "sk-test-secret-DO-NOT-PRINT";

interface FakeRuntime {
  getProvider(id: string): unknown;
  checkAuth(id: string): Promise<{ type: string } | undefined>;
  listCredentials(): Promise<readonly { providerId: string; type: string }[]>;
  login(id: string, type: string, interaction: AuthInteraction): Promise<unknown>;
  logout(id: string): Promise<void>;
  getProviders(): readonly unknown[];
}

function fakeRuntime(overrides: Partial<FakeRuntime> = {}): FakeRuntime {
  const provider = {
    id: "test-provider",
    name: "Test Provider",
    auth: {
      oauth: { name: "Test OAuth", isSubscription: true, login: async () => ({}) },
    },
  };
  return {
    getProvider: (id) => (id === "test-provider" ? provider : undefined),
    getProviders: () => [provider],
    checkAuth: async () => undefined,
    listCredentials: async () => [],
    login: async () => ({ accessToken: SECRET, refreshToken: SECRET }),
    logout: async () => {},
    ...overrides,
  };
}

function hooksFor(runtime: FakeRuntime, overrides: Partial<CliHooks> = {}): CliHooks {
  return {
    createModelRuntime: async () => runtime as never,
    ...overrides,
  };
}

function envelopeOf(stdout: string): Record<string, unknown> {
  const parsed = JSON.parse(stdout) as Record<string, unknown>;
  expect(parsed.schemaVersion).toBe(1);
  return parsed;
}

describe("auth args", () => {
  it("rejects bare `auth` and unknown actions", async () => {
    const { io } = makeIo();
    expect(await runCli(["auth"], io, hooksFor(fakeRuntime()))).toBe(2);
    expect(await runCli(["auth", "frobnicate"], io, hooksFor(fakeRuntime()))).toBe(2);
  });

  it("requires <provider> for login/logout and rejects --method on status", async () => {
    const { io } = makeIo();
    expect(await runCli(["auth", "login"], io, hooksFor(fakeRuntime()))).toBe(2);
    expect(await runCli(["auth", "logout"], io, hooksFor(fakeRuntime()))).toBe(2);
    expect(
      await runCli(["auth", "status", "--method", "device"], io, hooksFor(fakeRuntime())),
    ).toBe(2);
  });
});

describe("auth status", () => {
  it("returns credentials and provider readiness without secrets", async () => {
    const runtime = fakeRuntime({
      checkAuth: async () => ({ type: "oauth" }),
      listCredentials: async () => [{ providerId: "test-provider", type: "oauth" }],
    });
    const { io, out } = makeIo();
    const code = await runCli(["auth", "status"], io, hooksFor(runtime));
    expect(code).toBe(0);
    const env = envelopeOf(out());
    expect(env.ok).toBe(true);
    const result = env.result as {
      credentials: { providerId: string; type: string }[];
      providers: { providerId: string; configured: boolean; subscription: boolean }[];
    };
    expect(result.credentials).toEqual([{ providerId: "test-provider", type: "oauth" }]);
    expect(result.providers[0]).toMatchObject({
      providerId: "test-provider",
      configured: true,
      subscription: true,
      methods: ["oauth"],
    });
    expect(out()).not.toContain(SECRET);
  });

  it("fails with INVALID_REQUEST for an unknown provider filter", async () => {
    const { io, out } = makeIo();
    const code = await runCli(["auth", "status", "nope"], io, hooksFor(fakeRuntime()));
    expect(code).toBe(2);
    expect(envelopeOf(out()).ok).toBe(false);
  });
});

describe("auth logout", () => {
  it("removes the credential and reports loggedOut", async () => {
    let loggedOut: string | undefined;
    const runtime = fakeRuntime({
      logout: async (id) => {
        loggedOut = id;
      },
    });
    const { io, out } = makeIo();
    const code = await runCli(["auth", "logout", "test-provider"], io, hooksFor(runtime));
    expect(code).toBe(0);
    expect(loggedOut).toBe("test-provider");
    expect((envelopeOf(out()).result as Record<string, unknown>).loggedOut).toBe(true);
  });
});

describe("auth login", () => {
  it("rejects unknown providers with INVALID_REQUEST", async () => {
    const { io } = makeIo();
    const code = await runCli(["auth", "login", "nope"], io, hooksFor(fakeRuntime()));
    expect(code).toBe(2);
  });

  it("auto-answers select prompts via --method and never prints secrets", async () => {
    const prompts: AuthPrompt[] = [];
    const openedUrls: string[] = [];
    const runtime = fakeRuntime({
      login: async (_id, _type, interaction) => {
        const selected = await interaction.prompt({
          type: "select",
          message: "Choose login method",
          options: [
            { id: "browser", label: "Browser" },
            { id: "device", label: "Device code" },
          ],
        });
        prompts.push({ type: "select", message: `selected:${selected}`, options: [] });
        interaction.notify({
          type: "device_code",
          userCode: "ABCD-1234",
          verificationUri: "https://example.test/device",
        });
        return { accessToken: SECRET, refreshToken: SECRET };
      },
      checkAuth: async () => ({ type: "oauth" }),
    });
    const { io, out, err } = makeIo();
    const code = await runCli(
      ["auth", "login", "test-provider", "--method", "device"],
      io,
      hooksFor(runtime, { openUrl: (url) => openedUrls.push(url) }),
    );
    expect(code).toBe(0);
    expect(prompts[0]?.message).toBe("selected:device");
    expect(err()).toContain("ABCD-1234");
    expect(openedUrls).toEqual([]);
    const env = envelopeOf(out());
    expect(env.ok).toBe(true);
    expect(env.result).toMatchObject({
      provider: "test-provider",
      authenticated: true,
      method: "oauth",
      subscription: true,
    });
    expect(out()).not.toContain(SECRET);
    expect(err()).not.toContain(SECRET);
  });

  it("requests a browser launch through the injected openUrl hook on auth_url", async () => {
    const openedUrls: string[] = [];
    const runtime = fakeRuntime({
      login: async (_id, _type, interaction) => {
        interaction.notify({ type: "auth_url", url: "https://example.test/auth" });
        return { accessToken: SECRET, refreshToken: SECRET };
      },
      checkAuth: async () => ({ type: "oauth" }),
    });
    const { io, out, err } = makeIo();
    const code = await runCli(
      ["auth", "login", "test-provider"],
      io,
      hooksFor(runtime, { openUrl: (url) => openedUrls.push(url) }),
    );
    expect(code).toBe(0);
    expect(openedUrls).toEqual(["https://example.test/auth"]);
    expect(err()).toContain("https://example.test/auth");
    const env = envelopeOf(out());
    expect(env.ok).toBe(true);
    expect(env.result).toMatchObject({ provider: "test-provider", authenticated: true });
    expect(out()).not.toContain(SECRET);
    expect(err()).not.toContain(SECRET);
  });

  it("fails a select prompt on non-TTY stdin without --method, listing option ids", async () => {
    const runtime = fakeRuntime({
      login: async (_id, _type, interaction) => {
        await interaction.prompt({
          type: "select",
          message: "Choose login method",
          options: [
            { id: "browser", label: "Browser" },
            { id: "device", label: "Device code" },
          ],
        });
        return {};
      },
    });
    const { io, err } = makeIo();
    const code = await runCli(["auth", "login", "test-provider"], io, hooksFor(runtime));
    expect(code).toBe(2);
    expect(err()).toContain("browser|device");
  });

  it("routes an interactive TTY select prompt through the injected io.stderr", async () => {
    let selected = "";
    const runtime = fakeRuntime({
      login: async (_id, _type, interaction) => {
        selected = await interaction.prompt({
          type: "select",
          message: "Choose login method",
          options: [
            { id: "browser", label: "Browser" },
            { id: "device", label: "Device code", description: "For headless" },
          ],
        });
        return { accessToken: SECRET, refreshToken: SECRET };
      },
      checkAuth: async () => ({ type: "oauth" }),
    });
    // TTY stdin に "2" + Enter を feed。prompt 表示は injected stderr へ出る。
    const { io, out, err } = makeIo("2\n", true);
    const code = await runCli(
      ["auth", "login", "test-provider"],
      io,
      hooksFor(runtime, { openUrl: () => {} }),
    );
    expect(code).toBe(0);
    expect(selected).toBe("device");
    expect(err()).toContain("Choose login method");
    expect(err()).toContain("1) Browser");
    expect(err()).toContain("2) Device code");
    expect(err()).toContain("select [1-2]:");
    expect(envelopeOf(out()).ok).toBe(true);
    expect(out()).not.toContain(SECRET);
    expect(err()).not.toContain(SECRET);
  });

  it("routes a text prompt through the injected io.stderr and returns the answer", async () => {
    let answered = "";
    const runtime = fakeRuntime({
      login: async (_id, _type, interaction) => {
        answered = await interaction.prompt({
          type: "text",
          message: "Paste authorization code",
        });
        return { accessToken: SECRET, refreshToken: SECRET };
      },
      checkAuth: async () => ({ type: "oauth" }),
    });
    const { io, out, err } = makeIo("code-123\n", true);
    const code = await runCli(
      ["auth", "login", "test-provider"],
      io,
      hooksFor(runtime, { openUrl: () => {} }),
    );
    expect(code).toBe(0);
    expect(answered).toBe("code-123");
    expect(err()).toContain("Paste authorization code");
    expect(envelopeOf(out()).ok).toBe(true);
    expect(out()).not.toContain(SECRET);
    expect(err()).not.toContain(SECRET);
  });

  it("passes --type api_key through to the runtime login", async () => {
    let seenType: string | undefined;
    const keyProvider = {
      id: "key-provider",
      name: "Key Provider",
      auth: { apiKey: { name: "Test API key", login: async () => ({}) } },
    };
    const runtime = fakeRuntime({
      getProvider: (id) => (id === "key-provider" ? keyProvider : undefined),
      getProviders: () => [keyProvider],
      login: async (_id, type, interaction) => {
        seenType = type;
        await interaction.prompt({ type: "secret", message: "API key" }).catch(() => "");
        return { type: "api_key", key: SECRET };
      },
      checkAuth: async () => ({ type: "api_key" }),
    });
    const { io, out, err } = makeIo();
    const code = await runCli(
      ["auth", "login", "key-provider", "--type", "api_key"],
      io,
      hooksFor(runtime),
    );
    expect(code).toBe(0);
    expect(seenType).toBe("api_key");
    const env = envelopeOf(out());
    expect(env.result).toMatchObject({ provider: "key-provider", method: "api_key" });
    expect(out()).not.toContain(SECRET);
    expect(err()).not.toContain(SECRET);
  });

  it("rejects api_key login on providers without an interactive key login", async () => {
    const { io } = makeIo();
    const code = await runCli(
      ["auth", "login", "test-provider", "--type", "api_key"],
      io,
      hooksFor(fakeRuntime()),
    );
    expect(code).toBe(2);
  });

  it("reads api_key secrets without echoing them to the terminal", async () => {
    const keyProvider = {
      id: "key-provider",
      name: "Key Provider",
      auth: { apiKey: { name: "Test API key", login: async () => ({}) } },
    };
    let entered = "";
    const runtime = fakeRuntime({
      getProvider: (id) => (id === "key-provider" ? keyProvider : undefined),
      getProviders: () => [keyProvider],
      login: async (_id, type, interaction) => {
        expect(type).toBe("api_key");
        entered = await interaction.prompt({ type: "secret", message: "API key" });
        return { type: "api_key", key: entered };
      },
      checkAuth: async () => ({ type: "api_key" }),
    });
    // TTY stdin に key + Enter を feed。no-echo reader が値を受け取る。
    const { io, out, err } = makeIo(`${SECRET}\n`, true);
    const code = await runCli(
      ["auth", "login", "key-provider", "--type", "api_key"],
      io,
      hooksFor(runtime),
    );
    expect(code).toBe(0);
    expect(entered).toBe(SECRET);
    // prompt メッセージは出るが secret 値自体は stderr/stdout/envelope に出ない
    expect(err()).toContain("API key");
    expect(err()).not.toContain(SECRET);
    expect(out()).not.toContain(SECRET);
  });

  it("rejects --type on non-login actions and invalid values", async () => {
    const { io } = makeIo();
    expect(await runCli(["auth", "status", "--type", "api_key"], io, hooksFor(fakeRuntime()))).toBe(
      2,
    );
    expect(
      await runCli(
        ["auth", "login", "test-provider", "--type", "bogus"],
        io,
        hooksFor(fakeRuntime()),
      ),
    ).toBe(2);
  });

  it("rejects providers without oauth support", async () => {
    const runtime = fakeRuntime({
      getProvider: (id) =>
        id === "key-only" ? { id, name: "KeyOnly", auth: { apiKey: {} } } : undefined,
      getProviders: () => [],
    });
    const { io } = makeIo();
    const code = await runCli(["auth", "login", "key-only"], io, hooksFor(runtime));
    expect(code).toBe(2);
  });

  it("propagates cancellation through the interaction signal", async () => {
    const ac = new AbortController();
    ac.abort();
    const runtime = fakeRuntime({
      login: async (_id, _type, interaction) => {
        if (interaction.signal?.aborted !== true) {
          throw new Error("signal not propagated");
        }
        throw new Error("login aborted");
      },
    });
    const { io, out } = makeIo();
    const code = await runCli(
      ["auth", "login", "test-provider"],
      io,
      hooksFor(runtime, { signal: ac.signal }),
    );
    expect(code).not.toBe(0);
    expect(envelopeOf(out()).ok).toBe(false);
  });
});
