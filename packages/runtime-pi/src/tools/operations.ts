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
import { rethrowAsDenied } from "./walker.js";

export interface BoundaryOperationsOptions {
  /** canonical 化済み workspace root（core `canonicalizeWorkspaceRoot` の戻り値）。 */
  root: string;
  symlinkPolicy: SymlinkPolicy;
  journal: MutationJournal;
}

async function resolve(
  opts: BoundaryOperationsOptions,
  absolutePath: string,
): Promise<ResolvedWorkspacePath> {
  try {
    return await resolveWorkspacePath(opts.root, absolutePath, opts.symlinkPolicy);
  } catch (e) {
    rethrowAsDenied(e);
  }
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
  const resolved = await resolve(opts, absolutePath);
  await opts.journal.captureBefore(resolved.real, resolved.display);
  const dir = path.dirname(resolved.real);
  const tmpCandidate = `.${path.basename(resolved.real)}.cheaper-tmp-${randomInt(0, 2 ** 31).toString(36)}`;
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
      return readdir(resolved.real);
    },
  };
}
