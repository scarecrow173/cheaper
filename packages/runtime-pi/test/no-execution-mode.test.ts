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

describe("no toolExecution / executionMode in runtime-pi sources", () => {
  it("packages/runtime-pi/src does not mention toolExecution or executionMode anywhere", async () => {
    // Pi の toolExecution: "parallel" 既定を維持し、#57 ToolScheduler の
    // concurrency limit 導入余地を pin する検査。
    // comment / string literal を strip せず生の source を走査する。
    // quoted key（"toolExecution": ...）や computed key、template literal
    // 経由の設定をも検出するためであり、「identifier の完全な不存在」を
    // 要求するのは #57 余地確保より強い意図的な厳格さ（コメント中の言及も
    // 許容しない）。将来 legitimate な sequential tool を追加する場合は
    // 本 test の緩和を要する。src/testsupport.ts も走査対象。
    const files = await collectTsFiles(SRC_DIR);
    expect(files.length).toBeGreaterThan(0);
    const offenders: string[] = [];
    for (const file of files) {
      const source = await readFile(file, "utf8");
      if (/\btoolExecution\b/.test(source) || /\bexecutionMode\b/.test(source)) {
        offenders.push(path.relative(SRC_DIR, file));
      }
    }
    expect(offenders).toEqual([]);
  });
});
