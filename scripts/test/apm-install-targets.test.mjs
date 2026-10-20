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
// issue #118 で hook 宣言は per-target sibling package(apm-hook-*)に
// 分離 — 各 target config にはその target の native event のみが
// 登録され、他 harness の event key は一切残らない
// (SPEC-2w4XxXeDF0glTfgrBVJnWb / ADR-1fVxZhjOm4z8ilPCUSXHMI)。

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
  TARGET_NATIVE_EVENTS,
} from "./lib/apm-fixture.mjs";

const HOOK_TARGETS = ["windsurf", "copilot", "codex", "claude", "cursor"];
const SKILL_NAMES = [
  "cheaper-usage",
  "cheaper-bulk-read",
  "cheaper-code-write",
  "cheaper-doc-write",
  "cheaper-tool-operate",
];

// 全 native event の集合 — foreign event 不在 assert の対照。
const ALL_EVENTS = new Set(Object.values(TARGET_NATIVE_EVENTS).flat());

const readJson = (file) => JSON.parse(readFileSync(file, "utf8"));
const apmMissing = !apmAvailable();

// event entry が flat(`command` 直下)でも nested(`hooks[].command`)でも
// command 文字列を取り出す。
const entryCommand = (entry) =>
  typeof entry.command === "string"
    ? String(entry.command)
    : String((entry.hooks ?? [{}])[0].command);

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

    it("deploys the binary-free hook bundle and registers native events only", {
      timeout: 120_000,
      skip: apmMissing && "apm CLI not available on PATH",
    }, () => {
      const install = apmInstall(consumer, root, target);
      assert.strictEqual(install.status, 0, install.stderr);

      // bundle layout — slim payload(issue #85): thin launcher + hooks.json
      // のみ。cli/ payload(version.json / workers / policy / bin)は
      // cargo binary の embedded resource に統合されたため配布しない。
      // deploy dir 名は hook package 名(apm-hook-*、issue #118)。
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

      const nativeEvents = TARGET_NATIVE_EVENTS[target];
      const expectedScript = {
        windsurf: /hook (pre-read-code|pre-run-command) --target windsurf/,
        cursor: /hook (before-read-file|before-shell-execution) --target cursor/,
        copilot: /hook pre-tool-use --target copilot/,
        claude: /hook pre-tool-use --target pretooluse/,
        codex: /hook pre-tool-use --target codex/,
      }[target];

      // 全 generated config で event key が native event のみで、
      // 登録 entry が対応 package の launcher を指すことを確認。
      for (const cfgPath of configPaths) {
        const cfg = readJson(cfgPath);
        // apm-hooks.json のみ top-level event key の flat shape。
        // copilot の <dep>-hooks.json や harness 本来の hooks.json /
        // settings.json は `hooks` 直下に event を nest する。
        const flat = path.basename(cfgPath) === "apm-hooks.json";
        const eventsObj = flat ? cfg : cfg.hooks;
        const eventKeys = Object.keys(eventsObj).filter((k) => ALL_EVENTS.has(k));
        assert.deepEqual(
          eventKeys.sort(),
          [...nativeEvents].sort(),
          `${target}: ${cfgPath} event keys`,
        );
        for (const event of nativeEvents) {
          const entries = eventsObj[event];
          assert.ok(Array.isArray(entries) && entries.length > 0, `${target}: ${cfgPath} ${event}`);
          assert.ok(
            entries.some((e) => expectedScript.test(entryCommand(e))),
            `${target}: ${cfgPath} ${event} -> ${expectedScript}`,
          );
          assert.doesNotMatch(
            JSON.stringify(entries),
            /node /,
            `${target}: ${cfgPath} ${event} must not invoke node`,
          );
        }
      }

      // claude: PreToolUse は native pretooluse entry 1 件のみ —
      // copilot 由来 flat entry(view|bash|powershell|shell matcher)の
      // 重複登録が無いこと(issue #118 の副次不具合が消えたことの pin)。
      if (target === "claude") {
        const settingsPath = configPaths.find((p) => p.endsWith("settings.json"));
        const settings = readJson(settingsPath);
        const groups = settings.hooks.PreToolUse;
        assert.strictEqual(groups.length, 1, "claude PreToolUse must have exactly one group");
        assert.strictEqual(
          groups[0].matcher,
          "^([Rr][Ee][Aa][Dd]|[Gg][Rr][Ee][Pp]|[Bb][Aa][Ss][Hh]|[Pp][Oo][Ww][Ee][Rr][Ss][Hh][Ee][Ll][Ll])$",
        );
        assert.match(String(groups[0].hooks[0].command), /hook pre-tool-use --target pretooluse/);
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

  it("deploys the skills and no hook bundle or hook config", {
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
