// APM consumer install target matrix — packages/cli/test/e2e/
// apm-install-targets.test.ts の node:test port(Phase 6 Commit C)。
//
// hook 対象 target の table-driven 検証。deploy layout・generated config
// の完全性・event 登録 shape を `apm install` 実実行で確認する。
// issue #85 で配布 model は cargo source install + thin PATH launcher に
// 移行 — deployed bundle は cheaper.sh / cheaper.ps1 / hooks.json /
// skills のみで cli/ payload を含まない。launcher の実行経路
// (PATH 上 CLI の passthrough)は scripts/test/launcher-passthrough.test.mjs
// が担う。

import assert from "node:assert/strict";
import { existsSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, beforeEach, describe, it } from "node:test";
import {
  apmAvailable,
  apmInstall,
  bundleRoot,
  generatedConfigPaths,
  makeConsumerLayout,
} from "./lib/apm-fixture.mjs";

const HOOK_TARGETS = ["windsurf", "copilot", "codex", "claude", "cursor"];
const SKILL_NAMES = [
  "cheaper-usage",
  "cheaper-bulk-read",
  "cheaper-code-write",
  "cheaper-doc-write",
];

const readJson = (file) => JSON.parse(readFileSync(file, "utf8"));
const apmMissing = !apmAvailable();

for (const target of HOOK_TARGETS) {
  describe(`APM consumer install smoke (${target} target)`, () => {
    let root;
    let consumer;

    beforeEach(() => {
      if (apmMissing) return;
      root = mkdtempSync(path.join(tmpdir(), `cheaper-apm-${target}-`));
      ({ consumer } = makeConsumerLayout(root, target));
    });
    afterEach(() => {
      if (root) rmSync(root, { recursive: true, force: true });
    });

    it("deploys the binary-free hook bundle and registers pre_read_code verbatim", {
      timeout: 120_000,
      skip: apmMissing && "apm CLI not available on PATH",
    }, () => {
      const install = apmInstall(consumer, root, target);
      assert.strictEqual(install.status, 0, install.stderr);

      // bundle layout — slim payload(issue #85): thin launcher + hooks.json
      // のみ。cli/ payload(version.json / workers / policy / bin)は
      // cargo binary の embedded resource に統合されたため配布しない。
      const bundle = bundleRoot(consumer, target);
      assert.notStrictEqual(bundle, null);
      for (const rel of ["cheaper.sh", "cheaper.ps1"]) {
        assert.ok(existsSync(path.join(bundle, rel)), `${target}: ${rel}`);
      }
      assert.strictEqual(
        existsSync(path.join(bundle, "cli")),
        false,
        `${target}: cli/ payload must not be deployed`,
      );
      // POSIX platform では deploy 済み cheaper.sh の exec bit を観測する
      // (Windows の mode は POSIX bit を持たないため conditional)。
      // hooks.json は `sh ./cheaper.sh` で起動するため bit 喪失でも
      // hook は動くが、直接実行できることは payload 契約として残す。
      if (process.platform !== "win32") {
        const mode = statSync(path.join(bundle, "cheaper.sh")).mode;
        assert.ok(mode & 0o111, `${target}: deployed cheaper.sh must be executable`);
      }

      // generated config: 存在 + JSON parse 可能
      const configPaths = generatedConfigPaths(consumer, target);
      assert.ok(configPaths.length > 0, `${target}: generated config present`);
      for (const p of configPaths) {
        assert.ok(existsSync(p), `${target}: ${p}`);
      }

      // 登録 event key が pre_read_code verbatim であることの記録
      // (windsurf では native event として発火。non-windsurf では
      // documented contract 上 expected no-op — ADR-0016)。
      if (target === "claude") {
        // .claude/settings.json: hooks → <event> → matcher → hooks の
        // nested shape
        const settingsPath = configPaths.find((p) => p.endsWith("settings.json"));
        const apmHooksPath = configPaths.find((p) => p.endsWith("apm-hooks.json"));
        assert.ok(settingsPath);
        assert.ok(apmHooksPath);
        const settings = readJson(settingsPath);
        const apmHooks = readJson(apmHooksPath);
        const settingsEvents = settings.hooks;
        const settingsEntry = settingsEvents.pre_read_code[0];
        assert.ok(settingsEntry);
        assert.ok(settingsEntry.matcher);
        const inner = settingsEntry.hooks;
        assert.ok(Array.isArray(inner));
        // ${CLAUDE_PROJECT_DIR} 補間はそのまま残る。command は
        // launcher 経由の `cheaper hook` 呼び出し。
        assert.match(String(inner[0].command), /hook pre-read-code --target windsurf/);
        assert.doesNotMatch(String(inner[0].command), /node /);
        // .claude/apm-hooks.json は top-level event key + 同じ nested shape
        const apmEntry = apmHooks.pre_read_code[0];
        assert.ok(apmEntry);
        assert.ok(apmEntry.matcher);
        assert.ok(Array.isArray(apmEntry.hooks));
        // apm の claude adapter は event 名を native casing に正規化
        // する: `preToolUse`(camelCase)は PascalCase `PreToolUse` に
        // 変換される。flat entry の matcher は lowercase
        // `view|bash|powershell|shell` に限定済みで claude の tool 名
        // (Read/Bash/…)には一致しないため copilot adapter は発火しない。
        const preToolEntry = settingsEvents.PreToolUse[0];
        assert.ok(preToolEntry);
        assert.strictEqual(preToolEntry.matcher, "view|bash|powershell|shell");
        assert.match(String(preToolEntry.hooks[0].command), /hook pre-tool-use --target copilot/);
        // apm-hooks.json 側も同じく PascalCase に正規化される
        const apmPreToolEntry = apmHooks.PreToolUse[0];
        assert.ok(apmPreToolEntry);
        assert.ok(apmPreToolEntry.matcher);
        // native PreToolUse group(DESIGN-034rA79Uog7JOa7evw6rOL):
        // matcher Read|Bash|PowerShell で pre-read-code-pretooluse
        // が live 登録される。copilot 由来の flat entry は lowercase
        // matcher を保持し、claude の capitalized tool 名には一致
        // しない(cross-fire 防止)。
        const preToolGroups = settingsEvents.PreToolUse;
        const nativeGroup = preToolGroups.find((g) => g.matcher === "Read|Bash|PowerShell");
        assert.ok(nativeGroup);
        assert.match(String(nativeGroup.hooks[0].command), /hook pre-tool-use --target pretooluse/);
        const copilotGroup = preToolGroups.find((g) => g.matcher === "view|bash|powershell|shell");
        assert.ok(copilotGroup);
      } else {
        // windsurf / codex / cursor / copilot: hooks.json(copilot は
        // dep 名由来の <dep>-hooks.json)は `hooks` 直下に event を
        // nest する shape。windsurf / codex / cursor では apm が同じ
        // 登録を top-level event key shape の apm-hooks.json としても
        // 追加生成する(DISC-0009 観測事実)。
        for (const cfgPath of configPaths) {
          const cfg = readJson(cfgPath);
          const flat = path.basename(cfgPath) === "apm-hooks.json";
          const entries = flat ? cfg.pre_read_code : cfg.hooks.pre_read_code;
          assert.ok(Array.isArray(entries), `${target}: ${cfgPath}`);
          assert.ok(entries.length > 0);
          assert.match(String(entries[0].command), /hook pre-read-code --target windsurf/);
          assert.doesNotMatch(String(entries[0].command), /node /);
          // preToolUse(copilot native event)の登録も verbatim で全
          // target config に入る。copilot では functional、他 target
          // では unknown event または matcher 不一致として no-op
          // (ADR-0023 / DESIGN-034rA79Uog7JOa7evw6rOL)。
          const preToolEntries = flat ? cfg.preToolUse : cfg.hooks.preToolUse;
          assert.ok(Array.isArray(preToolEntries), `${target}: ${cfgPath} preToolUse`);
          assert.ok(preToolEntries.length > 0);
          assert.match(String(preToolEntries[0].command), /hook pre-tool-use --target copilot/);
          assert.strictEqual(preToolEntries[0].matcher, "view|bash|powershell|shell");
          if (!flat && (target === "cursor" || target === "copilot")) {
            assert.strictEqual(cfg.version, 1);
          }
          // per-target native event 登録(DESIGN-034rA79Uog7JOa7evw6rOL)。
          // flat entry は `command`、nested matcher group は
          // `hooks[].command` に script path を持つ。
          const hooksObj = flat ? cfg : cfg.hooks;
          const entryCommand = (entry) =>
            typeof entry.command === "string"
              ? String(entry.command)
              : String((entry.hooks ?? [{}])[0].command);
          const nativeEvents = {
            windsurf: [["pre_run_command", "hook pre-run-command --target windsurf"]],
            // copilot では nested PreToolUse group が preToolUse array
            // に merge されて現れる(実質 no-op — 登録形状の記録として
            // assert)。
            copilot: [["preToolUse", "hook pre-tool-use --target pretooluse"]],
            codex: [["PreToolUse", "hook pre-tool-use --target pretooluse"]],
            cursor: [
              ["beforeReadFile", "hook before-read-file --target cursor"],
              ["beforeShellExecution", "hook before-shell-execution --target cursor"],
            ],
          };
          for (const [event, script] of nativeEvents[target] ?? []) {
            const evEntries = hooksObj[event];
            assert.ok(Array.isArray(evEntries), `${target}: ${cfgPath} ${event}`);
            assert.ok(evEntries.length > 0);
            assert.ok(
              evEntries.some((e) => entryCommand(e).includes(script)),
              `${target}: ${cfgPath} ${event} -> ${script}`,
            );
          }
        }
      }
    });
  });
}

