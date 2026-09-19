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

// hook 対象 target の table-driven 検証。windsurf は
// apm-install.test.ts が hooks.json + hook runtime 挙動を網羅済み
// だが、generated config の完全性（apm-hooks.json を含む）は本 file
// の windsurf case がカバーする（DESIGN-0018 §2）。
const HOOK_TARGETS = ["windsurf", "copilot", "codex", "claude", "cursor"] as const;
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

function runCli(cli: string, args: string[], cwd: string, input?: string) {
  return spawnSync(process.execPath, [cli, ...args], { cwd, encoding: "utf8", input });
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
      "routing-policy.js",
      "pre-read-code.js",
      "pre-run-command.js",
      "pre-read-code-copilot.js",
      "pre-read-code-pretooluse.js",
      "pre-read-code-cursor.js",
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
      // worker の .md 本体は残る。
      expect(existsSync(path.join(bundle as string, "cli", "version.json"))).toBe(false);
      expect(existsSync(path.join(bundle as string, "cli", "package.json"))).toBe(false);
      expect(existsSync(path.join(bundle as string, "cli", "workers", "bulk-reader.md"))).toBe(
        true,
      );
      for (const rel of [
        path.join("cli", "workers", "schemas", "bulk-reader.input.schema.json"),
        path.join("cli", "workers", "schemas", "bulk-reader.output.schema.json"),
      ]) {
        expect(existsSync(path.join(bundle as string, rel)), `${target}: ${rel}`).toBe(false);
      }
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
    // （windsurf では native event として発火。non-windsurf では
    // documented contract 上 expected no-op。runtime loader の
    // 挙動は実機未検証 — ADR-0016）。
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
      // apm の claude adapter は event 名を native casing に正規化する:
      // `preToolUse`（camelCase）は PascalCase `PreToolUse` に変換される。
      // flat entry の matcher は lowercase `view|bash|powershell|shell`
      // に限定済みで claude の tool 名（Read/Bash/…）には一致しないため
      // copilot adapter は発火しない。claude native の Read/Bash/
      // PowerShell は下の nested group が intercept する。
      const preToolEntry = (settingsEvents.PreToolUse as JsonObject[])[0];
      expect(preToolEntry).toBeDefined();
      expect(preToolEntry.matcher).toBe("view|bash|powershell|shell");
      expect(String((preToolEntry.hooks as JsonObject[])[0].command)).toContain(
        "pre-read-code-copilot.js",
      );
      // apm-hooks.json 側も同じく PascalCase に正規化される
      const apmPreToolEntry = (apmHooks.PreToolUse as JsonObject[])[0];
      expect(apmPreToolEntry).toBeDefined();
      expect(apmPreToolEntry.matcher).toBeDefined();
      // native PreToolUse group（DESIGN-034rA79Uog7JOa7evw6rOL）:
      // matcher Read|Bash|PowerShell で pre-read-code-pretooluse.js が
      // live 登録される。copilot 由来の flat entry は lowercase matcher
      // `view|bash|powershell|shell` を保持し、claude の capitalized
      // tool 名には一致しない（cross-fire 防止）。
      const preToolGroups = settingsEvents.PreToolUse as JsonObject[];
      const nativeGroup = preToolGroups.find((g) => g.matcher === "Read|Bash|PowerShell");
      expect(nativeGroup).toBeDefined();
      expect(String(((nativeGroup as JsonObject).hooks as JsonObject[])[0].command)).toContain(
        "pre-read-code-pretooluse.js",
      );
      const copilotGroup = preToolGroups.find((g) => g.matcher === "view|bash|powershell|shell");
      expect(copilotGroup).toBeDefined();
    } else {
      // windsurf / codex / cursor / copilot: hooks.json（copilot は
      // dep 名由来の <dep>-hooks.json）は `hooks` 直下に event を
      // nest する shape。windsurf / codex / cursor では apm が同じ
      // 登録を top-level event key shape の apm-hooks.json としても
      // 追加生成する（DISC-0009 観測事実）。
      for (const cfgPath of configPaths) {
        const cfg = readJson(cfgPath);
        const flat = path.basename(cfgPath) === "apm-hooks.json";
        const entries = (
          flat ? cfg.pre_read_code : (cfg.hooks as JsonObject).pre_read_code
        ) as JsonObject[];
        expect(Array.isArray(entries), `${target}: ${cfgPath}`).toBe(true);
        expect(entries.length).toBeGreaterThan(0);
        expect(String(entries[0].command)).toContain("pre-read-code.js");
        // preToolUse（copilot native event）の登録も verbatim で全
        // target config に入る。copilot では functional、他 target
        // では unknown event または matcher 不一致として no-op
        //（ADR-0023 / DESIGN-034rA79Uog7JOa7evw6rOL）。
        const preToolEntries = (
          flat ? cfg.preToolUse : (cfg.hooks as JsonObject).preToolUse
        ) as JsonObject[];
        expect(Array.isArray(preToolEntries), `${target}: ${cfgPath} preToolUse`).toBe(true);
        expect(preToolEntries.length).toBeGreaterThan(0);
        expect(String(preToolEntries[0].command)).toContain("pre-read-code-copilot.js");
        expect(preToolEntries[0].matcher).toBe("view|bash|powershell|shell");
        if (!flat && (target === "cursor" || target === "copilot")) {
          expect(cfg.version).toBe(1);
        }
        // per-target native event 登録（DESIGN-034rA79Uog7JOa7evw6rOL）。
        // flat entry は `command`、nested matcher group は
        // `hooks[].command` に script path を持つ。
        const hooksObj = flat ? cfg : (cfg.hooks as JsonObject);
        const entryCommand = (entry: JsonObject) =>
          typeof entry.command === "string"
            ? String(entry.command)
            : String(((entry.hooks as JsonObject[]) ?? [{}])[0].command);
        const nativeEvents: Record<string, Array<[string, string]>> = {
          windsurf: [["pre_run_command", "pre-run-command.js"]],
          // copilot では nested PreToolUse group が preToolUse array に
          // merge されて現れる（matcher が lowercase tool 名に一致
          // しないため実質 no-op — 登録形状の記録として assert）。
          copilot: [["preToolUse", "pre-read-code-pretooluse.js"]],
          codex: [["PreToolUse", "pre-read-code-pretooluse.js"]],
          cursor: [
            ["beforeReadFile", "pre-read-code-cursor.js"],
            ["beforeShellExecution", "pre-read-code-cursor.js"],
          ],
        };
        for (const [event, script] of nativeEvents[target] ?? []) {
          const evEntries = hooksObj[event] as JsonObject[];
          expect(Array.isArray(evEntries), `${target}: ${cfgPath} ${event}`).toBe(true);
          expect(evEntries.length).toBeGreaterThan(0);
          expect(
            evEntries.some((e) => entryCommand(e).includes(script)),
            `${target}: ${cfgPath} ${event} -> ${script}`,
          ).toBe(true);
        }
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
      // version.json が deploy されないため --version は exit 70 で
      // version 解決に失敗する（現状の deploy gap を記録。
      // count-tokens は上で動作確認済み）。
      expect(version.status).toBe(70);
      expect(version.stdout + version.stderr).toContain("failed to resolve package version");

      // schemas/*.json の欠落は file-schema worker も壊す: worker
      // definition load が provider/model 呼出し前に schema 欠落で
      // 失敗するため auth なしで決定的に再現する。実測では schema が
      // output → input の順に load されるため OUTPUT_SCHEMA_NOT_FOUND
      // (exit 3) が先に出る。
      const bulk = runCli(
        cli as string,
        ["bulk-read", "--cwd", consumer],
        consumer,
        JSON.stringify({ task: "t", paths: ["sample.txt"] }),
      );
      expect(bulk.status, bulk.stderr).toBe(3);
      const env = JSON.parse(bulk.stdout) as JsonObject;
      expect(env.ok).toBe(false);
      expect((env.error as JsonObject).code).toBe("OUTPUT_SCHEMA_NOT_FOUND");
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
    for (const ht of HOOK_TARGETS) {
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
