// Slim APM payload contract — issue #85 / ADR-034Dmb3kNAO4Ub6FxuQCtC。
// APM package は launcher + hooks.json + skills のみを配布する。
// executable payload は一切含まず、CLI は consumer が
// `cargo install --git … --locked cheaper-cli` で PATH 上に用意する。
// 検証項目:
//   - 必須 nonempty asset: cheaper.sh / cheaper.ps1 / hooks.json
//   - cli/ ディレクトリの不在(旧 acquisition payload の完全撤去)
//   - hooks/ 内 file の拡張子 allowlist + PE/ELF magic-byte 検出
//   - byte-level 契約: cheaper.ps1 = UTF-8 BOM + CRLF、
//     cheaper.sh = #!/bin/sh shebang + LF
//   - git index mode cheaper.sh = 100755(index 情報がある場合)
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

export function checkApmPayload(repoRoot) {
  const errors = [];
  const hooksDir = path.join(repoRoot, "packages/apm/.apm/hooks");

  for (const asset of REQUIRED_ASSETS) {
    const relative = `packages/apm/.apm/hooks/${asset}`;
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
    errors.push("cli/ payload must not exist — CLI is cargo-installed, not bundled");

  // 拡張子 allowlist + binary sniffing(POSIX mode では判定しない —
  // cheaper.sh は legitimately 100755)。
  if (existsSync(hooksDir)) {
    for (const file of walkFiles(hooksDir)) {
      const relative = path.relative(repoRoot, file).split(path.sep).join("/");
      const ext = path.extname(file);
      if (ext !== "" && !EXTENSION_ALLOWLIST.has(ext))
        errors.push(`file extension not allowed in hooks payload: ${relative} (${ext})`);
      const kind = sniffExecutable(readFileSync(file));
      if (kind) errors.push(`executable payload is forbidden: ${relative} (${kind})`);
    }
  }

  // Byte-level launcher 契約。
  const ps1 = path.join(hooksDir, "cheaper.ps1");
  if (existsSync(ps1)) {
    const bytes = readFileSync(ps1);
    if (!bytes.subarray(0, 3).equals(Buffer.from([0xef, 0xbb, 0xbf])))
      errors.push("cheaper.ps1 must start with UTF-8 BOM (EF BB BF) for Windows PowerShell 5.1");
    if (!bytes.includes("\r\n"))
      errors.push("cheaper.ps1 must use CRLF line endings for Windows PowerShell 5.1");
  }
  const sh = path.join(hooksDir, "cheaper.sh");
  if (existsSync(sh)) {
    const bytes = readFileSync(sh);
    if (!bytes.toString("utf8", 0, 9).startsWith("#!/bin/sh"))
      errors.push("cheaper.sh must start with a #!/bin/sh shebang");
    if (bytes.includes("\r\n")) errors.push("cheaper.sh must use LF line endings (CR detected)");
    try {
      const mode = execFileSync("git", ["ls-files", "-s", "packages/apm/.apm/hooks/cheaper.sh"], {
        cwd: repoRoot,
        encoding: "utf8",
      }).match(/^(\d+)/)?.[1];
      if (mode && mode !== "100755")
        errors.push(`cheaper.sh git index mode must be 100755, got ${mode}`);
    } catch {
      // index 情報が無い環境(export 済み snapshot 等)では skip。
    }
  }

  // hooks.json ↔ launcher 整合。
  try {
    const hooks = JSON.parse(readFileSync(path.join(hooksDir, "hooks.json"), "utf8"));
    const text = readFileSync(path.join(hooksDir, "hooks.json"), "utf8");
    if (/(?:^|[\s"'])\.\/cheaper(?![.\w])/m.test(text))
      errors.push(
        'hooks.json must not reference bare "./cheaper" (use sh ./cheaper.sh or ./cheaper.ps1)',
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
        errors.push(`POSIX command must invoke "sh ./cheaper.sh": ${entry.command}`);
      if (typeof entry.powershell === "string" && !entry.powershell.includes("./cheaper.ps1"))
        errors.push(`powershell entry must invoke "./cheaper.ps1": ${entry.powershell}`);
    }
  } catch (error) {
    errors.push(`hooks.json unreadable or inconsistent: ${error.message}`);
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
