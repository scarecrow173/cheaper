import { spawnSync } from "node:child_process";
import { mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  apmAvailable,
  apmInstall,
  bundledCli,
  bundleRoot,
  makeConsumerLayout,
} from "./apm-fixture.js";

describe("APM consumer install smoke (windsurf target)", () => {
  let root: string;
  let consumer: string;

  beforeEach(() => {
    if (!apmAvailable()) return;
    root = mkdtempSync(path.join(tmpdir(), "cheaper-apm-"));
    ({ consumer } = makeConsumerLayout(root));
  });
  afterEach(() => {
    if (root) rmSync(root, { recursive: true, force: true });
  });

  it("deploys skills, hook bundle, and a working bundled CLI", async (t) => {
    if (!apmAvailable()) {
      t.skip("apm CLI not available on PATH");
      return;
    }
    const install = apmInstall(consumer, root);
    expect(install.status).toBe(0);

    // skills deploy
    const skillsDir = path.join(consumer, ".agents", "skills");
    for (const s of [
      "cheaper-usage",
      "cheaper-bulk-read",
      "cheaper-code-write",
      "cheaper-doc-write",
    ]) {
      expect(readdirSync(path.join(skillsDir, s)), `skill ${s} deployed`).toContain("SKILL.md");
    }

    // hook config + bundle deploy（bundle dir は dep dir 名 = "pkg"）
    const hooksJson = JSON.parse(
      readFileSync(path.join(consumer, ".windsurf", "hooks.json"), "utf8"),
    ) as { hooks: Record<string, Array<{ command: string; powershell?: string }>> };
    const entry = hooksJson.hooks.pre_read_code?.[0];
    expect(entry).toBeDefined();
    const bundle = bundleRoot(consumer);
    expect(entry.command).toContain("pre-read-code.js");
    expect(entry.command).toContain("cli/bin/cheaper.mjs");
    // powershell -Command collapses the hook's exit 2 to 1 unless the command
    // propagates $LASTEXITCODE — without it Cascade never sees the block.
    expect(entry.powershell).toContain("exit $LASTEXITCODE");
    const cli = bundledCli(consumer);

    // bundled CLI が動く
    const version = spawnSync(process.execPath, [cli, "--version"], {
      cwd: consumer,
      encoding: "utf8",
    });
    expect(version.status).toBe(0);
    expect(version.stdout.trim()).toMatch(/^\d+\.\d+\.\d+$/);

    // hook が large file を bundled CLI で block する
    writeFileSync(
      path.join(consumer, "big.txt"),
      "lorem ipsum dolor sit amet ".repeat(4000),
      "utf8",
    );
    const hook = spawnSync(process.execPath, [path.join(bundle, "pre-read-code.js"), cli], {
      cwd: consumer,
      input: JSON.stringify({
        agent_action_name: "pre_read_code",
        tool_info: { file_path: path.join(consumer, "big.txt") },
      }),
      encoding: "utf8",
    });
    expect(hook.status).toBe(2);
    expect(hook.stderr).toContain("bulk-read");
  }, 120_000);
});
