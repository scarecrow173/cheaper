import { rm, symlink, writeFile } from "node:fs/promises";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  DEFAULT_THRESHOLD,
  evaluate,
  extractReadCandidates,
  threshold,
} from "../../src/hook/routing-policy.js";
import { cliRoot, makeIsolatedEnv } from "../e2e/helpers.js";

// routing-policy.js（packages/apm/.apm/hooks/）の TS 移植の parity
// test。canonical request に対して JS 版と同一の verdict を返すことを
// 固定する（TSPEC-034z50PbuzTYDWvQXYdvaG）。

const BIG_TEXT = "lorem ipsum dolor sit amet ".repeat(4000);

describe("routing policy (cli port)", () => {
  let dir: string;
  let ws: string;
  let env: Record<string, string>;
  let opts: { env: NodeJS.ProcessEnv; builtinDir: string };

  beforeEach(async () => {
    ({ dir, ws, env } = await makeIsolatedEnv());
    opts = { env: env as NodeJS.ProcessEnv, builtinDir: cliRoot };
  });
  afterEach(async () => {
    await rm(dir, { recursive: true, force: true });
  });

  const fileRead = (p: string) =>
    evaluate({ kind: "file-read", path: p, cwd: ws, sourceTool: "test" }, opts);
  const shell = (command: string) =>
    evaluate({ kind: "shell", command, cwd: ws, sourceTool: "test" }, opts);

  describe("threshold", () => {
    it("defaults to 8192 and honors CHEAPER_READ_TOKEN_THRESHOLD", () => {
      expect(DEFAULT_THRESHOLD).toBe(8192);
      expect(threshold({} as NodeJS.ProcessEnv)).toBe(8192);
      expect(threshold({ CHEAPER_READ_TOKEN_THRESHOLD: "1024" } as NodeJS.ProcessEnv)).toBe(1024);
      // JS 版と同じく parseInt の部分 parse を許容する
      expect(threshold({ CHEAPER_READ_TOKEN_THRESHOLD: "1024abc" } as NodeJS.ProcessEnv)).toBe(
        1024,
      );
      expect(threshold({ CHEAPER_READ_TOKEN_THRESHOLD: "0" } as NodeJS.ProcessEnv)).toBe(8192);
      expect(threshold({ CHEAPER_READ_TOKEN_THRESHOLD: "-5" } as NodeJS.ProcessEnv)).toBe(8192);
      expect(threshold({ CHEAPER_READ_TOKEN_THRESHOLD: "abc" } as NodeJS.ProcessEnv)).toBe(8192);
    });
  });

  describe("file-read", () => {
    it("allows a file outside the workspace without measuring", async () => {
      const outside = path.join(dir, "outside.txt");
      await writeFile(outside, BIG_TEXT, "utf8");
      expect(await fileRead(outside)).toEqual({ decision: "allow" });
    });

    it("allows missing files, directories, and the cwd itself", async () => {
      expect(await fileRead(path.join(ws, "missing.txt"))).toEqual({ decision: "allow" });
      expect(await fileRead(ws)).toEqual({ decision: "allow" });
      expect(await fileRead(".")).toEqual({ decision: "allow" });
    });

    it("allows a file at or below the byte fast path", async () => {
      await writeFile(path.join(ws, "small.txt"), "hello\n", "utf8");
      expect(await fileRead(path.join(ws, "small.txt"))).toEqual({ decision: "allow" });
    });

    it("denies a file over the token threshold with the JS-format reason", async () => {
      await writeFile(path.join(ws, "big.txt"), BIG_TEXT, "utf8");
      const v = await fileRead(path.join(ws, "big.txt"));
      expect(v.decision).toBe("deny");
      if (v.decision !== "deny") return;
      expect(v.reason).toMatch(/'big\.txt' has \d+ tokens \(threshold 8192\)\./);
      expect(v.reason).toContain("cheaper-bulk-read skill");
      expect(v.reason).toContain("workspace-relative path: big.txt");
    });

    it("denies on FILE_TOO_LARGE (fail-closed)", async () => {
      await writeFile(path.join(ws, "big.bin"), Buffer.alloc(64, 0x61));
      const strict = {
        env: {
          ...env,
          CHEAPER_READ_TOKEN_THRESHOLD: "8",
          CHEAPER_COUNT_MAX_BYTES: "16",
        } as NodeJS.ProcessEnv,
        builtinDir: cliRoot,
      };
      const v = await evaluate(
        { kind: "file-read", path: path.join(ws, "big.bin"), cwd: ws },
        strict,
      );
      expect(v.decision).toBe("deny");
      if (v.decision !== "deny") return;
      expect(v.reason).toContain("'big.bin' is 64 bytes");
      expect(v.reason).toContain("exceeds the 16-byte measurement cap");
      expect(v.reason).toContain("workspace-relative path: big.bin");
    });

    it("fails open under a symlinked cwd (parity quirk)", async () => {
      // raw cwd で resolve した path を canonical root で数えるため、
      // realpath と一致しない cwd では WorkspaceEscapeError → allow。
      // request.cwd を直接与えるため platform 非依存に再現する。
      const link = path.join(dir, "ws-link");
      await symlink(ws, link, process.platform === "win32" ? "junction" : "dir");
      await writeFile(path.join(ws, "big.txt"), BIG_TEXT, "utf8");
      const v = await evaluate({ kind: "file-read", path: "big.txt", cwd: link }, opts);
      expect(v).toEqual({ decision: "allow" });
    });

    it("fails open when the workspace root does not exist", async () => {
      const gone = path.join(dir, "no-such-dir");
      const v = await evaluate(
        {
          kind: "file-read",
          // containment は raw cwd 基準の lexical 判定を通すため
          // cwd 内の path を与えつつ実在しない root にする
          path: path.join(gone, "..", "..", gone, "big.txt"),
          cwd: gone,
        },
        opts,
      );
      expect(v).toEqual({ decision: "allow" });
    });

    it("allows malformed requests", async () => {
      for (const req of [
        null,
        42,
        "x",
        {},
        { kind: "file-read" },
        { kind: "file-read", path: "" },
      ]) {
        expect(await evaluate(req, opts)).toEqual({ decision: "allow" });
      }
    });
  });

  describe("shell read candidates", () => {
    it("extracts file operands matching the JS tokenizer", () => {
      expect(extractReadCandidates("cat big.txt")).toEqual(["big.txt"]);
      expect(extractReadCandidates("head -n 200 big.txt")).toEqual(["big.txt"]);
      expect(extractReadCandidates("cat small.txt big.txt")).toEqual(["small.txt", "big.txt"]);
      expect(extractReadCandidates('cat "path with spaces/big.txt"')).toEqual([
        "path with spaces/big.txt",
      ]);
      expect(extractReadCandidates("cat big.txt | grep foo")).toEqual(["big.txt"]);
      expect(extractReadCandidates("cat big.txt && cd src")).toEqual(["big.txt"]);
      expect(extractReadCandidates("cat $(ls)")).toEqual([]);
      expect(extractReadCandidates("cat *.txt")).toEqual([]);
      expect(extractReadCandidates("cat < big.txt")).toEqual([]);
      expect(extractReadCandidates('cat "unclosed')).toEqual([]);
      expect(extractReadCandidates("cat.exe big.txt")).toEqual(["big.txt"]);
      // cwd 遷移後の read candidate は fail-open（抽出しない）
      expect(extractReadCandidates("cd src && cat big.txt")).toEqual([]);
      expect(extractReadCandidates("FOO=1 cat big.txt")).toEqual(["big.txt"]);
    });
  });

  describe("shell verdict", () => {
    beforeEach(async () => {
      await writeFile(path.join(ws, "big.txt"), BIG_TEXT, "utf8");
      await writeFile(path.join(ws, "small.txt"), "hello\n", "utf8");
    });

    it("denies a large read candidate with the blocked-command prefix", async () => {
      const v = await shell("cat big.txt");
      expect(v.decision).toBe("deny");
      if (v.decision !== "deny") return;
      expect(v.reason).toContain("blocked shell command 'cat big.txt'.");
      expect(v.reason).toContain("workspace-relative path: big.txt");
    });

    it("allows commands without read candidates and after cwd changes", async () => {
      expect(await shell("cat small.txt")).toEqual({ decision: "allow" });
      expect(await shell("cat")).toEqual({ decision: "allow" });
      expect(await shell("ls -la")).toEqual({ decision: "allow" });
      expect(await shell("cd src && cat big.txt")).toEqual({ decision: "allow" });
      expect(await shell("cat missing.txt")).toEqual({ decision: "allow" });
    });

    it("denies when any operand is large and shares the file-read verdict", async () => {
      const v = await shell("cat small.txt big.txt");
      expect(v.decision).toBe("deny");
      if (v.decision === "deny") {
        expect(v.reason).toContain("workspace-relative path: big.txt");
      }
      const direct = await fileRead(path.join(ws, "big.txt"));
      expect(direct.decision).toBe("deny");
    });
  });

  describe("measurement path", () => {
    it("does not load the tokenizer for files under the byte threshold", async () => {
      // byte fast path で allow になる場合、計測 import が走らないことを
      // 正常系（tokenizer 未初期化でも pass）で間接確認する。
      await writeFile(path.join(ws, "tiny.txt"), "x", "utf8");
      const v = await fileRead(path.join(ws, "tiny.txt"));
      expect(v).toEqual({ decision: "allow" });
    });

    it("honors CHEAPER_COUNT_MAX_BYTES as fail-closed deny", async () => {
      await writeFile(path.join(ws, "big.txt"), BIG_TEXT, "utf8");
      const strict = {
        env: { ...env, CHEAPER_COUNT_MAX_BYTES: "1024" } as NodeJS.ProcessEnv,
        builtinDir: cliRoot,
      };
      const v = await evaluate({ kind: "file-read", path: "big.txt", cwd: ws }, strict);
      expect(v.decision).toBe("deny");
      if (v.decision === "deny") {
        expect(v.reason).toContain("bytes");
        expect(v.reason).toContain("bulk-read");
      }
    });
  });
});
