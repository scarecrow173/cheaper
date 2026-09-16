import { spawnSync } from "node:child_process";
import {
  cpSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { cliRoot } from "./helpers.js";

const repoRoot = path.join(cliRoot, "..", "..");

function apmAvailable(): boolean {
  const r = spawnSync("apm", ["--version"], { shell: true, encoding: "utf8" });
  return r.status === 0;
}

// local path dep として install できる最小 package（repo の .apm/ + 無依存
// apm.yml）を作る。repo 本体は transitive remote deps と node_modules を
// 含むため、そのままでは local dep に使えない（DESIGN-0014 / DISC-0006）。
function makeConsumerLayout(root: string): { pkg: string; consumer: string } {
  const pkg = path.join(root, "pkg");
  const consumer = path.join(root, "consumer");
  mkdirSync(pkg, { recursive: true });
  mkdirSync(consumer, { recursive: true });
  cpSync(path.join(repoRoot, ".apm"), path.join(pkg, ".apm"), { recursive: true });
  writeFileSync(path.join(pkg, "apm.yml"), 'name: cheaper\nversion: "0.0.0"\n');
  writeFileSync(
    path.join(consumer, "apm.yml"),
    'name: consumer\nversion: "0.0.1"\ntargets:\n- windsurf\ndependencies:\n  apm:\n  - path: ../pkg\n',
  );
  return { pkg, consumer };
}

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
    const install = spawnSync("apm", ["install", "--target", "windsurf"], {
      cwd: consumer,
      shell: true,
      encoding: "utf8",
      env: { ...process.env, HOME: root, USERPROFILE: root },
    });
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
    ) as { hooks: Record<string, Array<{ command: string }>> };
    const entry = hooksJson.hooks.pre_read_code?.[0];
    expect(entry).toBeDefined();
    const bundleRoot = path.join(consumer, ".windsurf", "hooks", "pkg");
    expect(entry.command).toContain("pre-read-code.js");
    expect(entry.command).toContain("cli/bin/cheaper.mjs");
    const bundledCli = path.join(bundleRoot, "cli", "bin", "cheaper.mjs");

    // bundled CLI が動く
    const version = spawnSync(process.execPath, [bundledCli, "--version"], {
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
    const hook = spawnSync(
      process.execPath,
      [path.join(bundleRoot, "pre-read-code.js"), bundledCli],
      {
        cwd: consumer,
        input: JSON.stringify({
          agent_action_name: "pre_read_code",
          tool_info: { file_path: path.join(consumer, "big.txt") },
        }),
        encoding: "utf8",
      },
    );
    expect(hook.status).toBe(2);
    expect(hook.stderr).toContain("bulk-read");
  }, 120_000);
});
