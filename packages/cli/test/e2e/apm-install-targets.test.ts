import { spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  apmAvailable,
  apmInstall,
  bundledCli,
  bundleRoot,
  generatedConfigPaths,
  makeConsumerLayout,
} from "./apm-fixture.js";

// non-windsurf target の table-driven 検証。windsurf は
// apm-install.test.ts が網羅済みのため対象外（DESIGN-0018 §2）。
const HOOK_TARGETS = ["copilot", "codex", "claude", "cursor"] as const;
type HookTarget = (typeof HOOK_TARGETS)[number];

const SKILL_NAMES = [
  "cheaper-usage",
  "cheaper-bulk-read",
  "cheaper-code-write",
  "cheaper-doc-write",
] as const;

type JsonObject = Record<string, unknown>;

function readJson(file: string): JsonObject {
  return JSON.parse(readFileSync(file, "utf8")) as JsonObject;
}

function runCli(cli: string, args: string[], cwd: string) {
  return spawnSync(process.execPath, [cli, ...args], { cwd, encoding: "utf8" });
}

describe.each(HOOK_TARGETS)("APM consumer install smoke (%s target)", (target: HookTarget) => {
  let root: string;
  let consumer: string;

  beforeEach(() => {
    if (!apmAvailable()) return;
    root = mkdtempSync(path.join(tmpdir(), `cheaper-apm-${target}-`));
    ({ consumer } = makeConsumerLayout(root, target));
  });
  afterEach(() => {
    if (root) rmSync(root, { recursive: true, force: true });
  });

  it("deploys the hook bundle, registers pre_read_code verbatim, and runs the bundled CLI", async (t) => {
    if (!apmAvailable()) {
      t.skip("apm CLI not available on PATH");
      return;
    }
    const install = apmInstall(consumer, root, target);
    expect(install.status, install.stderr).toBe(0);

    // bundle layout（copilot 差分は後述）
    const bundle = bundleRoot(consumer, target);
    expect(bundle).not.toBeNull();
    for (const rel of [
      "pre-read-code.js",
      path.join("cli", "bin", "cheaper.mjs"),
      path.join("cli", "workers"),
      path.join("cli", "policy.yaml"),
    ]) {
      expect(existsSync(path.join(bundle as string, rel)), `${target}: ${rel}`).toBe(true);
    }
    if (target === "copilot") {
      // DESIGN-0018 §3 との差分を記録: APM 0.28.0 は copilot の scripts
      // deploy から *.json を除外するため cli/version.json（と
      // package.json / workers/schemas/*.json）は deploy されない。
      expect(existsSync(path.join(bundle as string, "cli", "version.json"))).toBe(false);
    } else {
      expect(existsSync(path.join(bundle as string, "cli", "version.json"))).toBe(true);
    }

    // generated config: 存在 + JSON parse 可能
    const configPaths = generatedConfigPaths(consumer, target);
    expect(configPaths.length, `${target}: generated config present`).toBeGreaterThan(0);
    for (const p of configPaths) {
      expect(existsSync(p), `${target}: ${p}`).toBe(true);
    }

    // 登録 event key が pre_read_code verbatim であることの記録
    // （non-windsurf では発火しない documented no-op — ADR-0016）。
    if (target === "claude") {
      // .claude/settings.json: hooks → <event> → matcher → hooks の nested shape
      const settingsPath = configPaths.find((p) => p.endsWith("settings.json"));
      const apmHooksPath = configPaths.find((p) => p.endsWith("apm-hooks.json"));
      expect(settingsPath).toBeDefined();
      expect(apmHooksPath).toBeDefined();
      const settings = readJson(settingsPath as string);
      const apmHooks = readJson(apmHooksPath as string);
      const settingsEvents = settings.hooks as JsonObject;
      const settingsEntry = (settingsEvents.pre_read_code as JsonObject[])[0];
      expect(settingsEntry).toBeDefined();
      expect(settingsEntry.matcher).toBeDefined();
      const inner = settingsEntry.hooks as JsonObject[];
      expect(Array.isArray(inner)).toBe(true);
      // ${CLAUDE_PROJECT_DIR} 補間はそのまま残る
      expect(String(inner[0].command)).toContain("pre-read-code.js");
      // .claude/apm-hooks.json は top-level event key + 同じ nested shape
      const apmEntry = (apmHooks.pre_read_code as JsonObject[])[0];
      expect(apmEntry).toBeDefined();
      expect(apmEntry.matcher).toBeDefined();
      expect(Array.isArray(apmEntry.hooks)).toBe(true);
    } else {
      // codex / cursor / copilot: <config>.hooks.<event>
      const cfg = readJson(configPaths[0]);
      const events = cfg.hooks as JsonObject;
      const entries = events.pre_read_code as JsonObject[];
      expect(Array.isArray(entries), `${target}: hooks.pre_read_code`).toBe(true);
      expect(entries.length).toBeGreaterThan(0);
      expect(String(entries[0].command)).toContain("pre-read-code.js");
      if (target === "cursor" || target === "copilot") {
        expect(cfg.version).toBe(1);
      }
    }

    // bundled CLI が consumer workspace で動く
    const cli = bundledCli(consumer, target);
    expect(cli).not.toBeNull();
    writeFileSync(
      path.join(consumer, "sample.txt"),
      "lorem ipsum dolor sit amet ".repeat(40),
      "utf8",
    );
    const count = runCli(
      cli as string,
      ["count-tokens", path.join(consumer, "sample.txt"), "--cwd", consumer],
      consumer,
    );
    expect(count.status, count.stderr).toBe(0);

    const version = runCli(cli as string, ["--version"], consumer);
    if (target === "copilot") {
      // version.json が deploy されないため --version は version 解決に失敗
      // する（現状の deploy gap を記録。count-tokens は上で動作確認済み）。
      expect(version.status).not.toBe(0);
      expect(version.stdout + version.stderr).toContain("failed to resolve package version");
    } else {
      expect(version.status, version.stderr).toBe(0);
      expect(version.stdout.trim()).toMatch(/^\d+\.\d+\.\d+$/);
    }
  }, 120_000);
});

