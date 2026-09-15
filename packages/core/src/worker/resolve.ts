import { access, readFile } from "node:fs/promises";
import path from "node:path";
import type { ConfigDirs } from "../config/dirs.js";
import { CheaperError } from "../errors.js";
import { parseWorkerFile } from "./parse.js";
import { validateWorkerDefinition, type WorkerDefinition, type WorkerOrigin } from "./validate.js";

const NAME_PATTERN = /^[a-z0-9][a-z0-9-]*$/;

async function exists(p: string): Promise<boolean> {
  try {
    await access(p);
    return true;
  } catch {
    return false;
  }
}

async function loadWorker(sourcePath: string, origin: WorkerOrigin): Promise<WorkerDefinition> {
  const content = await readFile(sourcePath, "utf8");
  return validateWorkerDefinition(parseWorkerFile(content, sourcePath), { sourcePath, origin });
}

export async function resolveWorker(name: string, dirs: ConfigDirs): Promise<WorkerDefinition> {
  if (!NAME_PATTERN.test(name)) {
    throw new CheaperError("INVALID_WORKER_NAME", `invalid worker name: ${name}`, {
      details: { name },
    });
  }
  const candidates: Array<{ dir: string; origin: WorkerOrigin }> = [
    { dir: dirs.projectDir, origin: "project" },
    { dir: dirs.userConfigDir, origin: "user" },
    { dir: dirs.builtinDir, origin: "builtin" },
  ];
  const searched: string[] = [];
  for (const { dir, origin } of candidates) {
    const p = path.join(dir, "workers", `${name}.md`);
    searched.push(p);
    if (await exists(p)) {
      return loadWorker(p, origin);
    }
  }
  throw new CheaperError("WORKER_NOT_FOUND", `worker not found: ${name}`, {
    details: { name, searched },
  });
}

export async function resolveWorkerFile(filePath: string, cwd: string): Promise<WorkerDefinition> {
  const resolved = path.resolve(cwd, filePath);
  if (!(await exists(resolved))) {
    throw new CheaperError("WORKER_NOT_FOUND", `worker file not found: ${filePath}`, {
      details: { filePath, searched: [resolved] },
    });
  }
  return loadWorker(resolved, "explicit");
}
