import { readdir, readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const SRC_DIR = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../src");

async function collectTsFiles(dir: string): Promise<string[]> {
  const entries = await readdir(dir, { withFileTypes: true });
  const files: string[] = [];
  for (const entry of entries) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) files.push(...(await collectTsFiles(full)));
    else if (entry.name.endsWith(".ts")) files.push(full);
  }
  return files;
}

/** comment（//・block）と string literal を簡易 strip する。 */
function stripCommentsAndStrings(source: string): string {
  return source
    .replace(/\/\*[\s\S]*?\*\//g, " ")
    .replace(/\/\/[^\n]*/g, " ")
    .replace(/`(?:\\.|[^`\\])*`/g, "``")
    .replace(/"(?:\\.|[^"\\])*"/g, '""')
    .replace(/'(?:\\.|[^'\\])*'/g, "''");
}

describe("no toolExecution / executionMode in runtime-pi sources", () => {
  it("packages/runtime-pi/src does not set toolExecution or executionMode", async () => {
    // Pi の toolExecution: "parallel" 既定を維持し、#57 ToolScheduler の
    // concurrency limit 導入余地を pin する検査。
    // 「identifier の完全な不存在」を要求するのは #57 余地確保より強い
    // 意図的な厳格さであり、将来 legitimate な sequential tool を追加する
    // 場合は本 test の緩和を要する。src/testsupport.ts も走査対象。
    const files = await collectTsFiles(SRC_DIR);
    expect(files.length).toBeGreaterThan(0);
    const offenders: string[] = [];
    for (const file of files) {
      const stripped = stripCommentsAndStrings(await readFile(file, "utf8"));
      if (/\btoolExecution\b/.test(stripped) || /\bexecutionMode\b/.test(stripped)) {
        offenders.push(path.relative(SRC_DIR, file));
      }
    }
    expect(offenders).toEqual([]);
  });
});