describe("APM consumer install smoke (agent-skills target)", () => {
  const target = "agent-skills";
  let root: string;
  let consumer: string;

  beforeEach(() => {
    if (!apmAvailable()) return;
    root = mkdtempSync(path.join(tmpdir(), "cheaper-apm-agent-skills-"));
    ({ consumer } = makeConsumerLayout(root, target));
  });
  afterEach(() => {
    if (root) rmSync(root, { recursive: true, force: true });
  });

  it("deploys the 4 skills and no hook bundle or hook config", async (t) => {
    if (!apmAvailable()) {
      t.skip("apm CLI not available on PATH");
      return;
    }
    const install = apmInstall(consumer, root, target);
    expect(install.status, install.stderr).toBe(0);

    const skillsDir = path.join(consumer, ".agents", "skills");
    for (const s of SKILL_NAMES) {
      expect(readdirSync(path.join(skillsDir, s)), `skill ${s} deployed`).toContain("SKILL.md");
    }

    // hook deploy artifact は生成されない（skills-only deploy が正しい挙動）。
    // 不存在 assert は known bundle dir / generated config file の具体 path
    // に限定し、空 dir 生成等の apm 挙動変化には結合しない（DESIGN-0018 risks）。
    expect(bundleRoot(consumer, target)).toBeNull();
    expect(generatedConfigPaths(consumer, target)).toEqual([]);
    for (const ht of ["windsurf", ...HOOK_TARGETS]) {
      const b = bundleRoot(consumer, ht);
      if (b) {
        expect(existsSync(b), `${ht} bundle dir absent`).toBe(false);
      }
      for (const cfg of generatedConfigPaths(consumer, ht)) {
        expect(existsSync(cfg), `${ht} config ${cfg} absent`).toBe(false);
      }
    }
  }, 120_000);
});