describe("APM consumer install smoke (agent-skills target)", () => {
  const target = "agent-skills";
  let root;
  let consumer;

  beforeEach(() => {
    if (apmMissing) return;
    root = mkdtempSync(path.join(tmpdir(), "cheaper-apm-agent-skills-"));
    ({ consumer } = makeConsumerLayout(root, target));
  });
  afterEach(() => {
    if (root) rmSync(root, { recursive: true, force: true });
  });

  it("deploys the 4 skills and no hook bundle or hook config", {
    timeout: 120_000,
    skip: apmMissing && "apm CLI not available on PATH",
  }, () => {
    const install = apmInstall(consumer, root, target);
    assert.strictEqual(install.status, 0, install.stderr);

    const skillsDir = path.join(consumer, ".agents", "skills");
    for (const s of SKILL_NAMES) {
      assert.ok(readdirSync(path.join(skillsDir, s)).includes("SKILL.md"), `skill ${s} deployed`);
    }

    // hook deploy artifact は生成されない(skills-only deploy が正しい
    // 挙動)。不存在 assert は known bundle dir / generated config
    // file の具体 path に限定する(DESIGN-0018 risks)。
    assert.strictEqual(bundleRoot(consumer, target), null);
    assert.deepStrictEqual(generatedConfigPaths(consumer, target), []);
    for (const ht of HOOK_TARGETS) {
      const b = bundleRoot(consumer, ht);
      if (b) {
        assert.strictEqual(existsSync(b), false, `${ht} bundle dir absent`);
      }
      for (const cfg of generatedConfigPaths(consumer, ht)) {
        assert.strictEqual(existsSync(cfg), false, `${ht} config ${cfg} absent`);
      }
    }
  });
});
