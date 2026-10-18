// Slim APM payload contract — issue #85 / ADR-034Dmb3kNAO4Ub6FxuQCtC、
// issue #118(per-target hook package split / ADR-1fVxZhjOm4z8ilPCUSXHMI)。
// APM package は launcher + hooks.json + skills のみを配布する。
// executable payload は一切含まず、CLI は consumer が
// `cargo install --git … --locked cheaper-cli` で PATH 上に用意する。
// 検証項目:
//   - hook package は `packages/*/.apm/hooks/` を glob で発見
//     (hardcoded list にしない — 追加漏れを検査で検出する)
//   - 各 hook package の必須 nonempty asset: cheaper.sh / cheaper.ps1 /
//     hooks.json
//   - packages/apm に .apm/hooks/ が存在しないこと(hook 宣言は
//     per-target sibling package の責務)
//   - cli/ ディレクトリの不在(旧 acquisition payload の完全撤去)
//   - hooks/ 内 file の拡張子 allowlist + PE/ELF magic-byte 検出
//   - byte-level 契約: cheaper.ps1 = UTF-8 BOM + CRLF、
//     cheaper.sh = #!/bin/sh shebang + LF
//   - git index mode cheaper.sh = 100755(index 情報がある場合)
//   - launcher copy 間の byte 一致(4 package の cheaper.sh 同士・
//     cheaper.ps1 同士が同一 — 片方だけ更新される drift を防ぐ)
//   - wiring: 親 packages/apm/apm.yml の dependencies.apm が全 hook
//     package を path dep + 整合する targets: で宣言していること
//   - hooks.json ↔ launcher 整合: POSIX entry は `sh ./cheaper.sh`、
//     powershell entry は `./cheaper.ps1` を参照し、bare `./cheaper` なし
import { execFileSync } from "node:child_process";
import { existsSync, lstatSync, readdirSync, readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const REQUIRED_ASSETS = ["cheaper.sh", "cheaper.ps1", "hooks.json"];
const EXTENSION_ALLOWLIST = new Set([".sh", ".ps1", ".json", ".md", ".yaml", ".txt"]);
const PE_MAGIC = Buffer.from([0x4d, 0x5a]); // "MZ"
const ELF_MAGIC = Buffer.from([0x7f, 0x45, 0x4c, 0x46]); // "\x7fELF"

function* walkFiles(dir) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const file = path.join(dir, entry.name);
    if (entry.isDirectory()) yield* walkFiles(file);
    else if (entry.isFile()) yield file;
  }
}

function sniffExecutable(bytes) {
  if (bytes.subarray(0, 2).equals(PE_MAGIC)) return "PE (Windows executable)";
  if (bytes.subarray(0, 4).equals(ELF_MAGIC)) return "ELF (POSIX executable)";
  return null;
}

// packages/ 直下で `.apm/hooks/` を持つ package を発見する
// (glob — 新 hook package の追加が検査対象外になることを防ぐ)。
function findHookPackages(repoRoot) {
  const packagesDir = path.join(repoRoot, "packages");
  if (!existsSync(packagesDir)) return [];
  return readdirSync(packagesDir, { withFileTypes: true })
    .filter((d) => d.isDirectory())
    .map((d) => d.name)
    .filter((name) => existsSync(path.join(packagesDir, name, ".apm", "hooks")))
    .sort();
}

