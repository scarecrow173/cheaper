import { mkdtemp, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

export async function makeTmpDir(): Promise<string> {
  return mkdtemp(path.join(os.tmpdir(), "cheaper-test-"));
}

export async function cleanupTmpDir(dir: string): Promise<void> {
  await rm(dir, { recursive: true, force: true });
}
