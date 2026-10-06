import { mkdir, symlink, writeFile } from "node:fs/promises";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it, type TestContext } from "vitest";
import { CheaperError } from "../src/errors.js";
import {
  claimContains,
  claimsConflict,
  manifestsConflict,
  normalizeResourceClaim,
  normalizeResourceManifest,
  type ResourceClaim,
  type ResourceManifest,
  type ResourceManifestInput,
  type ResourceScope,
  resourceContains,
  resourceKey,
  resourcesOverlap,
} from "../src/resources/index.js";
import { canonicalizeWorkspaceRoot, WorkspaceEscapeError } from "../src/workspace/paths.js";
import { cleanupTmpDir, makeTmpDir } from "./fixtures/tmp.js";

function claim(
  identity: string,
  scope: ResourceScope = "exact",
  mode: ResourceClaim["mode"] = "shared",
): ResourceClaim {
  return { resource: { kind: "workspace", path: identity, scope }, mode };
}

function manifest(claims: ResourceClaim[]): ResourceManifest {
  return {
    reads: claims.filter((c) => c.mode === "shared"),
    writes: claims.filter((c) => c.mode === "exclusive"),
    produces: [],
  };
}

async function junction(target: string, alias: string, ctx: TestContext): Promise<void> {
  try {
    await symlink(target, alias, process.platform === "win32" ? "junction" : "dir");
  } catch (e) {
    const code = (e as NodeJS.ErrnoException).code;
    if (code === "EPERM" || code === "EACCES" || code === "ENOTSUP") {
      ctx.skip(`symlink unavailable: ${code}`);
    }
    throw e;
  }
}

