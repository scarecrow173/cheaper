import {
  existsSync,
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
import { apmAvailable, apmInstall, apmPackageDir } from "./apm-fixture.js";
import { cliRoot } from "./helpers.js";

const repoRoot = path.join(cliRoot, "..", "..");
const rootApmYml = path.join(repoRoot, "apm.yml");

// Issue #9 / DESIGN-0021: repository root は development-only APM
// project、consumer へ配布する primitives は packages/apm の nested
// package に閉じる。root install が cheaper own skill/hook を deploy
// しないことを構造と挙動の 2 層で保証する。

describe("development / distributable APM package separation", () => {
  it("keeps no distributable .apm/ primitives at the repository root", () => {
    expect(existsSync(path.join(repoRoot, ".apm"))).toBe(false);
  });

  it("ships skills and hooks in the nested distributable package", () => {
    expect(existsSync(path.join(apmPackageDir, "apm.yml"))).toBe(true);
    expect(existsSync(path.join(apmPackageDir, ".apm", "hooks"))).toBe(true);
    expect(existsSync(path.join(apmPackageDir, ".apm", "skills"))).toBe(true);
    const skills = readdirSync(path.join(apmPackageDir, ".apm", "skills"));
    for (const s of [
      "cheaper-usage",
      "cheaper-bulk-read",
      "cheaper-code-write",
      "cheaper-doc-write",
    ]) {
      expect(skills).toContain(s);
    }
  });

  it("declares the root project as development-only", () => {
    const yml = readFileSync(rootApmYml, "utf8");
    expect(yml).toMatch(/^name: cheaper-dev$/m);
    // `includes` は primitives source discovery の宣言であり、dev-only
    // project には不要（.apm/ も root-level source dir も存在しない）。
    expect(yml).not.toMatch(/^includes:/m);
    // 自身の distribution package を dependency として持たない。
    expect(yml).not.toMatch(/cheaper\/packages\/apm/);
    expect(yml).not.toMatch(/path:\s*\.?\.?\/?packages\/apm/);
  });

  it("declares the distributable package name for consumers", () => {
    const yml = readFileSync(path.join(apmPackageDir, "apm.yml"), "utf8");
    expect(yml).toMatch(/^name: cheaper$/m);
  });

  it("documents remote dependency refs with the APM #ref grammar", () => {
    // APM v0.28.0 の manifest grammar は
    // `owner/repo[/virtual_path][#ref]`。`@ref` は invalid なので、
    // consumer-facing の記述が `@` 形式へ drift しないことを静的に
    // guard する。
    const docs = [
      "README.md",
      "docs/guides/parent-agent-integration.md",
      "docs/adr/0019-separate-dev-apm-project-via-nested-distributable-package.md",
      "docs/designs/0021-dev-apm-project-and-distributable-package-layout.md",
    ];
    const refs: string[] = [];
    for (const rel of docs) {
      const p = path.join(repoRoot, rel);
      if (!existsSync(p)) continue;
      const text = readFileSync(p, "utf8");
      expect(text, rel).not.toMatch(/packages\/apm@/);
      refs.push(...(text.match(/packages\/apm#[\w./-]*/g) ?? []));
    }
    expect(refs.length).toBeGreaterThan(0);
  });
});

describe("root development project install does not self-deploy", () => {
  let dir: string;
  let project: string;

  beforeEach(() => {
    if (!apmAvailable()) return;
    dir = mkdtempSync(path.join(tmpdir(), "cheaper-dev-apm-"));
    project = path.join(dir, "project");
  });
  afterEach(() => {
    if (dir) rmSync(dir, { recursive: true, force: true });
  });

  it("deploys no cheaper skills or hooks for a project without .apm/", (t) => {
    if (!apmAvailable()) {
      t.skip("apm CLI not available on PATH");
      return;
    }
    // root apm.yml と同形の development project（.apm/ を持たず、remote
    // dep を解決しない最小 manifest）を一時 dir に作る。
    mkdirSync(project, { recursive: true });
    writeFileSync(
      path.join(project, "apm.yml"),
      'name: cheaper-dev\nversion: "0.0.0"\ntargets:\n- windsurf\n',
      "utf8",
    );

    const install = apmInstall(project, dir);
    expect(install.status).toBe(0);

    // cheaper own skill/hook は一切 deploy されない。target dir が
    // 生成されないケースも存在として扱う。
    const skillsDir = path.join(project, ".agents", "skills");
    if (existsSync(skillsDir)) {
      expect(readdirSync(skillsDir).filter((d) => d.startsWith("cheaper-"))).toEqual([]);
    }
    const hooksJsonPath = path.join(project, ".windsurf", "hooks.json");
    if (existsSync(hooksJsonPath)) {
      const hooksJson = JSON.parse(readFileSync(hooksJsonPath, "utf8")) as {
        hooks?: Record<string, unknown>;
      };
      expect(hooksJson.hooks?.pre_read_code).toBeUndefined();
    }
    expect(existsSync(path.join(project, ".windsurf", "hooks", "pkg"))).toBe(false);
  }, 60_000);
});
