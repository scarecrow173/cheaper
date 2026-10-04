import { constants } from "node:fs";
import { access, open, realpath, stat } from "node:fs/promises";
import path from "node:path";
import {
  CheaperError,
  isShellProcessExecutable,
  type PathDenylist,
  resolveWorkspacePath,
  type SymlinkPolicy,
} from "@cheaper/core";

interface Options {
  root: string;
  symlinkPolicy: SymlinkPolicy;
  denylist?: PathDenylist | undefined;
  /** host の解決 context。子 env へは継承しない。 */
  searchPath?: string;
  execPath?: string;
}
function unsupported(): CheaperError {
  return new CheaperError(
    "POLICY_VIOLATION",
    "process requires a supported native executable; shell and script shims are unsupported",
  );
}
async function native(candidate: string): Promise<string | undefined> {
  try {
    const resolved = await realpath(candidate);
    if (isShellProcessExecutable(resolved) || !(await stat(resolved)).isFile()) return undefined;
    if (process.platform === "win32" && !/\.(exe|com)$/i.test(resolved)) return undefined;
    await access(resolved, constants.X_OK);
    // executable text を execvp/kernel の interpreter fallback へ渡さない。
    const file = await open(resolved, "r");
    const header = Buffer.alloc(4);
    try {
      await file.read(header, 0, header.length, 0);
    } finally {
      await file.close();
    }
    const magic = header.toString("hex");
    const supported =
      process.platform === "win32"
        ? magic.startsWith("4d5a")
        : magic === "7f454c46" ||
          (process.platform === "darwin" &&
            [
              "feedface",
              "feedfacf",
              "cefaedfe",
              "cffaedfe",
              "cafebabe",
              "bebafeca",
              "cafebabf",
              "bfbafeca",
            ].includes(magic));
    return supported ? resolved : undefined;
  } catch {
    return undefined;
  }
}

/** native argv0 解決だけを所有し、shell・PATH 子 env・任意 spawn API は提供しない。 */
export async function resolveProcessExecutable(command: string, options: Options): Promise<string> {
  if (isShellProcessExecutable(command)) throw unsupported();
  if (command.includes("/") || command.includes("\\") || path.isAbsolute(command)) {
    let candidate = command;
    if (!path.isAbsolute(command)) {
      const resolved = await resolveWorkspacePath(options.root, command, options.symlinkPolicy);
      if (options.denylist?.isDenied(resolved.display)) throw unsupported();
      candidate = resolved.real;
    }
    const found = await native(candidate);
    if (found) return found;
    throw unsupported();
  }
  const execPath = options.execPath ?? process.execPath;
  if (command === "node" && /^node(?:\.exe)?$/i.test(path.basename(execPath))) {
    const found = await native(execPath);
    if (found) return found;
  }
  const candidates =
    process.platform === "win32" && path.extname(command) === ""
      ? [`${command}.exe`, `${command}.com`]
      : [command];
  for (const directory of (options.searchPath ?? process.env.PATH ?? "").split(path.delimiter)) {
    if (!path.isAbsolute(directory)) continue;
    for (const name of candidates) {
      const found = await native(path.join(directory, name));
      if (found) return found;
    }
  }
  throw unsupported();
}