describe("resource identity and manifest admission", () => {
  let dir: string;
  let root: string;
  beforeEach(async () => {
    dir = await makeTmpDir();
    root = await canonicalizeWorkspaceRoot(dir);
  });
  afterEach(async () => cleanupTmpDir(dir));

  const normalize = (input?: ResourceManifestInput) =>
    normalizeResourceManifest(root, input, "deny");

  // AC1: 未作成 file / directory に明示 scope の identity がある。
  it("normalizes future exact and subtree identities without requiring their existence", async () => {
    const exact = await normalizeResourceClaim(
      root,
      { path: "src/./a/../a/future.ts", scope: "exact" },
      "shared",
      "deny",
    );
    const subtree = await normalizeResourceClaim(
      root,
      { path: "src/a", scope: "subtree" },
      "exclusive",
      "deny",
    );
    expect(exact).toEqual(claim(path.join(root, "src", "a", "future.ts")));
    expect(subtree).toEqual(claim(path.join(root, "src", "a"), "subtree", "exclusive"));
    expect(resourceContains(subtree.resource, exact.resource)).toBe(true);
    expect(resourceContains(subtree.resource, claim(path.join(root, "src", "a")).resource)).toBe(
      true,
    );
    expect(resourceContains(subtree.resource, claim(path.join(root, "src", "ab")).resource)).toBe(
      false,
    );
  });

  // Coverage (a): unknown と declared-none は異なる。
  it.each([undefined, {}])(
    "falls back for omitted resources or empty object: %j",
    async (input) => {
      expect(await normalize(input)).toEqual({
        reads: [claim(root, "subtree")],
        writes: [claim(root, "subtree", "exclusive")],
        produces: [],
      });
    },
  );

  it.each(["reads", "writes"] as const)("treats %s null as unknown access", async (bucket) => {
    const input = { reads: [], writes: [], produces: [], [bucket]: null };
    const result = await normalize(input);
    expect(result[bucket]).toEqual([
      claim(root, "subtree", bucket === "reads" ? "shared" : "exclusive"),
    ]);
    expect(result[bucket === "reads" ? "writes" : "reads"]).toEqual([]);
  });

  it.each(["reads", "writes"] as const)("treats omitted %s as unknown access", async (bucket) => {
    const input = bucket === "reads" ? { writes: [] } : { reads: [] };
    expect((await normalize(input))[bucket]).toEqual([
      claim(root, "subtree", bucket === "reads" ? "shared" : "exclusive"),
    ]);
  });

  it("keeps explicit empty arrays empty and never invents producer ownership", async () => {
    expect(await normalize({ reads: [], writes: [], produces: [] })).toEqual(manifest([]));
    expect((await normalize({ reads: [], writes: [] })).produces).toEqual([]);
  });

  it("derives each bucket mode from declarations", async () => {
    const input = { path: "future", scope: "exact" as const };
    expect(await normalize({ reads: [input], writes: [input], produces: [input] })).toEqual({
      reads: [claim(path.join(root, "future"))],
      writes: [claim(path.join(root, "future"), "exact", "exclusive")],
      produces: [claim(path.join(root, "future"), "exact", "exclusive")],
    });
  });

  // Coverage (b), (h): 契約違反は INVALID_REQUEST、boundary error は非 wrap。
  it.each([
    null,
    [],
    { produces: null },
    { reads: "unknown" },
    { writes: {} },
    { read: [] },
    { unknown: [] },
    { reads: [{ path: "x", scope: "exact", mode: "shared" }] },
    { reads: [{ path: "x", scope: "exact", extra: true }] },
    { reads: [{ path: "x", scope: "exact" }, claim("absolute-placeholder")] },
    { writes: [claim("absolute-placeholder", "exact", "shared")] },
    { produces: [claim("absolute-placeholder", "exact", "shared")] },
    { reads: [claim("absolute-placeholder", "exact", "exclusive")] },
    { reads: [{ resource: { kind: "artifact", path: "x", scope: "exact" }, mode: "shared" }] },
    {
      reads: [
        { resource: { kind: "workspace", path: "relative", scope: "exact" }, mode: "shared" },
      ],
    },
    {
      reads: [
        { resource: { kind: "workspace", path: "x", scope: "exact", extra: true }, mode: "shared" },
      ],
    },
  ])("rejects malformed manifest admission: %j", async (input) => {
    await expect(normalize(input as ResourceManifestInput)).rejects.toMatchObject({
      code: "INVALID_REQUEST",
    });
  });

  it.each([
    { path: "", scope: "exact" },
    { path: "x\0y", scope: "exact" },
    { path: "x", scope: "invalid" },
    { path: 123, scope: "exact" },
    { scope: "exact" },
    { path: "x" },
    { path: "C:relative", scope: "exact" },
    { path: "/absolute", scope: "exact" },
  ])("rejects malformed declarations: %j", async (input) => {
    await expect(normalize({ reads: [input] } as ResourceManifestInput)).rejects.toBeInstanceOf(
      CheaperError,
    );
    await expect(normalize({ reads: [input] } as ResourceManifestInput)).rejects.toMatchObject({
      code: "INVALID_REQUEST",
    });
  });

  it("rejects invalid standalone mode, policy and an absolute declaration", async () => {
    await expect(
      normalizeResourceClaim(root, { path: "x", scope: "exact" }, "invalid" as "shared", "deny"),
    ).rejects.toMatchObject({ code: "INVALID_REQUEST" });
    await expect(normalizeResourceManifest(root, {}, "invalid" as "deny")).rejects.toMatchObject({
      code: "INVALID_REQUEST",
    });
    await expect(normalize({ reads: [{ path: root, scope: "subtree" }] })).rejects.toMatchObject({
      code: "INVALID_REQUEST",
    });
  });

  it.each(["../escape", "../escape/*.json", "src/../../escape"])(
    "propagates WorkspaceEscapeError for declaration escape: %s",
    async (input) => {
      await expect(normalize({ reads: [{ path: input, scope: "exact" }] })).rejects.toBeInstanceOf(
        WorkspaceEscapeError,
      );
    },
  );

  it("checks canonical claims at the same workspace boundary", async () => {
    const outside = claim(path.join(path.dirname(root), "outside"), "exact");
    await expect(normalize({ reads: [outside] })).rejects.toBeInstanceOf(WorkspaceEscapeError);
  });

  // Coverage (f), (g): identity を glob として再解釈せず permutation を安定化。
  it("is idempotent for canonical input including literal glob metacharacters", async () => {
    const input = {
      reads: [claim(path.join(root, "(literal).json"))],
      writes: [claim(path.join(root, "new"), "subtree", "exclusive")],
      produces: [claim(path.join(root, "(output).json"), "exact", "exclusive")],
    };
    expect(await normalize(input)).toEqual(input);
    expect(await normalize(await normalize(input))).toEqual(input);
  });

  it("sorts and deduplicates deterministically without subsuming finer declarations", async () => {
    const declarations = [
      { path: "z", scope: "exact" as const },
      { path: "a/x", scope: "exact" as const },
      { path: "a", scope: "subtree" as const },
      { path: "a/./x", scope: "exact" as const },
      { path: "a", scope: "subtree" as const },
    ];
    const result = await normalize({ reads: declarations, writes: [], produces: [] });
    expect(
      await normalize({ reads: [...declarations].reverse(), writes: [], produces: [] }),
    ).toEqual(result);
    expect(result.reads).toHaveLength(3);
    expect(result.reads.map((c) => path.relative(root, c.resource.path))).toEqual([
      "a",
      path.join("a", "x"),
      "z",
    ]);
  });

  it.runIf(process.platform === "win32")(
    "uses Windows case identity and stable case dedup",
    async () => {
      const declarations = [
        { path: "Future/File", scope: "exact" as const },
        { path: "future/file", scope: "exact" as const },
      ];
      const a = await normalize({ reads: declarations, writes: [] });
      const b = await normalize({ reads: [...declarations].reverse(), writes: [] });
      expect(a).toEqual(b);
      expect(a.reads).toHaveLength(1);
      expect(resourceKey(claim(path.join(root, "Future", "File")).resource)).toBe(
        resourceKey(claim(path.join(root, "future", "file")).resource),
      );
    },
  );

  it.skipIf(process.platform === "win32")("preserves POSIX case distinction", async () => {
    const result = await normalize({
      reads: [
        { path: "Case/f", scope: "exact" },
        { path: "case/f", scope: "exact" },
      ],
      writes: [],
    });
    expect(result.reads).toHaveLength(2);
    expect(new Set(result.reads.map((c) => resourceKey(c.resource))).size).toBe(2);
  });

  it("resolves within-workspace aliases but enforces deny and external-target policy", async (ctx) => {
    await mkdir(path.join(root, "actual"));
    await junction(path.join(root, "actual"), path.join(root, "alias"), ctx);
    const result = await normalizeResourceManifest(
      root,
      {
        writes: [
          { path: "alias/future", scope: "exact" },
          { path: "actual/future", scope: "exact" },
        ],
        reads: [],
      },
      "within-workspace",
    );
    expect(result.writes).toEqual([
      claim(path.join(root, "actual", "future"), "exact", "exclusive"),
    ]);
    await expect(
      normalize({ writes: [{ path: "alias/future", scope: "exact" }] }),
    ).rejects.toBeInstanceOf(WorkspaceEscapeError);
    await junction(path.dirname(root), path.join(root, "outside-alias"), ctx);
    await expect(
      normalizeResourceManifest(
        root,
        {
          writes: [{ path: "outside-alias/future", scope: "exact" }],
        },
        "within-workspace",
      ),
    ).rejects.toBeInstanceOf(WorkspaceEscapeError);
  });

  // Coverage (c)〜(e): safety 近似と ownership を混同しない。
  it.each(["deny", "within-workspace"] as const)(
    "uses policy-specific glob safety and preserves producer ownership: %s",
    async (policy) => {
      const result = await normalizeResourceManifest(
        root,
        {
          reads: [{ path: "dir/*.json", scope: "exact" }],
          writes: [{ path: "dir/**/output", scope: "exact" }],
          produces: [{ path: "dir/**", scope: "subtree" }],
        },
        policy,
      );
      const safetyPath = policy === "deny" ? path.join(root, "dir") : root;
      expect(result.reads).toEqual([claim(safetyPath, "subtree")]);
      expect(result.writes).toEqual([claim(safetyPath, "subtree", "exclusive")]);
      expect(result.produces).toEqual([claim(path.join(root, "dir"), "subtree", "exclusive")]);
    },
  );

  it("widens glob safety to cover canonical targets outside the literal ancestor", async (ctx) => {
    await mkdir(path.join(root, "dir"));
    await mkdir(path.join(root, "other"));
    await writeFile(path.join(root, "other", "target.json"), "{}");
    await junction(path.join(root, "other"), path.join(root, "dir", "link"), ctx);
    const result = await normalizeResourceManifest(
      root,
      {
        writes: [{ path: "dir/**/*.json", scope: "exact" }],
        reads: [],
      },
      "within-workspace",
    );
    const target = await normalizeResourceClaim(
      root,
      { path: "dir/link/target.json", scope: "exact" },
      "shared",
      "within-workspace",
    );
    expect(result.writes.some((c) => claimContains(c, target))).toBe(true);
  });

  it("checks the literal ancestor boundary before widening glob safety", async (ctx) => {
    await junction(path.dirname(root), path.join(root, "outside"), ctx);
    await expect(
      normalizeResourceManifest(
        root,
        {
          reads: [{ path: "outside/*.json", scope: "exact" }],
        },
        "within-workspace",
      ),
    ).rejects.toBeInstanceOf(WorkspaceEscapeError);
  });

  it.each(["*", "dir/?", "dir/[abc]", "dir/{a,b}", "dir/+(a|b)", "!dir/*.json"])(
    "approximates glob/negation safely: %s",
    async (pattern) => {
      const result = await normalize({ reads: [{ path: pattern, scope: "exact" }], writes: [] });
      const ancestor = pattern.startsWith("!") || pattern === "*" ? root : path.join(root, "dir");
      expect(result.reads).toEqual([claim(ancestor, "subtree")]);
    },
  );

  it.runIf(process.platform === "win32")(
    "treats Windows backslashes as separators in globs",
    async () => {
      expect((await normalize({ reads: [{ path: "dir\\*.json", scope: "exact" }] })).reads).toEqual(
        [claim(path.join(root, "dir"), "subtree")],
      );
    },
  );

  it.skipIf(process.platform === "win32")(
    "cuts the POSIX literal ancestor before a backslash escape",
    async () => {
      expect(
        (await normalize({ reads: [{ path: "d\\ir/*.json", scope: "exact" }] })).reads,
      ).toEqual([claim(root, "subtree")]);
    },
  );

  it.each(["dir/*.json", "dir/**/file", "dir/{a,b}", "!dir/**"])(
    "rejects partial or ambiguous producers: %s",
    async (pattern) => {
      await expect(
        normalize({ produces: [{ path: pattern, scope: "subtree" }] }),
      ).rejects.toMatchObject({ code: "INVALID_REQUEST" });
    },
  );

  it("requires explicit subtree for directory ownership shorthand", async () => {
    await expect(
      normalize({ produces: [{ path: "dir/**", scope: "exact" }] }),
    ).rejects.toMatchObject({ code: "INVALID_REQUEST" });
    expect((await normalize({ produces: [{ path: "**", scope: "subtree" }] })).produces).toEqual([
      claim(root, "subtree", "exclusive"),
    ]);
  });

  it.each(["dir/*/../file", "dir/**/../../other", "!dir/../file"])(
    "rejects traversal hidden behind glob/negation approximation: %s",
    async (pattern) => {
      await expect(normalize({ reads: [{ path: pattern, scope: "exact" }] })).rejects.toMatchObject(
        { code: "INVALID_REQUEST" },
      );
    },
  );
});

