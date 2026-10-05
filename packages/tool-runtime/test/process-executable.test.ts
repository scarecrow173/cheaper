import { mkdir, mkdtemp, realpath, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { resolveProcessExecutable } from "../src/process-executable.js";

let root: string;
beforeEach(async () => {
  root = await realpath(await mkdtemp(path.join(process.cwd(), ".tmp-process-executable-")));
});
afterEach(async () => {
  expect(path.dirname(root)).toBe(await realpath(process.cwd()));
  await rm(root, { recursive: true, force: true });
});
const options = () => ({
  root,
  symlinkPolicy: "deny" as const,
  searchPath: "",
  execPath: path.join(root, "cheaper.exe"),
});
const suffix = process.platform === "win32" ? ".exe" : "";
describe("supported native executable resolution", () => {
  it("uses a real Node executable instead of a SEA executable", async () => {
    expect(
      await resolveProcessExecutable("node", { ...options(), execPath: process.execPath }),
    ).toBe(await realpath(process.execPath));
    const dir = path.join(root, "native");
    await mkdir(dir);
    const native = path.join(dir, `node${suffix}`);
    await writeFile(
      native,
      process.platform === "win32"
        ? Buffer.from("MZbinary")
        : Buffer.from([0x7f, 0x45, 0x4c, 0x46]),
      { mode: 0o755 },
    );
    expect(await resolveProcessExecutable("node", { ...options(), searchPath: dir })).toBe(
      await realpath(native),
    );
  });
  it("fails closed when PATH has no supported executable", async () => {
    await expect(resolveProcessExecutable("node", options())).rejects.toMatchObject({
      code: "POLICY_VIOLATION",
    });
    await expect(resolveProcessExecutable("missing-tool", options())).rejects.toMatchObject({
      code: "POLICY_VIOLATION",
    });
  });
  it("refuses script shims and executable text without using a shell", async () => {
    const shim = path.join(root, "pnpm.cmd");
    await writeFile(shim, "echo shim");
    await expect(resolveProcessExecutable(shim, options())).rejects.toMatchObject({
      code: "POLICY_VIOLATION",
    });
    const text = path.join(root, `script${suffix}`);
    await writeFile(text, "#!/bin/sh\necho hidden-shell\n", { mode: 0o755 });
    await expect(resolveProcessExecutable(text, options())).rejects.toMatchObject({
      code: "POLICY_VIOLATION",
    });
  });
});
