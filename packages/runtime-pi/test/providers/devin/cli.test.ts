import { describe, expect, it } from "vitest";
import {
  createDevinCli,
  DevinCliNotFoundError,
  type DevinCliResult,
  type DevinCliRunner,
  devinChildEnv,
} from "../../../src/providers/devin/cli.js";

function fakeRunner(
  handler: (
    bin: string,
    args: string[],
    opts: { env: NodeJS.ProcessEnv },
  ) => DevinCliResult | Promise<DevinCliResult>,
): DevinCliRunner {
  return { run: handler };
}

describe("devinChildEnv", () => {
  it("sets XDG_DATA_HOME to the isolated data dir without mutating inputs", () => {
    const env = { PATH: "/usr/bin", CHEAPER_CONFIG_DIR: "/cfg" };
    const before = process.env.XDG_DATA_HOME;
    const child = devinChildEnv(env, "/cfg/providers/devin/data");

    expect(child.XDG_DATA_HOME).toBe("/cfg/providers/devin/data");
    expect(child.PATH).toBe("/usr/bin");
    expect(env.XDG_DATA_HOME).toBeUndefined();
    expect(process.env.XDG_DATA_HOME).toBe(before);
  });

  it("sets LOCALAPPDATA alongside XDG_DATA_HOME on win32", () => {
    const child = devinChildEnv({}, "/data");
    expect(child.XDG_DATA_HOME).toBe("/data");
    if (process.platform === "win32") {
      expect(child.LOCALAPPDATA).toBe("/data");
    }
  });
});

describe("createDevinCli", () => {
  it("runs devin with separated argv and the isolated env", async () => {
    const seen: { bin: string; args: string[]; env: NodeJS.ProcessEnv }[] = [];
    const cli = createDevinCli({
      env: {},
      dataDir: "/cfg/providers/devin/data",
      runner: fakeRunner((bin, args, opts) => {
        seen.push({ bin, args, env: opts.env });
        return { stdout: "ok", stderr: "", code: 0 };
      }),
    });

    const result = await cli.run(["auth", "status"]);
    expect(result).toEqual({ stdout: "ok", stderr: "", code: 0 });
    expect(seen[0]?.bin).toBe("devin");
    expect(seen[0]?.args).toEqual(["auth", "status"]);
    expect(seen[0]?.env.XDG_DATA_HOME).toBe("/cfg/providers/devin/data");
  });

  it("honors DEVIN_CLI override for binary resolution", async () => {
    const seen: string[] = [];
    const cli = createDevinCli({
      env: { DEVIN_CLI: "/custom/devin" },
      dataDir: "/data",
      runner: fakeRunner((bin) => {
        seen.push(bin);
        return { stdout: "", stderr: "", code: 0 };
      }),
    });

    await cli.run(["version"]);
    expect(cli.bin).toBe("/custom/devin");
    expect(seen).toEqual(["/custom/devin"]);
  });

  it("maps ENOENT to an actionable DevinCliNotFoundError", async () => {
    const cli = createDevinCli({
      env: {},
      dataDir: "/data",
      runner: {
        run: () => {
          const err = new Error("spawn devin ENOENT") as Error & { code: string };
          err.code = "ENOENT";
          return Promise.reject(err);
        },
      },
    });

    await expect(cli.run(["version"])).rejects.toThrow(DevinCliNotFoundError);
    await expect(cli.run(["version"])).rejects.toThrow(/DEVIN_CLI/);
  });

  it("version() returns trimmed stdout, or null on failure", async () => {
    const cli = createDevinCli({
      env: {},
      dataDir: "/data",
      runner: fakeRunner(() => ({ stdout: "  1.2.3\n", stderr: "", code: 0 })),
    });
    expect(await cli.version()).toBe("1.2.3");

    const failing = createDevinCli({
      env: {},
      dataDir: "/data",
      runner: fakeRunner(() => ({ stdout: "", stderr: "boom", code: 1 })),
    });
    expect(await failing.version()).toBeNull();

    const missing = createDevinCli({
      env: {},
      dataDir: "/data",
      runner: { run: () => Promise.reject(new Error("nope")) },
    });
    expect(await missing.version()).toBeNull();
  });
});
