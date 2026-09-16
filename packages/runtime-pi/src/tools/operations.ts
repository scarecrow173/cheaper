import { randomInt } from "node:crypto";
import { constants } from "node:fs";
import {
  access,
  mkdir,
  readdir,
  readFile,
  rename,
  stat,
  unlink,
  writeFile,
} from "node:fs/promises";
import path from "node:path";
import {
  type PathDenylist,
  type ResolvedWorkspacePath,
  resolveWorkspacePath,
  type SymlinkPolicy,
} from "@cheaper/core";
import type {
  EditOperations,
  LsOperations,
  ReadOperations,
  WriteOperations,
} from "@earendil-works/pi-coding-agent";
import type { MutationJournal } from "../journal.js";
import { POLICY_DENIED_MESSAGE, rethrowAsDenied, SCOPE_DENIED_MESSAGE } from "./walker.js";

export interface BoundaryOperationsOptions {
  /** canonical 化済み workspace root（core `canonicalizeWorkspaceRoot` の戻り値）。 */
  root: string;
  symlinkPolicy: SymlinkPolicy;
  journal: MutationJournal;
  /** privacy deny list（SPEC-0010）。match した path への全 access を拒否する。 */
  denylist?: PathDenylist | undefined;
  /**
   * write scope（`cheaper.writeScope` 宣言 worker の targets、正規化済み
   * display path 集合）。設定時、write/edit/mkdir による mutation は
   * scope path に限られる。read 系は制限しない。
   */
  writeScope?: ReadonlySet<string> | undefined;
}

async function resolve(
  opts: BoundaryOperationsOptions,
  absolutePath: string,
): Promise<ResolvedWorkspacePath> {
  let resolved: ResolvedWorkspacePath;
  try {
    resolved = await resolveWorkspacePath(opts.root, absolutePath, opts.symlinkPolicy);
  } catch (e) {
    rethrowAsDenied(e);
  }
  if (opts.denylist?.isDenied(resolved.display)) {
    throw new Error(POLICY_DENIED_MESSAGE);
  }
  return resolved;
}

/** write/edit/mkdir 用: boundary+deny のうえ write scope membership を要求する。 */
async function resolveForWrite(
  opts: BoundaryOperationsOptions,
  absolutePath: string,
): Promise<ResolvedWorkspacePath> {
  const resolved = await resolve(opts, absolutePath);
  if (opts.writeScope !== undefined && !opts.writeScope.has(resolved.display)) {
    throw new Error(SCOPE_DENIED_MESSAGE);
  }
  return resolved;
}

/**
 * 同ディレクトリ tmp + rename による原子書き込み（DESIGN-0002）。
 * tmp path も boundary 判定を通す。失敗時は finally で tmp を unlink する。
 */
async function atomicWrite(
  opts: BoundaryOperationsOptions,
  absolutePath: string,
  content: string,
): Promise<void> {
  const resolved = await resolveForWrite(opts, absolutePath);
  await opts.journal.captureBefore(resolved.real, resolved.display);
  const dir = path.dirname(resolved.real);
  const tmpCandidate = `.${path.basename(resolved.real)}.cheaper-tmp-${randomInt(0, 2 ** 31).toString(36)}`;
  // tmp は target 認可済み write の実装詳細のため scope check は通さない
  // （deny check は resolve 内で引き続き有効）。
  const tmpResolved = await resolve(opts, path.join(dir, tmpCandidate));
  const tmp = tmpResolved.real;
  try {
    await writeFile(tmp, content, "utf8");
    await rename(tmp, resolved.real);
  } finally {
    await unlink(tmp).catch(() => {});
  }
}

export function createReadOperations(opts: BoundaryOperationsOptions): ReadOperations {
  return {
    readFile: async (absolutePath) => {
      const resolved = await resolve(opts, absolutePath);
      return readFile(resolved.real);
    },
    access: async (absolutePath) => {
      const resolved = await resolve(opts, absolutePath);
      await access(resolved.real, constants.R_OK);
    },
    // detectImageMimeType は提供しない。Pi 側は未提供時に画像検出を skip し、
    // ops.readFile 経由のテキスト読みに倒れるため boundary 迂回はない（IR 記録）。
  };
}

export function createWriteOperations(opts: BoundaryOperationsOptions): WriteOperations {
  return {
    writeFile: (absolutePath, content) => atomicWrite(opts, absolutePath, content),
    mkdir: async (dir) => {
      const resolved = await resolve(opts, dir);
      // write tool は target の parent dir を先に mkdir するため、scope path の
      // 祖先 dir に限り許可する（dir 自体の scope membership は要求しない）。
      if (opts.writeScope !== undefined) {
        const d = resolved.display;
        const allowed =
          d === "." || [...opts.writeScope].some((t) => t === d || t.startsWith(`${d}/`));
        if (!allowed) throw new Error(SCOPE_DENIED_MESSAGE);
      }
      await mkdir(resolved.real, { recursive: true });
    },
  };
}

export function createEditOperations(opts: BoundaryOperationsOptions): EditOperations {
  return {
    readFile: async (absolutePath) => {
      const resolved = await resolve(opts, absolutePath);
      return readFile(resolved.real);
    },
    writeFile: (absolutePath, content) => atomicWrite(opts, absolutePath, content),
    access: async (absolutePath) => {
      const resolved = await resolve(opts, absolutePath);
      await access(resolved.real, constants.R_OK | constants.W_OK);
    },
  };
}

export function createLsOperations(opts: BoundaryOperationsOptions): LsOperations {
  return {
    exists: async (absolutePath) => {
      const resolved = await resolve(opts, absolutePath);
      try {
        await access(resolved.real, constants.F_OK);
        return true;
      } catch {
        return false;
      }
    },
    stat: async (absolutePath) => {
      const resolved = await resolve(opts, absolutePath);
      const st = await stat(resolved.real);
      return { isDirectory: () => st.isDirectory() };
    },
    readdir: async (absolutePath) => {
      const resolved = await resolve(opts, absolutePath);
      const names = await readdir(resolved.real);
      const denylist = opts.denylist;
      if (denylist === undefined) return names;
      const prefix = resolved.display === "." ? "" : `${resolved.display}/`;
      // deny entry は listing に出さない（存在の秘匿）。
      return names.filter((name) => !denylist.isDenied(`${prefix}${name}`));
    },
  };
}
