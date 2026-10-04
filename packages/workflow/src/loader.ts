import { readFile, stat } from "node:fs/promises";
import path from "node:path";
import {
  CheaperError,
  type PathDenylist,
  resolveWorkspacePath,
  type SymlinkPolicy,
} from "@cheaper/core";
import type { WorkflowCompiler } from "./compiler.js";
import type { WorkflowDefinition } from "./types.js";

export interface WorkflowFileOptions {
  /** canonical 化済み workspace root。 */
  root: string;
  path: string;
  symlinkPolicy: SymlinkPolicy;
  denylist: PathDenylist;
}

export type WorkflowCompilerFactory = (
  raw: unknown,
) => WorkflowCompiler | Promise<WorkflowCompiler>;

export interface LoadedWorkflow {
  definition: WorkflowDefinition;
  compiler: WorkflowCompiler;
}

function checkDenied(display: string, denylist: PathDenylist): void {
  const segments = display.split("/");
  for (let i = 1; i <= segments.length; i++) {
    if (denylist.isDenied(segments.slice(0, i).join("/"))) {
      throw new CheaperError("INVALID_REQUEST", "workflow path is denied by policy");
    }
  }
}

/** 明示 file の I/O 境界だけを所有し、定義検証は canonical compiler に委譲する。 */
export async function loadWorkflowFile(
  options: WorkflowFileOptions,
  createCompiler: WorkflowCompilerFactory,
): Promise<LoadedWorkflow> {
  let source: string;
  try {
    const resolved = await resolveWorkspacePath(options.root, options.path, options.symlinkPolicy);
    const logical = path.relative(options.root, path.resolve(options.root, options.path));
    checkDenied(logical.split(path.sep).join("/"), options.denylist);
    checkDenied(resolved.display, options.denylist);
    if (!(await stat(resolved.real)).isFile()) {
      throw new CheaperError("INVALID_REQUEST", "workflow path must be a regular file");
    }
    source = await readFile(resolved.real, "utf8");
  } catch (cause) {
    if (cause instanceof CheaperError) throw cause;
    throw new CheaperError("INVALID_REQUEST", "cannot read workflow file", {
      details: { path: options.path },
      cause,
    });
  }
  let raw: unknown;
  try {
    raw = JSON.parse(source);
  } catch {
    // JSON.parse の message は private 本文を含み得るため公開しない。
    throw new CheaperError("INVALID_INPUT", "workflow file must contain valid JSON");
  }
  const compiler = await createCompiler(raw);
  return { definition: compiler.validateDefinition(raw), compiler };
}
