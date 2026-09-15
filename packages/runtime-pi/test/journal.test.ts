import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { MutationJournal } from "../src/journal.js";

let dir: string;

beforeEach(async () => {
  dir = await mkdtemp(path.join(tmpdir(), "cheaper-journal-"));
});
afterEach(async () => {
  await rm(dir, { recursive: true, force: true });
});

describe("MutationJournal", () => {
  it("reports empty changes when nothing was captured", async () => {
    const j = new MutationJournal();
    expect(j.isEmpty()).toBe(true);
    const changes = await j.finalize();
    expect(changes).toEqual({
      files: [],
      totals: { files: 0, linesAdded: 0, linesDeleted: 0 },
    });
  });

  it("records a created file (before=null) with added line count", async () => {
    const j = new MutationJournal();
    const real = path.join(dir, "new.txt");
    await j.captureBefore(real, "new.txt");
    await writeFile(real, "a\nb\nc\n", "utf8");
    const changes = await j.finalize();
    expect(changes.files).toEqual([
      { path: "new.txt", kind: "created", linesAdded: 3, linesDeleted: 0 },
    ]);
    expect(changes.totals).toEqual({ files: 1, linesAdded: 3, linesDeleted: 0 });
  });

  it("records a modified file with added/deleted counts", async () => {
    const real = path.join(dir, "mod.txt");
    await writeFile(real, "one\ntwo\nthree\n", "utf8");
    const j = new MutationJournal();
    await j.captureBefore(real, "mod.txt");
    await writeFile(real, "one\nTWO\nthree\nfour\n", "utf8");
    const changes = await j.finalize();
    expect(changes.files).toEqual([
      { path: "mod.txt", kind: "modified", linesAdded: 2, linesDeleted: 1 },
    ]);
    expect(changes.totals).toEqual({ files: 1, linesAdded: 2, linesDeleted: 1 });
  });

  it("keeps the first before-content across multiple captures", async () => {
    const real = path.join(dir, "multi.txt");
    await writeFile(real, "orig\n", "utf8");
    const j = new MutationJournal();
    await j.captureBefore(real, "multi.txt");
    await writeFile(real, "second\n", "utf8");
    await j.captureBefore(real, "multi.txt"); // must not overwrite before
    await writeFile(real, "third\n", "utf8");
    const changes = await j.finalize();
    expect(changes.files).toEqual([
      { path: "multi.txt", kind: "modified", linesAdded: 1, linesDeleted: 1 },
    ]);
  });

  it("excludes files whose final content equals the before-content", async () => {
    const real = path.join(dir, "same.txt");
    await writeFile(real, "stable\n", "utf8");
    const j = new MutationJournal();
    await j.captureBefore(real, "same.txt");
    await writeFile(real, "stable\n", "utf8"); // rewritten identically
    const changes = await j.finalize();
    expect(changes.files).toEqual([]);
    expect(changes.totals.files).toBe(0);
  });

  it("aggregates totals across multiple files", async () => {
    const j = new MutationJournal();
    const a = path.join(dir, "a.txt");
    const b = path.join(dir, "b.txt");
    await writeFile(b, "x\n", "utf8");
    await j.captureBefore(a, "a.txt");
    await j.captureBefore(b, "b.txt");
    await writeFile(a, "1\n2\n", "utf8");
    await writeFile(b, "x\ny\n", "utf8");
    const changes = await j.finalize();
    expect(changes.files).toHaveLength(2);
    expect(changes.totals).toEqual({ files: 2, linesAdded: 3, linesDeleted: 0 });
  });
});
