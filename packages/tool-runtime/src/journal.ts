import { readFile } from "node:fs/promises";
import { diffLines } from "diff";

/** meta.changes.files の 1 エントリ（DESIGN-0005 envelope）。 */
export interface FileChange {
  /** workspace 相対 POSIX 区切り path。 */
  path: string;
  kind: "created" | "modified";
  linesAdded: number;
  linesDeleted: number;
}

/** meta.changes の shape。 */
export interface ChangesSummary {
  files: FileChange[];
  totals: { files: number; linesAdded: number; linesDeleted: number };
}

interface JournalEntry {
  display: string;
  /** 初回書き込み前の内容。存在しなかったファイルは null。 */
  before: Buffer | null;
}

/**
 * mutation journal（DESIGN-0002 / ADR-0006）。
 * write/edit operations が初回書き込み前に `captureBefore` を呼び、
 * 実行終了時に `finalize` が before と disk 上の最終内容を比較して
 * `meta.changes` を生成する。差分なしのエントリは除外する。
 */
export class MutationJournal {
  private readonly entries = new Map<string, JournalEntry>();

  /**
   * real path をキーに初回書き込み前の内容を記録する。
   * 同一 path の 2 回目以降の呼び出しは before を保持する（冪等）。
   */
  async captureBefore(real: string, display: string): Promise<void> {
    if (this.entries.has(real)) return;
    let before: Buffer | null;
    try {
      before = await readFile(real);
    } catch {
      // 不存在は null（created）。その他の読み取り失敗も null 扱いとし、
      // 後続の書き込み自体が失敗するか finalize の disk 比較に委ねる。
      before = null;
    }
    this.entries.set(real, { display, before });
  }

  /** journal に記録があるか（abort 分類の PARTIAL_MUTATION 判定に使用）。 */
  isEmpty(): boolean {
    return this.entries.size === 0;
  }

  /**
   * before と disk 上の最終内容を比較し、差分のあるファイルのみを
   * `ChangesSummary` として返す。行差分は `diffLines` で算出する。
   */
  async finalize(): Promise<ChangesSummary> {
    const files: FileChange[] = [];
    for (const [real, entry] of this.entries) {
      let after: Buffer | null;
      try {
        after = await readFile(real);
      } catch {
        after = null;
      }
      const beforeStr = entry.before === null ? null : entry.before.toString("utf8");
      const afterStr = after === null ? null : after.toString("utf8");
      if (beforeStr === afterStr) continue; // 差分なし → 除外
      let linesAdded = 0;
      let linesDeleted = 0;
      for (const part of diffLines(beforeStr ?? "", afterStr ?? "")) {
        if (part.added === true) linesAdded += part.count ?? 0;
        if (part.removed === true) linesDeleted += part.count ?? 0;
      }
      files.push({
        path: entry.display,
        kind: entry.before === null ? "created" : "modified",
        linesAdded,
        linesDeleted,
      });
    }
    files.sort((a, b) => a.path.localeCompare(b.path));
    const totals = files.reduce(
      (acc, f) => ({
        files: acc.files + 1,
        linesAdded: acc.linesAdded + f.linesAdded,
        linesDeleted: acc.linesDeleted + f.linesDeleted,
      }),
      { files: 0, linesAdded: 0, linesDeleted: 0 },
    );
    return { files, totals };
  }
}