describe("pure resource predicates", () => {
  const base = path.resolve("not-created-resource-fixture");
  const at = (p: string, scope: ResourceScope = "exact", mode: ResourceClaim["mode"] = "shared") =>
    claim(path.join(base, p), scope, mode);

  it.each([
    ["a", "exact", "a", "exact", true],
    ["a", "exact", "a", "subtree", false],
    ["a", "subtree", "a", "exact", true],
    ["a", "subtree", "a", "subtree", true],
    ["a", "subtree", "a/b", "exact", true],
    ["a", "subtree", "a/b", "subtree", true],
    ["a", "subtree", "ab", "exact", false],
    ["a/b", "subtree", "a", "exact", false],
    ["a", "exact", "a/b", "exact", false],
  ] as const)("containment %s/%s covers %s/%s: %s", (a, as, b, bs, expected) => {
    expect(resourceContains(at(a, as).resource, at(b, bs).resource)).toBe(expected);
  });

  it.each([
    ["shared", "shared", true, false],
    ["shared", "exclusive", false, true],
    ["exclusive", "shared", true, true],
    ["exclusive", "exclusive", true, true],
  ] as const)("mode coverage/conflict %s→%s", (a, b, contains, conflict) => {
    expect(claimContains(at("a", "subtree", a), at("a/b", "exact", b))).toBe(contains);
    expect(claimsConflict(at("a", "subtree", a), at("a/b", "exact", b))).toBe(conflict);
  });

  it("checks symmetric overlap/conflict and disjoint paths without filesystem state", () => {
    const a = at("a", "subtree", "exclusive");
    const b = at("a/b", "exact");
    const sibling = at("ab", "subtree", "exclusive");
    expect(resourcesOverlap(a.resource, b.resource)).toBe(true);
    expect(resourcesOverlap(b.resource, a.resource)).toBe(true);
    expect(claimsConflict(a, b)).toBe(true);
    expect(claimsConflict(b, a)).toBe(true);
    expect(claimsConflict(a, sibling)).toBe(false);
    expect(resourceKey(a.resource)).not.toBe(resourceKey(at("a").resource));
  });

  it("compares filesystem-root resources correctly", () => {
    const filesystemRoot = path.parse(base).root;
    expect(resourceContains(claim(filesystemRoot, "subtree").resource, at("a").resource)).toBe(
      true,
    );
  });

  it("includes every manifest bucket in conflict detection", () => {
    const read = manifest([at("a/b")]);
    expect(manifestsConflict(read, manifest([at("a/b")]))).toBe(false);
    expect(manifestsConflict(read, manifest([at("a", "subtree", "exclusive")]))).toBe(true);
    expect(
      manifestsConflict(read, {
        reads: [],
        writes: [],
        produces: [at("a", "subtree", "exclusive")],
      }),
    ).toBe(true);
    expect(manifestsConflict(read, manifest([at("ab", "subtree", "exclusive")]))).toBe(false);
    expect(manifestsConflict(manifest([]), read)).toBe(false);
  });
});