// minimal YAML 抽出 — apm.yml の `target:`/`targets:`/`dependencies.apm`
// entry を line ベースで読む(依存を増やさない既存方針に合わせる)。
function manifestTargets(text) {
  const single = /^target:\s*([A-Za-z-]+)\s*$/m.exec(text)?.[1];
  const plural = /^targets:\s*\[([^\]]*)\]\s*$/m.exec(text)?.[1];
  const targets = [];
  if (single) targets.push(single);
  if (plural)
    for (const t of plural.split(",")) {
      const name = t.trim().replace(/^["']|["']$/g, "");
      if (name) targets.push(name);
    }
  return targets.sort();
}

// dependencies.apm の `- path: <p>` + `targets: [...]` entry を抽出。
// line ベース: `dependencies:` → `apm:` 直下の entry 群を走査する。
function manifestDepEdges(text) {
  const edges = [];
  const lines = text.split(/\r?\n/);
  let inApmDeps = false;
  for (const line of lines) {
    if (/^dependencies:/.test(line)) {
      inApmDeps = "pending";
      continue;
    }
    if (inApmDeps === "pending") {
      if (/^\s+apm:/.test(line)) {
        inApmDeps = true;
        continue;
      }
      if (/^\S/.test(line)) break; // dependencies 直後に別 group
      continue;
    }
    if (!inApmDeps) continue;
    if (/^\S/.test(line)) break; // top-level key で section 終了
    const dep = /^\s+-\s*path:\s*(\S+)/.exec(line);
    if (dep) {
      edges.push({ path: dep[1], targets: [] });
      continue;
    }
    const tg = /^\s+(targets?:)\s*(.+)$/.exec(line);
    if (tg && edges.length) edges[edges.length - 1].targets = manifestTargets(`${tg[1]} ${tg[2]}`);
  }
  return edges;
}

function checkHookPackage(repoRoot, pkgName, errors) {
  const relBase = `packages/${pkgName}/.apm/hooks`;
  const hooksDir = path.join(repoRoot, relBase);

  for (const asset of REQUIRED_ASSETS) {
    const relative = `${relBase}/${asset}`;
    const file = path.join(repoRoot, relative);
    try {
      const stat = lstatSync(file);
      if (!stat.isFile() || stat.size === 0) {
        errors.push(`asset must be a nonempty regular file: ${relative}`);
        continue;
      }
      const text = readFileSync(file, "utf8");
      if (text.startsWith("version https://git-lfs")) {
        errors.push(`asset is an LFS pointer: ${relative}`);
        continue;
      }
      if (asset.endsWith(".json")) {
        try {
          JSON.parse(text);
        } catch {
          errors.push(`invalid JSON: ${relative}`);
        }
      }
    } catch (error) {
      errors.push(`asset missing or unreadable: ${relative} (${error.message})`);
    }
  }

  const cliDir = path.join(hooksDir, "cli");
  if (existsSync(cliDir))
    errors.push(`${relBase}: cli/ payload must not exist — CLI is cargo-installed, not bundled`);

  // 拡張子 allowlist + binary sniffing(POSIX mode では判定しない —
  // cheaper.sh は legitimately 100755)。
  for (const file of walkFiles(hooksDir)) {
    const relative = path.relative(repoRoot, file).split(path.sep).join("/");
    const ext = path.extname(file);
    if (ext !== "" && !EXTENSION_ALLOWLIST.has(ext))
      errors.push(`file extension not allowed in hooks payload: ${relative} (${ext})`);
    const kind = sniffExecutable(readFileSync(file));
    if (kind) errors.push(`executable payload is forbidden: ${relative} (${kind})`);
  }

  // Byte-level launcher 契約。
  const ps1 = path.join(hooksDir, "cheaper.ps1");
  if (existsSync(ps1)) {
    const bytes = readFileSync(ps1);
    if (!bytes.subarray(0, 3).equals(Buffer.from([0xef, 0xbb, 0xbf])))
      errors.push(
        `${relBase}/cheaper.ps1 must start with UTF-8 BOM (EF BB BF) for Windows PowerShell 5.1`,
      );
    if (!bytes.includes("\r\n"))
      errors.push(`${relBase}/cheaper.ps1 must use CRLF line endings for Windows PowerShell 5.1`);
  }
  const sh = path.join(hooksDir, "cheaper.sh");
  if (existsSync(sh)) {
    const bytes = readFileSync(sh);
    if (!bytes.toString("utf8", 0, 9).startsWith("#!/bin/sh"))
      errors.push(`${relBase}/cheaper.sh must start with a #!/bin/sh shebang`);
    if (bytes.includes("\r\n"))
      errors.push(`${relBase}/cheaper.sh must use LF line endings (CR detected)`);
    try {
      const mode = execFileSync("git", ["ls-files", "-s", `${relBase}/cheaper.sh`], {
        cwd: repoRoot,
        encoding: "utf8",
      }).match(/^(\d+)/)?.[1];
      if (mode && mode !== "100755")
        errors.push(`${relBase}/cheaper.sh git index mode must be 100755, got ${mode}`);
    } catch {
      // index 情報が無い環境(export 済み snapshot 等)では skip。
    }
  }

  // hooks.json ↔ launcher 整合。
  try {
    const hooksJsonPath = path.join(hooksDir, "hooks.json");
    const hooks = JSON.parse(readFileSync(hooksJsonPath, "utf8"));
    const text = readFileSync(hooksJsonPath, "utf8");
    if (/(?:^|[\s"'])\.\/cheaper(?![.\w])/m.test(text))
      errors.push(
        `${relBase}/hooks.json must not reference bare "./cheaper" (use sh ./cheaper.sh or ./cheaper.ps1)`,
      );
    const entries = [];
    const collect = (value) => {
      if (Array.isArray(value)) for (const item of value) collect(item);
      else if (value && typeof value === "object") {
        if (typeof value.command === "string") entries.push(value);
        for (const nested of Object.values(value)) if (typeof nested === "object") collect(nested);
      }
    };
    collect(hooks.hooks ?? {});
    for (const entry of entries) {
      if (!entry.command.includes("sh ./cheaper.sh"))
        errors.push(`${relBase}: POSIX command must invoke "sh ./cheaper.sh": ${entry.command}`);
      if (typeof entry.powershell === "string" && !entry.powershell.includes("./cheaper.ps1"))
        errors.push(
          `${relBase}: powershell entry must invoke "./cheaper.ps1": ${entry.powershell}`,
        );
    }
  } catch (error) {
    errors.push(`${relBase}/hooks.json unreadable or inconsistent: ${error.message}`);
  }
}

export function checkApmPayload(repoRoot) {
  const errors = [];
  const hookPackages = findHookPackages(repoRoot);

  // packages/apm は skills-only — hook 宣言を持たない。
  if (existsSync(path.join(repoRoot, "packages/apm/.apm/hooks")))
    errors.push(
      "packages/apm/.apm/hooks must not exist — hook declarations live in per-target apm-hook-* packages (issue #118)",
    );
  if (hookPackages.length === 0)
    errors.push(
      "no hook package found under packages/*/.apm/hooks — apm-hook-* packages are missing",
    );

  for (const pkg of hookPackages) checkHookPackage(repoRoot, pkg, errors);

  // launcher copy 間の byte 一致(drift guard)。
  for (const launcher of ["cheaper.sh", "cheaper.ps1"]) {
    const contents = new Map();
    for (const pkg of hookPackages) {
      const file = path.join(repoRoot, "packages", pkg, ".apm", "hooks", launcher);
      if (existsSync(file)) contents.set(pkg, readFileSync(file));
    }
    const variants = new Set([...contents.values()].map((b) => b.toString("base64")));
    if (variants.size > 1)
      errors.push(
        `${launcher} copies diverge across hook packages: ${[...contents.keys()].join(", ")}`,
      );
  }

  // wiring: 親 apm.yml が発見した hook package を全て dep 宣言し、
  // dep-edge targets が package manifest の target と一致すること。
  const parentManifest = path.join(repoRoot, "packages/apm/apm.yml");
  if (!existsSync(parentManifest)) {
    errors.push("packages/apm/apm.yml missing");
    return errors;
  }
  const parentText = readFileSync(parentManifest, "utf8");
  const edges = manifestDepEdges(parentText);
  for (const pkg of hookPackages) {
    const pkgText = readFileSync(path.join(repoRoot, "packages", pkg, "apm.yml"), "utf8");
    const pkgTargets = manifestTargets(pkgText);
    const edge = edges.find((e) => e.path.endsWith(`/${pkg}`));
    if (!edge) {
      errors.push(`packages/apm/apm.yml dependencies.apm is missing a path dep for ${pkg}`);
      continue;
    }
    if (JSON.stringify(edge.targets) !== JSON.stringify(pkgTargets))
      errors.push(
        `packages/apm/apm.yml dep-edge targets [${edge.targets}] do not match ${pkg} manifest targets [${pkgTargets}]`,
      );
  }

  return errors;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const errors = checkApmPayload(fileURLToPath(new URL("..", import.meta.url)));
  if (errors.length) {
    console.error(errors.join("\n"));
    process.exitCode = 1;
  } else console.log("APM slim payload OK (CLI is cargo-installed on PATH)");
}
