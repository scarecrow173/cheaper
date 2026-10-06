// Dumps TypeScript `packages/core` behavior into deterministic JSON golden
// fixtures consumed by the Rust `cheaper-core` replay harness
// (crates/cheaper-core/tests/golden/). Run after `pnpm --filter
// @cheaper/core build`; re-running must produce no diff.
//
// Format: each fixture file is { "platform", "cases": [...] }. Cases carry
// the operation (`op`), inputs, optional `files`/`symlinks` filesystem
// setup (written under a per-case tmp dir), and `expected` — the result
// `{ok: ...}` or error `{kind, ...}`. Absolute paths are replaced by
// `<ROOT>` (canonical workspace root), `<OUT>` (its parent), and `<BASE>`
// tokens; the Rust side re-binds the tokens to its own tmp dir. `axis`
// marks platform-sensitive cases ("win32"/"posix"); "any" runs on both.

import { mkdirSync, readFileSync, realpathSync, symlinkSync, writeFileSync } from "node:fs";
import { mkdtemp, mkdir, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = fileURLToPath(new URL("..", import.meta.url));
const core = await import(new URL("../packages/core/dist/index.js", import.meta.url).href);
const OUT_DIR = path.join(repoRoot, "crates", "cheaper-core", "tests", "golden", "fixtures");
const PLATFORM = process.platform;

// ---------- deterministic output ----------

function sortKeys(v) {
  if (Array.isArray(v)) return v.map(sortKeys);
  if (v !== null && typeof v === "object") {
    const o = {};
    for (const k of Object.keys(v).sort()) o[k] = sortKeys(v[k]);
    return o;
  }
  return v;
}

const written = [];
function writeFixture(rel, payload) {
  const p = path.join(OUT_DIR, rel);
  mkdirSync(path.dirname(p), { recursive: true });
  writeFileSync(p, `${JSON.stringify(sortKeys(payload), null, 2)}\n`);
  written.push(`${rel} (${payload.cases?.length ?? 0} cases)`);
}

// ---------- error capture ----------

/**
 * Per the plan's normalization rules: schema `errors` arrays keep only
 * `instancePath` (message wording differs between ajv and the Rust
 * `jsonschema` crate) and `errno` diagnostics are dropped (the raw value
 * is OS-specific).
 */
function normalizeDetails(v) {
  if (Array.isArray(v)) return v.map(normalizeDetails);
  if (v !== null && typeof v === "object") {
    const o = {};
    for (const [k, val] of Object.entries(v)) {
      if (k === "errno") continue;
      if (k === "errors" && Array.isArray(val)) {
        o[k] = val.map((e2) => ({ instancePath: e2?.instancePath ?? "" }));
        continue;
      }
      o[k] = normalizeDetails(val);
    }
    return o;
  }
  return v;
}

function cap(e) {
  if (e instanceof core.CheaperError) {
    return {
      kind: "cheaper",
      code: e.code,
      exitCode: e.exitCode,
      retryable: e.retryable,
      details: e.details === undefined ? null : normalizeDetails(e.details),
    };
  }
  if (e instanceof core.WorkspaceEscapeError) {
    return {
      kind: "escape",
      input: e.input,
      reason: e.message.replace(/^workspace escape: /, ""),
    };
  }
  if (e instanceof core.FileTooLargeError) {
    return { kind: "tooLarge", path: e.path, bytes: e.bytes, maxBytes: e.maxBytes };
  }
  if (e instanceof RangeError) return { kind: "range", message: e.message };
  if (e instanceof Error && typeof e.code === "string") return { kind: "io" };
  return { kind: "other", name: e?.name ?? typeof e, message: String(e?.message ?? e) };
}

async function attempt(fn) {
  try {
    return { ok: await fn() };
  } catch (e) {
    return { error: cap(e) };
  }
}

// ---------- tmp dirs, files, scrub ----------

const TMP_BASE = await mkdtemp(path.join(os.tmpdir(), "cheaper-golden-"));

async function writeFiles(dir, files) {
  for (const [rel, content] of Object.entries(files ?? {})) {
    const p = path.join(dir, ...rel.split("/"));
    await mkdir(path.dirname(p), { recursive: true });
    if (content !== null && typeof content === "object" && "base64" in content) {
      writeFileSync(p, Buffer.from(content.base64, "base64"));
    } else {
      writeFileSync(p, String(content), "utf8");
    }
  }
}

function makeSymlinks(dir, outDir, list) {
  for (const s of list ?? []) {
    const target = s.target === "<OUT>" ? outDir : path.join(dir, ...s.target.split("/"));
    const link = path.join(dir, ...s.link.split("/"));
    const type = s.type ?? (process.platform === "win32" ? "junction" : "dir");
    symlinkSync(target, link, type);
  }
}

/**
 * `needles`: [[token, absolutePath], ...] applied longest-first so nested
 * roots win. Both `\` and `/` spellings are replaced (display paths are
 * POSIX, `searched`/real paths are platform-native).
 */
function scrub(v, needles) {
  const list = [...needles].sort((a, b) => b[1].length - a[1].length);
  const sub = (s) => {
    for (const [token, p] of list) {
      s = s
        .split(p)
        .join(token)
        .split(p.replaceAll("\\", "/"))
        .join(token)
        // JSON.stringify'd paths (e.g. `resourceKey` output) carry
        // doubled backslashes — scrub that escaped form too so fixtures
        // never embed machine paths.
        .split(p.replaceAll("\\", "\\\\"))
        .join(token);
    }
    return s;
  };
  const walk = (x) => {
    if (typeof x === "string") return sub(x);
    if (Array.isArray(x)) return x.map(walk);
    if (x !== null && typeof x === "object") {
      const o = {};
      for (const [k, val] of Object.entries(x)) o[k] = walk(val);
      return o;
    }
    return x;
  };
  return walk(v);
}

/** Inverse of `scrub`: binds path tokens in recorded case inputs back to
 * real paths so the TS implementation runs against the same absolute
 * values the Rust replay will construct. */
function unScrub(v, needles) {
  const map = new Map(needles.map(([token, p]) => [token, p]));
  const walk = (x) => {
    if (typeof x === "string") {
      let s = x;
      for (const [token, p] of map) s = s.split(token).join(p);
      return s;
    }
    if (Array.isArray(x)) return x.map(walk);
    if (x !== null && typeof x === "object") {
      const o = {};
      for (const [k, val] of Object.entries(x)) o[k] = walk(val);
      return o;
    }
    return x;
  };
  return walk(v);
}

/** Per-case context: fresh canonicalized root + sibling "outside" dir. */
async function caseDir(index, files, rootRel, symlinks) {
  const dir = path.join(TMP_BASE, `c${index}`);
  await mkdir(dir, { recursive: true });
  await writeFiles(dir, files);
  const outDir = path.join(TMP_BASE, "outside");
  await mkdir(outDir, { recursive: true });
  const root = realpathSync(rootRel ? path.join(dir, rootRel) : dir);
  try {
    makeSymlinks(dir, outDir, symlinks);
  } catch {
    return undefined; // symlinks unavailable → skip the case entirely
  }
  const out = realpathSync(outDir);
  const base = realpathSync(TMP_BASE);
  return {
    dir: realpathSync(dir),
    root,
    needles: [
      ["<ROOT>", root],
      ["<OUT>", out],
      ["<ABS>", path.resolve("golden-pred-base")],
      ["<BASE>", base],
    ],
  };
}

function withPlatform(platform, fn) {
  const desc = Object.getOwnPropertyDescriptor(process, "platform");
  Object.defineProperty(process, "platform", { value: platform });
  try {
    return fn();
  } finally {
    Object.defineProperty(process, "platform", desc);
  }
}

// ---------- dumps of composite shapes ----------

const dumpModel = (m) =>
  m.cost === undefined
    ? { provider: m.provider, id: m.id }
    : { provider: m.provider, id: m.id, cost: m.cost };

const dumpPricing = (p) => (p === undefined ? null : { ...p });

const dumpClassification = (c) => ({
  tier: c.tier,
  source: c.source,
  ...(c.auto !== undefined ? { auto: c.auto } : {}),
});

const dumpResolved = (r) => ({
  model: dumpModel(r.model),
  tier: r.tier,
  tierSource: r.tierSource,
  ...(r.pricing !== undefined ? { pricing: dumpPricing(r.pricing) } : {}),
  ...(r.auto !== undefined ? { auto: r.auto } : {}),
  selection: r.selection,
});

const dumpClaim = (c) => ({ resource: c.resource, mode: c.mode });
const dumpManifest = (m) => ({
  reads: m.reads.map(dumpClaim),
  writes: m.writes.map(dumpClaim),
  produces: m.produces.map(dumpClaim),
});

const dumpSchemaRef = (r) => ({ ...r });

function dumpWorkerDef(def) {
  return {
    name: def.name,
    description: def.description,
    tools: [...def.tools],
    cheaper: def.cheaper,
    instructions: def.instructions,
    origin: def.origin,
    sourcePath: def.sourcePath,
    directlyComparable: core.isDirectlyComparableWorker(def),
    inputSchema: def.inputSchema === undefined ? null : { schema: def.inputSchema.schema },
    outputSchema: { schema: def.outputSchema.schema },
  };
}

function dumpPolicy(p) {
  return JSON.parse(JSON.stringify(p));
}

// ---------- shared worker file text ----------

const OUT_SCHEMA = { type: "object", required: ["ok"], properties: { ok: { type: "boolean" } } };
const IN_SCHEMA = {
  type: "object",
  required: ["q"],
  properties: { q: { type: "string" } },
};

function workerMd(fm) {
  const lines = [];
  for (const [k, v] of Object.entries(fm)) {
    if (v === undefined) continue;
    if (k === "cheaper") {
      lines.push("cheaper:");
      const push = (prefix, obj) => {
        for (const [k2, v2] of Object.entries(obj)) {
          if (v2 === undefined) continue;
          if (v2 !== null && typeof v2 === "object" && !Array.isArray(v2)) {
            lines.push(`${prefix}${k2}:`);
            push(`${prefix}  `, v2);
          } else {
            lines.push(`${prefix}${k2}: ${JSON.stringify(v2)}`);
          }
        }
      };
      push("  ", v);
    } else {
      lines.push(`${k}: ${JSON.stringify(v)}`);
    }
  }
  return `---\n${lines.join("\n")}\n---\nDo things.\n`;
}

const BASE_FM = {
  name: "w",
  description: "d",
  tools: ["read"],
  cheaper: { model: { tier: "low" }, output: { schema: { inline: OUT_SCHEMA } } },
};

// ===================================================================
// errors.json — ERROR_TABLE contract
// ===================================================================

{
  const table = {};
  for (const code of Object.keys(core.ERROR_TABLE).sort()) {
    table[code] = {
      exitCode: core.ERROR_TABLE[code].exitCode,
      retryable: core.ERROR_TABLE[code].retryable,
    };
  }
  const e = new core.CheaperError("TIMEOUT", "timed out");
  writeFixture("errors.json", {
    platform: PLATFORM,
    table,
    cases: [
      {
        name: "CheaperError carries table values",
        expected: {
          ok: {
            code: e.code,
            exitCode: e.exitCode,
            retryable: e.retryable,
            message: e.message,
          },
        },
      },
      {
        name: "isCheaperError discriminates",
        expected: {
          ok: [
            core.isCheaperError(new core.CheaperError("ABORTED", "x")),
            core.isCheaperError(new Error("x")),
            core.isCheaperError("x"),
            core.isCheaperError(null),
          ],
        },
      },
    ],
  });
}

// ===================================================================
// duration.json — parseDuration
// ===================================================================

{
  const cases = [];
  const push = (input) =>
    cases.push({
      name: `parseDuration(${JSON.stringify(input)})`,
      input,
      expected: (() => {
        try {
          return { ok: { ms: core.parseDuration(input) } };
        } catch (e) {
          return { error: cap(e) };
        }
      })(),
    });
  for (const input of ["500ms", "1ms", "30s", "5m", "2h"]) push(input);
  // JS `Number` semantics: integer text parses through f64 — the 2^53
  // band rounds rather than errors.
  push("9007199254740993ms"); // Number() rounds to 2^53 → 9007199254740992
  push("9223372036854775807ms"); // 2^63-1 → rounds to 2^63
  for (const input of [
    "0s",
    "0ms",
    "01s",
    "10",
    "s",
    "1.5s",
    "1d",
    "-5s",
    "",
    "ms",
    "10ss",
    " 5s",
    "5s ",
    "1.0h",
  ])
    push(input);
  // Risk #8: TS returns a finite f64 here; the Rust port rejects
  // overflow with `invalid duration`. Record the TS value for
  // documentation and flag the intended divergence.
  cases.push({
    name: "u64 overflow is rejected by the Rust port",
    input: "999999999999999999999h",
    rustOverflow: true,
    expected: { ok: { ms: core.parseDuration("999999999999999999999h") } },
  });
  writeFixture("duration.json", { platform: PLATFORM, cases });
}

// ===================================================================
// vocabulary.json — normalizeToolNames / TOOL_NAMES / aliases
// ===================================================================

{
  const cases = [];
  const push = (name, input) =>
    cases.push({
      name,
      input,
      expected: (() => {
        try {
          return { ok: core.normalizeToolNames(input) };
        } catch (e) {
          return { error: cap(e) };
        }
      })(),
    });
  push("canonical passthrough", ["read", "grep"]);
  push("aliases", ["Read", "Grep", "Glob", "Write", "Edit"]);
  push("full canonical set", [...core.TOOL_NAMES]);
  for (const [name, input] of [
    ["undefined", undefined],
    ["null", null],
    ["string", "read"],
    ["number", 5],
    ["object", {}],
    ["empty array", []],
    ["non-string entry", ["read", 3]],
    ["unknown tool", ["bash"]],
    ["case variant", ["READ"]],
    ["duplicate after alias", ["read", "Read"]],
  ])
    push(name, input);
  writeFixture("vocabulary.json", {
    platform: PLATFORM,
    toolNames: [...core.TOOL_NAMES],
    aliases: { ...core.TOOL_ALIASES },
    cases,
  });
}

// ===================================================================
// dirs.json — resolveUserConfigDir / resolveConfigDirs
// ===================================================================

{
  const cases = [];
  const push = (name, opts) =>
    cases.push({
      name,
      env: opts.env ?? null,
      homedir: opts.homedir ?? null,
      cwd: opts.cwd ?? null,
      builtinDir: opts.builtinDir ?? null,
      expected: { ok: { ...core.resolveConfigDirs(opts) } },
    });
  push("CHEAPER_CONFIG_DIR wins", {
    cwd: "/work/proj",
    builtinDir: "/builtin",
    homedir: "/home/u",
    env: { CHEAPER_CONFIG_DIR: "/custom", XDG_CONFIG_HOME: "/xdg" },
  });
  push("XDG_CONFIG_HOME", {
    cwd: "/work/proj",
    builtinDir: "/builtin",
    homedir: "/home/u",
    env: { XDG_CONFIG_HOME: "/xdg" },
  });
  push("~/.config/cheaper", {
    cwd: "/work/proj",
    builtinDir: "/builtin",
    homedir: "/home/u",
    env: {},
  });
  push("empty env vars ignored", {
    cwd: "/work/proj",
    builtinDir: "/builtin",
    homedir: "/home/u",
    env: { CHEAPER_CONFIG_DIR: "", XDG_CONFIG_HOME: "" },
  });
  push("USERPROFILE home", {
    cwd: "/w",
    builtinDir: "/b",
    env: { USERPROFILE: "/up", HOME: "/h" },
  });
  push("HOME fallback", { cwd: "/w", builtinDir: "/b", env: { HOME: "/h" } });
  writeFixture("dirs.json", { platform: PLATFORM, cases });
}

// ===================================================================
// worker/parse.json — parseWorkerFile
// ===================================================================

{
  const SRC = "workers/w.md";
  const cases = [];
  const push = (name, content) =>
    cases.push({
      name,
      content,
      sourcePath: SRC,
      expected: (() => {
        try {
          const r = core.parseWorkerFile(content, SRC);
          return { ok: { frontmatter: r.frontmatter, body: r.body } };
        } catch (e) {
          return { error: cap(e) };
        }
      })(),
    });
  push("parses frontmatter + trimmed body", "---\nname: w\ntools: [read]\n---\n\n Do things.\n");
  push("no opening delimiter", "name: w\n---\nbody\n");
  push("empty content", "");
  push("indented delimiter", " ---\nname: w\n---\nbody\n");
  push("unclosed frontmatter", "---\nname: w\nbody without close\n");
  push("yaml parse error", "---\nname: [unclosed\n---\nbody\n");
  push("non-object frontmatter", "---\n- a\n- b\n---\nbody\n");
  push("empty frontmatter", "---\n\n---\nbody\n");
  push("empty body", "---\nname: w\n---\n   \n");
  push("keeps --- inside body", "---\nname: w\n---\nfirst\n---\nsecond\n");
  // YAML 1.2 core schema: y/n/yes/no/on/off are strings, not booleans.
  push("yaml 1.2 scalars stay strings", "---\nname: y\nflag: on\nother: off\n---\nbody\n");
  push("yaml 1.2 on key stays string", "---\nname: w\non: 1\n---\nbody\n");
  // JS trim strips U+FEFF — a padded close delimiter still counts.
  push("feff padded close delimiter", "---\nname: w\n---\uFEFF\nbody\n");
  push("feff only body", "---\nname: w\n---\n\uFEFF\n");
  // `split(/\r?\n/)` keeps a `\r` that is not followed by `\n`: the
  // literal "---\r" first line is not the `---` opener.
  push("literal CR at EOF", "---\r");
  push("crlf line endings", "---\r\nname: w\r\n---\r\nbody\r\n");
  writeFixture("worker/parse.json", { platform: PLATFORM, cases });
}

// ===================================================================
// worker/schema.json — parseSchemaRef / loadSchema / compileSchema
// ===================================================================

{
  const SRC = "workers/w.md";
  const cases = [];
  const push = (name, caseDef) => cases.push({ name, ...caseDef });
  for (const [name, raw] of [
    ["file ref", { file: "s.json" }],
    ["inline ref", { inline: { type: "object" } }],
    ["empty", {}],
    ["both keys", { file: "a.json", inline: {} }],
    ["non-object", "x"],
    ["null", null],
    ["file non-string", { file: 3 }],
    ["inline non-object", { inline: "x" }],
  ])
    push(`parseSchemaRef ${name}`, {
      op: "parseSchemaRef",
      raw,
      kind: "output",
      sourcePath: SRC,
      expected: (() => {
        try {
          return { ok: { ref: dumpSchemaRef(core.parseSchemaRef(raw, "output", SRC)) } };
        } catch (e) {
          return { error: cap(e) };
        }
      })(),
    });

  let idx = 0;
  const fsCase = async (name, files, op, extra = {}) => {
    const c = await caseDir(`s${idx++}`, files);
    const expected = await attempt(async () => {
      if (op === "loadSchema")
        return { schema: await core.loadSchema(extra.ref, c.root, extra.kind) };
      if (op === "compileValidate") {
        const compiled = core.compileSchema(extra.schema, extra.kind);
        const r = compiled.validate(extra.data);
        // instancePath is the comparison contract; ajv and jsonschema
        // message wording differs.
        return r.ok
          ? { ok: true }
          : { ok: false, errors: r.errors.map((e) => ({ instancePath: e.instancePath })) };
      }
      if (op === "compile") {
        core.compileSchema(extra.schema, extra.kind);
        return { compiled: true };
      }
      throw new Error(`op ${op}`);
    });
    push(name, {
      op,
      files,
      ...extra,
      expected: scrub(expected, c.needles),
    });
  };

  await fsCase("loadSchema .json", { "s.json": JSON.stringify({ type: "string" }) }, "loadSchema", {
    ref: { file: "s.json" },
    kind: "output",
  });
  await fsCase("loadSchema .yaml", { "s.yaml": "type: string\n" }, "loadSchema", {
    ref: { file: "s.yaml" },
    kind: "output",
  });
  await fsCase("loadSchema .yml", { "s.yml": "type: number\n" }, "loadSchema", {
    ref: { file: "s.yml" },
    kind: "output",
  });
  await fsCase("loadSchema output missing", {}, "loadSchema", {
    ref: { file: "no.json" },
    kind: "output",
  });
  await fsCase("loadSchema input missing", {}, "loadSchema", {
    ref: { file: "no.json" },
    kind: "input",
  });
  await fsCase("loadSchema bad extension", { "s.txt": "{}" }, "loadSchema", {
    ref: { file: "s.txt" },
    kind: "output",
  });
  await fsCase("loadSchema bad json", { "bad.json": "{not json" }, "loadSchema", {
    ref: { file: "bad.json" },
    kind: "input",
  });
  await fsCase("loadSchema inline", {}, "loadSchema", {
    ref: { inline: { type: "object" } },
    kind: "input",
  });
  await fsCase("compileSchema validate", {}, "compileValidate", {
    schema: { type: "object", required: ["a"], properties: { a: { type: "string" } } },
    kind: "output",
    data: { a: "x" },
  });
  await fsCase("compileSchema invalid data", {}, "compileValidate", {
    schema: { type: "object", required: ["a"], properties: { a: { type: "string" } } },
    kind: "output",
    data: { a: 1 },
  });
  await fsCase("compileSchema missing required", {}, "compileValidate", {
    schema: { type: "object", required: ["a"], properties: { a: { type: "string" } } },
    kind: "output",
    data: {},
  });
  await fsCase("compileSchema rejects draft-07", {}, "compile", {
    schema: { $schema: "http://json-schema.org/draft-07/schema#", type: "object" },
    kind: "output",
  });
  await fsCase("compileSchema rejects invalid schema", {}, "compile", {
    schema: { type: "nonsense" },
    kind: "input",
  });
  // AJV strict mode: unknown keywords and (with no format plugin
  // registered) any `format` are compile-time rejections.
  await fsCase("compileSchema unknown keyword", {}, "compile", {
    schema: { type: "object", propertiez: {} },
    kind: "output",
  });
  await fsCase("compileSchema nested unknown keyword", {}, "compile", {
    schema: { type: "object", properties: { a: { typoe: 1 } } },
    kind: "input",
  });
  await fsCase("compileSchema unknown format", {}, "compile", {
    schema: { type: "string", format: "email" },
    kind: "output",
  });
  await fsCase("compileSchema unknown format uuid", {}, "compile", {
    schema: { type: "string", format: "uuid" },
    kind: "input",
  });
  // strictRequired: every `required` member must appear in `properties`.
  await fsCase("compileSchema strict required", {}, "compile", {
    schema: { type: "object", properties: { a: {} }, required: ["b"] },
    kind: "input",
  });
  // `then`/`else` are only meaningful with `if`.
  await fsCase("compileSchema then without if", {}, "compile", {
    schema: { then: { type: "string" } },
    kind: "output",
  });
  // Union types: ["string","null"] is a legal nullable; ["a","b"]
  // without null and >2 members are strict errors.
  await fsCase("compileSchema nullable type ok", {}, "compile", {
    schema: { type: ["string", "null"] },
    kind: "output",
  });
  await fsCase("compileSchema union type rejects", {}, "compile", {
    schema: { type: ["string", "number"] },
    kind: "input",
  });
  await fsCase("compileSchema 3-member union rejects", {}, "compile", {
    schema: { type: ["string", "number", "null"] },
    kind: "input",
  });
  // `$defs` is lazy: unreferenced bad content compiles, but a `$ref`
  // reaches it and exposes the unknown keyword.
  await fsCase("compileSchema lazy defs unreferenced", {}, "compile", {
    schema: { type: "object", $defs: { a: { bogusKeyword: 1 } } },
    kind: "output",
  });
  await fsCase("compileSchema ref reaches defs", {}, "compile", {
    schema: { $ref: "#/$defs/a", $defs: { a: { bogusKeyword: 1 } } },
    kind: "input",
  });
  // `items`/`propertyNames`/`contains` type-context checks.
  await fsCase("compileSchema items on object type", {}, "compile", {
    schema: { type: "object", items: { type: "string" } },
    kind: "input",
  });
  writeFixture("worker/schema.json", { platform: PLATFORM, cases });
}

// ===================================================================
// worker/validate.json — validateWorkerDefinition / input / writeScope
// ===================================================================

{
  const cases = [];
  let idx = 0;
  const push = async (name, { files = {}, content, sourcePath = "w.md", origin = "project", op, input }) => {
    const c = await caseDir(`v${idx++}`, files);
    const sp = path.join(c.dir, sourcePath);
    const expected = await attempt(async () => {
      const parsed = core.parseWorkerFile(content, sp);
      const def = await core.validateWorkerDefinition(parsed, { sourcePath: sp, origin });
      if (op === "validateWorkerInput") {
        try {
          core.validateWorkerInput(def, input);
          return { inputOk: true };
        } catch (e) {
          const err = cap(e);
          return { error: err };
        }
      }
      if (op === "extractWriteScope") {
        try {
          const s = core.extractWriteScope(def, input);
          return { scope: s === undefined ? null : [...s] };
        } catch (e) {
          return { error: cap(e) };
        }
      }
      return { def: dumpWorkerDef(def) };
    });
    cases.push({
      name,
      op: op ?? "validate",
      files,
      content,
      sourcePath,
      origin,
      input: input ?? null,
      expected: scrub(expected, c.needles),
    });
  };

  const md = (over) => {
    const fm = structuredClone(BASE_FM);
    for (const [k, v] of Object.entries(over ?? {})) fm[k] = v;
    return workerMd(fm);
  };

  await push("valid definition", { content: md() });
  await push("ignores unknown top-level fields", { content: md({ extra: "ignored" }) });
  await push("invalid name", { content: md({ name: "Bad_Name" }) });
  await push("stem mismatch", { content: md({ name: "w" }), sourcePath: "other.md" });
  await push("empty name", { content: md({ name: "" }) });
  await push("empty description", { content: md({ description: "" }) });
  // JS trim strips U+FEFF — a FEFF-only description is missing.
  await push("feff description missing", {
    content:
      "---\nname: w\ndescription: \"\\uFEFF\"\ntools: [read]\ncheaper:\n  model: {tier: low}\n  output: {schema: {inline: {type: object}}}\n---\nbody\n",
  });
  // YAML 1.2: `on` is a string, so it is an accepted writeScope value.
  await push("writeScope yaml-1.2 on is string", {
    content:
      "---\nname: w\ndescription: d\ntools: [read]\ncheaper:\n  model: {tier: low}\n  output: {schema: {inline: {type: object}}}\n  writeScope: on\n---\nbody\n",
  });
  await push("missing tools", { content: md({ tools: undefined }) });
  await push("missing cheaper", { content: md({ cheaper: undefined }) });
  await push("missing output", {
    content: md({ cheaper: { model: { tier: "low" } } }),
  });
  await push("unknown cheaper key", {
    content: md({
      cheaper: {
        model: { tier: "low" },
        output: { schema: { inline: OUT_SCHEMA } },
        bogus: 1,
      },
    }),
  });
  await push("unknown model key", {
    content: md({
      cheaper: { model: { tier: "low", extra: 1 }, output: { schema: { inline: OUT_SCHEMA } } },
    }),
  });
  await push("workload accepted", {
    content: md({
      cheaper: {
        model: { tier: "low", workload: { outputPerInput: 0.05 } },
        output: { schema: { inline: OUT_SCHEMA } },
      },
    }),
  });
  for (const [label, v] of [
    ["string", "many"],
    ["negative", -0.1],
  ]) {
    // YAML can't express NaN in the md text; "string" case uses a quoted string.
    const fm = `---\nname: w\ndescription: d\ntools: [read]\ncheaper:\n  model:\n    tier: low\n    workload:\n      outputPerInput: ${JSON.stringify(v)}\n  output:\n    schema:\n      inline:\n        type: object\n---\nbody\n`;
    await push(`workload ${label}`, { content: fm });
  }
  await push("workload unknown key", {
    content:
      "---\nname: w\ndescription: d\ntools: [read]\ncheaper:\n  model:\n    tier: low\n    workload:\n      outputPerInput: 0.1\n      extra: 1\n  output:\n    schema:\n      inline:\n        type: object\n---\nbody\n",
  });
  await push("invalid tier", {
    content: md({
      cheaper: { model: { tier: "ultra" }, output: { schema: { inline: OUT_SCHEMA } } },
    }),
  });
  await push("file-based output schema", {
    files: { "out.schema.json": JSON.stringify(OUT_SCHEMA) },
    content: md({
      cheaper: { model: { tier: "medium" }, output: { schema: { file: "out.schema.json" } } },
    }),
  });
  await push("input schema ok", {
    op: "validateWorkerInput",
    content: md({
      cheaper: {
        model: { tier: "low" },
        input: { schema: { inline: IN_SCHEMA } },
        output: { schema: { inline: OUT_SCHEMA } },
      },
    }),
    input: { q: "x" },
  });
  await push("input schema violation", {
    op: "validateWorkerInput",
    content: md({
      cheaper: {
        model: { tier: "low" },
        input: { schema: { inline: IN_SCHEMA } },
        output: { schema: { inline: OUT_SCHEMA } },
      },
    }),
    input: { q: 1 },
  });
  await push("no input schema accepts anything", {
    op: "validateWorkerInput",
    content: md(),
    input: { anything: true },
  });
  await push("writeScope strings+objects", {
    op: "extractWriteScope",
    content: md({
      cheaper: {
        model: { tier: "low" },
        output: { schema: { inline: OUT_SCHEMA } },
        writeScope: "targets",
      },
    }),
    input: { targets: ["./a", { path: "b\\c" }, "./a"] },
  });
  await push("writeScope empty", {
    op: "extractWriteScope",
    content: md({
      cheaper: {
        model: { tier: "low" },
        output: { schema: { inline: OUT_SCHEMA } },
        writeScope: "targets",
      },
    }),
    input: { targets: [] },
  });
  for (const [label, input] of [
    ["missing", {}],
    ["null input", null],
    ["not array", { targets: "a" }],
    ["bad item", { targets: [1] }],
    ["bad path", { targets: [{ path: 3 }] }],
  ]) {
    await push(`writeScope invalid ${label}`, {
      op: "extractWriteScope",
      content: md({
        cheaper: {
          model: { tier: "low" },
          output: { schema: { inline: OUT_SCHEMA } },
          writeScope: "targets",
        },
      }),
      input,
    });
  }
  await push("writeScope undefined → null", {
    op: "extractWriteScope",
    content: md(),
    input: {},
  });
  cases.push({
    name: "isMutatingTool",
    op: "isMutatingTool",
    expected: {
      ok: {
        read: core.isMutatingTool("read"),
        write: core.isMutatingTool("write"),
        edit: core.isMutatingTool("edit"),
        process: core.isMutatingTool("process"),
        grep: core.isMutatingTool("grep"),
      },
    },
  });
  writeFixture("worker/validate.json", { platform: PLATFORM, cases });
}

// ===================================================================
// worker/resolve.json — resolveWorker / resolveWorkerFile
// ===================================================================

{
  const cases = [];
  let idx = 0;
  const push = async (name, { files, op, workerName, filePath, cwdRel = "." }) => {
    const c = await caseDir(`r${idx++}`, files);
    const dirs = {
      userConfigDir: path.join(c.dir, "user"),
      projectDir: path.join(c.dir, "proj", ".cheaper"),
      builtinDir: path.join(c.dir, "builtin"),
    };
    const expected = await attempt(async () => {
      if (op === "resolveWorker") return { def: dumpWorkerDef(await core.resolveWorker(workerName, dirs)) };
      if (op === "resolveWorkerFile") {
        const cwd = path.join(c.dir, cwdRel);
        return { def: dumpWorkerDef(await core.resolveWorkerFile(filePath, cwd)) };
      }
      throw new Error(op);
    });
    cases.push({
      name,
      op,
      files,
      workerName: workerName ?? null,
      filePath: filePath ?? null,
      cwdRel,
      dirs: { userConfigDir: "user", projectDir: "proj/.cheaper", builtinDir: "builtin" },
      expected: scrub(expected, c.needles),
    });
  };

  const workerFile = (name, tools = ["read"]) =>
    workerMd({
      name,
      description: "d",
      tools,
      cheaper: { model: { tier: "low" }, output: { schema: { inline: OUT_SCHEMA } } },
    });

  await push("project wins over user+builtin", {
    op: "resolveWorker",
    workerName: "w",
    files: {
      "proj/.cheaper/workers/w.md": workerFile("w"),
      "user/workers/w.md": workerFile("w", ["grep"]),
      "builtin/workers/w.md": workerFile("w", ["ls"]),
    },
  });
  await push("user wins over builtin", {
    op: "resolveWorker",
    workerName: "w",
    files: {
      "user/workers/w.md": workerFile("w", ["grep"]),
      "builtin/workers/w.md": workerFile("w", ["ls"]),
    },
  });
  await push("builtin used", {
    op: "resolveWorker",
    workerName: "w",
    files: { "builtin/workers/w.md": workerFile("w") },
  });
  await push("not found lists searched", {
    op: "resolveWorker",
    workerName: "ghost",
    files: {},
  });
  await push("invalid project worker does not fall back", {
    op: "resolveWorker",
    workerName: "w",
    files: {
      "proj/.cheaper/workers/w.md": workerFile("w").replace("name: \"w\"", "name: \"other\""),
      "user/workers/w.md": workerFile("w"),
    },
  });
  await push("invalid worker name", { op: "resolveWorker", workerName: "Bad Name", files: {} });
  await push("resolveWorkerFile explicit", {
    op: "resolveWorkerFile",
    filePath: "docs/custom.md",
    files: { "docs/custom.md": workerFile("custom") },
  });
  await push("resolveWorkerFile missing", {
    op: "resolveWorkerFile",
    filePath: "docs/none.md",
    files: {},
  });
  writeFixture("worker/resolve.json", { platform: PLATFORM, cases });
}

// ===================================================================
// policy/schema.json — validatePolicySchema / normalizePolicy /
// resolveConcurrencyPolicy
// ===================================================================

const BASE_POLICY = {
  version: 1,
  execution: {
    timeout: "2m",
    maxTurns: 10,
    maxToolCalls: 20,
    concurrency: { maxOperations: 8, mutations: "serialized" },
  },
  tools: { allow: ["read", "grep"] },
  filesystem: { symlinks: "deny", denyPaths: [".env"] },
  model: { maxTier: "high", tierFallback: "higher" },
  output: { maxTokens: 4096, repairAttempts: 1 },
  retry: { transientAttempts: 2 },
};

{
  const cases = [];
  const push = (name, op, input, extra = {}) =>
    cases.push({
      name,
      op,
      input,
      ...extra,
      expected: (() => {
        try {
          if (op === "validatePolicySchema") {
            const v = core.validatePolicySchema(input, extra.mode);
            return { ok: { policy: v } };
          }
          if (op === "normalizePolicy") {
            const validated = core.validatePolicySchema(input, "complete");
            return { ok: { policy: dumpPolicy(core.normalizePolicy(validated)) } };
          }
          if (op === "resolveConcurrencyPolicy") {
            return { ok: core.resolveConcurrencyPolicy(input) };
          }
          throw new Error(op);
        } catch (e) {
          return { error: cap(e) };
        }
      })(),
    });

  push("complete valid", "validatePolicySchema", BASE_POLICY, { mode: "complete" });
  push("complete missing section", "validatePolicySchema", (() => {
    const p = structuredClone(BASE_POLICY);
    delete p.tools;
    return p;
  })(), { mode: "complete" });
  push("partial missing sections ok", "validatePolicySchema", (() => {
    const p = structuredClone(BASE_POLICY);
    delete p.tools;
    delete p.model;
    delete p.execution.concurrency;
    return p;
  })(), { mode: "partial" });
  push("complete requires concurrency fields", "validatePolicySchema", (() => {
    const p = structuredClone(BASE_POLICY);
    delete p.execution.concurrency.maxOperations;
    return p;
  })(), { mode: "complete" });
  push("partial allows concurrency omission", "validatePolicySchema", (() => {
    const p = structuredClone(BASE_POLICY);
    delete p.execution.concurrency;
    return p;
  })(), { mode: "partial" });
  push("unknown top-level field", "validatePolicySchema", { ...BASE_POLICY, bogus: 1 }, {
    mode: "complete",
  });
  push("unknown nested field", "validatePolicySchema", {
    ...structuredClone(BASE_POLICY),
    execution: { ...BASE_POLICY.execution, x: 1 },
  }, { mode: "complete" });
  push("wrong version", "validatePolicySchema", { ...structuredClone(BASE_POLICY), version: 2 }, {
    mode: "complete",
  });
  push("missing version", "validatePolicySchema", (() => {
    const p = structuredClone(BASE_POLICY);
    delete p.version;
    return p;
  })(), { mode: "complete" });
  push("bad duration", "validatePolicySchema", {
    ...structuredClone(BASE_POLICY),
    execution: { ...BASE_POLICY.execution, timeout: "5x" },
  }, { mode: "complete" });
  push("process shell task rejected pre-merge", "validatePolicySchema", {
    ...structuredClone(BASE_POLICY),
    process: {
      maxConcurrent: 1,
      tasks: { t: { command: ["sh"], inputs: [], outputs: [] } },
    },
  }, { mode: "complete" });
  push("normalize valid policy", "normalizePolicy", BASE_POLICY);
  push("normalize fills workflow/process/concurrency", "normalizePolicy", {
    ...structuredClone(BASE_POLICY),
    execution: { timeout: "30s", maxTurns: 5, maxToolCalls: 9 },
  });
  push("normalize alias tools", "normalizePolicy", {
    ...structuredClone(BASE_POLICY),
    tools: { allow: ["Read", "Glob", "edit"] },
  });
  push("normalize unknown tool", "normalizePolicy", {
    ...structuredClone(BASE_POLICY),
    tools: { allow: ["bash"] },
  });
  push("normalize duplicate tools", "normalizePolicy", {
    ...structuredClone(BASE_POLICY),
    tools: { allow: ["read", "Read"] },
  });
  push("concurrency defaults", "resolveConcurrencyPolicy", null);
  push("concurrency valid", "resolveConcurrencyPolicy", {
    maxOperations: 3,
    mutations: "per-resource",
  });
  for (const [label, input] of [
    ["zero", { maxOperations: 0 }],
    ["float", { maxOperations: 1.5 }],
    ["bad mutations", { mutations: "bogus" }],
    ["non-number", { maxOperations: "x" }],
  ])
    push(`concurrency ${label}`, "resolveConcurrencyPolicy", input);
  // JS `??` semantics: `null` fields coalesce to the defaults — the
  // resolver does not go through the strict schema for inner fields.
  push("concurrency null maxOperations", "resolveConcurrencyPolicy", { maxOperations: null });
  push("concurrency null mutations", "resolveConcurrencyPolicy", { mutations: null });
  push("concurrency all null", "resolveConcurrencyPolicy", { maxOperations: null, mutations: null });
  writeFixture("policy/schema.json", { platform: PLATFORM, cases });
}

// ===================================================================
// policy/merge.json — mergePolicyLayers
// ===================================================================

{
  const cases = [];
  const push = (name, base, override, mode = "replace") =>
    cases.push({
      name,
      base,
      override,
      mode,
      expected: (() => {
        try {
          return {
            ok: {
              merged: core.mergePolicyLayers(base, override, { mode }),
              baseAfter: base,
              overrideAfter: override,
            },
          };
        } catch (e) {
          return { error: cap(e) };
        }
      })(),
    });
  push(
    "deep merge / scalar + array replace",
    { a: { x: 1, y: 2 }, b: [1, 2], c: "s" },
    { a: { x: 9 }, b: [3], c: "t", d: true },
  );
  push(
    "restrict takes numeric min + duration min + serialized",
    {
      execution: { concurrency: { maxOperations: 8, mutations: "serialized" } },
      workflow: { maxJobs: 4, artifacts: { maxReadBytes: 65536 } },
      process: { maxConcurrent: 2, maxOutputBytes: 100, timeout: "2m" },
    },
    {
      execution: { concurrency: { maxOperations: 16, mutations: "per-resource" } },
      workflow: { maxJobs: 1, artifacts: { maxReadBytes: 99999 } },
      process: { maxConcurrent: 8, maxOutputBytes: 50, timeout: "5m", killGrace: "1s" },
    },
    "restrict-concurrency",
  );
  push(
    "restrict mutations only tightens",
    { execution: { concurrency: { mutations: "serialized" } } },
    { execution: { concurrency: { mutations: "per-resource" } } },
    "restrict-concurrency",
  );
  push(
    "restrict duration min keeps shorter",
    { process: { timeout: "90s" } },
    { process: { timeout: "1m" } },
    "restrict-concurrency",
  );
  push(
    "replace mode ignores restrict fields",
    { process: { maxConcurrent: 2 } },
    { process: { maxConcurrent: 8 } },
  );
  writeFixture("policy/merge.json", { platform: PLATFORM, cases });
}

// ===================================================================
// policy/load.json — loadPolicy
// ===================================================================

{
  const builtinYaml = readFileSync(path.join(repoRoot, "packages", "cli", "policy.yaml"), "utf8");
  const cases = [];
  let idx = 0;
  const push = async (name, files) => {
    const c = await caseDir(`l${idx++}`, files);
    const expected = await attempt(async () => ({
      policy: dumpPolicy(
        await core.loadPolicy({
          builtinPolicyPath: path.join(c.dir, "builtin", "policy.yaml"),
          userConfigDir: path.join(c.dir, "user"),
          projectDir: path.join(c.dir, "proj", ".cheaper"),
        }),
      ),
    }));
    cases.push({
      name,
      files,
      opts: {
        builtinPolicyPath: "builtin/policy.yaml",
        userConfigDir: "user",
        projectDir: "proj/.cheaper",
      },
      expected: scrub(expected, c.needles),
    });
  };

  await push("builtin only", { "builtin/policy.yaml": builtinYaml });
  await push("user+project layering", {
    "builtin/policy.yaml": builtinYaml,
    "user/policy.yaml":
      "version: 1\nmodel:\n  maxTier: medium\nexecution:\n  timeout: 5m\ntools:\n  allow: [read]\n",
    "proj/.cheaper/policy.yaml":
      "version: 1\nexecution:\n  concurrency:\n    maxOperations: 2\nprocess:\n  timeout: 30s\n",
  });
  await push("invalid user policy errors", {
    "builtin/policy.yaml": builtinYaml,
    "user/policy.yaml": "version: 1\nbogus: 1\n",
  });
  await push("invalid project policy errors", {
    "builtin/policy.yaml": builtinYaml,
    "proj/.cheaper/policy.yaml": "version: 1\nbogus: 1\n",
  });
  await push("missing builtin errors", {
    "user/policy.yaml": "version: 1\n",
  });
  await push("unreadable builtin errors (dir)", {
    "builtin/policy.yaml/marker.txt": "x",
  });
  await push("malformed yaml errors", {
    "builtin/policy.yaml": builtinYaml,
    "user/policy.yaml": "version: [unclosed\n",
  });
  await push("non-object yaml errors", {
    "builtin/policy.yaml": builtinYaml,
    "user/policy.yaml": "- a\n- b\n",
  });
  writeFixture("policy/load.json", { platform: PLATFORM, cases });
}

// ===================================================================
// policy/restrict.json — restrictPolicy / checkCapabilities
// ===================================================================

{
  const cases = [];
  let idx = 0;
  const push = (name, restrictions, policyInput = BASE_POLICY) => {
    const base = core.normalizePolicy(core.validatePolicySchema(policyInput, "complete"));
    cases.push({
      name,
      policy: policyInput,
      restrictions,
      expected: (() => {
        try {
          return { ok: { policy: dumpPolicy(core.restrictPolicy(base, restrictions)) } };
        } catch (e) {
          return { error: cap(e) };
        }
      })(),
    });
  };
  push("no restrictions", {});
  push("timeout below policy", { timeoutMs: 60_000 });
  push("timeout equal to policy", { timeoutMs: 120_000 });
  push("timeout over policy", { timeoutMs: 300_000 });
  push("concurrency narrows", { concurrency: { maxOperations: 2 } });
  push("concurrency widens → violation", { concurrency: { maxOperations: 16 } });
  push("serialized→per-resource → violation", {
    concurrency: { mutations: "per-resource" },
  });
  // JS `??` semantics on the merged restriction: `null` fields coalesce
  // to the policy defaults (8/serialized) rather than failing or
  // treating null as a value. `concurrency: null` itself is a no-op.
  push("concurrency null maxOperations → policy default", { concurrency: { maxOperations: null } });
  push("concurrency null mutations → policy default", { concurrency: { mutations: null } });
  push("concurrency null object → no-op", { concurrency: null });
  // Against a tighter base the same `null` restriction widens back to
  // 8 → violation (null is not "no restriction"; it is the default).
  const tighterPolicy = structuredClone(BASE_POLICY);
  tighterPolicy.execution.concurrency = { maxOperations: 4, mutations: "serialized" };
  push(
    "concurrency null maxOperations widens to default → violation",
    { concurrency: { maxOperations: null } },
    tighterPolicy,
  );

  // checkCapabilities cases need a real WorkerDefinition; build one.
  const policyWith = (allow) =>
    core.normalizePolicy(
      core.validatePolicySchema(
        { ...structuredClone(BASE_POLICY), tools: { allow } },
        "complete",
      ),
    );
  const capCase = async (name, tools, allow, tier = "low", maxTier = "high") => {
    const c = await caseDir(`c${idx++}`, {
      "w.md": workerMd({
        name: "w",
        description: "d",
        tools,
        cheaper: { model: { tier }, output: { schema: { inline: OUT_SCHEMA } } },
      }),
    });
    const sp = path.join(c.dir, "w.md");
    const p = policyWith(allow);
    p.model.maxTier = maxTier;
    const expected = await attempt(async () => {
      const def = await core.validateWorkerDefinition(core.parseWorkerFile(
        readFileSync(sp, "utf8"),
        sp,
      ), { sourcePath: sp, origin: "project" });
      return { tools: core.checkCapabilities(def, p) };
    });
    cases.push({
      name,
      op: "checkCapabilities",
      policy: {
        ...structuredClone(BASE_POLICY),
        tools: { allow },
        model: { ...BASE_POLICY.model, maxTier },
      },
      files: { "w.md": workerMd({
        name: "w",
        description: "d",
        tools,
        cheaper: { model: { tier }, output: { schema: { inline: OUT_SCHEMA } } },
      }) },
      expected: scrub(expected, c.needles),
    });
  };
  await capCase("allowed tools pass", ["read"], ["read", "grep"]);
  await capCase("missing tool → violation", ["read", "write"], ["read"]);
  await capCase("tier over max → violation", ["read"], ["read"], "high", "medium");
  writeFixture("policy/restrict.json", { platform: PLATFORM, cases });
}

// ===================================================================
// policy/process.json — process policy
// ===================================================================

{
  const cases = [];
  const push = (name, op, input, extra = {}, axis = "any") =>
    cases.push({
      name,
      op,
      input,
      ...extra,
      axis,
      expected: (() => {
        try {
          if (op === "resolveProcessPolicy") return { ok: core.resolveProcessPolicy(input) };
          if (op === "isShellProcessExecutable")
            return { ok: core.isShellProcessExecutable(input) };
          if (op === "resolveProcessTask")
            return { ok: core.resolveProcessTask(extra.policy, input) };
          if (op === "processTaskResources")
            return { ok: core.processTaskResources(extra.policy, input) };
          throw new Error(op);
        } catch (e) {
          return { error: cap(e) };
        }
      })(),
    });

  push("defaults", "resolveProcessPolicy", null);
  push("empty object", "resolveProcessPolicy", {});
  push("valid tasks sorted + filled", "resolveProcessPolicy", {
    maxConcurrent: 3,
    timeout: "90s",
    maxOutputBytes: 1024,
    killGrace: "1s",
    heartbeat: { interval: "10s" },
    tasks: {
      b: { command: ["bb"], inputs: ["i"], outputs: ["o"] },
      a: { command: ["aa"], inputs: [], outputs: [], cwd: "sub", env: { K: "v" } },
    },
  });
  for (const [label, input] of [
    ["unknown field", { bogus: 1 }],
    ["bad timeout", { timeout: "5x" }],
    ["timeout over limit", { timeout: "100000000000000000000h" }],
    ["maxOutputBytes over cap", { maxOutputBytes: 16777217 }],
    ["shell executable", { tasks: { t: { command: ["bash", "-c", "x"], inputs: [], outputs: [] } } }],
    ["script shim", { tasks: { t: { command: ["run.sh"], inputs: [], outputs: [] } } }],
    ["argv shell syntax", { tasks: { t: { command: ["echo", "a|b"], inputs: [], outputs: [] } } }],
    ["argv subshell", { tasks: { t: { command: ["echo", "$(x)"], inputs: [], outputs: [] } } }],
    ["absolute input", { tasks: { t: { command: ["cc"], inputs: ["/abs"], outputs: [] } } }],
    ["traversal input", { tasks: { t: { command: ["cc"], inputs: ["a/../b"], outputs: [] } } }],
    ["drive-relative cwd", { tasks: { t: { command: ["cc"], inputs: [], outputs: [], cwd: "d:e" } } }],
    ["bad task name", { tasks: { "Bad Name": { command: ["cc"], inputs: [], outputs: [] } } }],
    ["missing command", { tasks: { t: { inputs: [], outputs: [] } } }],
  ])
    push(`invalid ${label}`, "resolveProcessPolicy", input);
  // localeCompare sorts env keys base-letter-first: "A" < "b" < "C"
  // (byte order would give A, C, b). No case collision → axis "any".
  push("env keys sort by localeCompare", "resolveProcessPolicy", {
    tasks: {
      t: { command: ["cc"], inputs: [], outputs: [], env: { C: "3", b: "1", A: "2" } },
    },
  });
  // localeCompare primary weight: punctuation < digit < letter, and
  // within punctuation `_` < `-` (UTF-16 code order gives `-` < `_`),
  // so the sort is "a_1" < "a-2" < "a0" < "b" < "z_1" < "za".
  push("task names sort punct before digits/letters", "resolveProcessPolicy", {
    tasks: {
      b: { command: ["cc"], inputs: [], outputs: [] },
      a_1: { command: ["cc"], inputs: [], outputs: [] },
      a0: { command: ["cc"], inputs: [], outputs: [] },
      "a-2": { command: ["cc"], inputs: [], outputs: [] },
      z_1: { command: ["cc"], inputs: [], outputs: [] },
      za: { command: ["cc"], inputs: [], outputs: [] },
    },
  });
  // Same class ordering inside per-task env keys.
  push("env keys sort punct before digits", "resolveProcessPolicy", {
    tasks: {
      t: { command: ["cc"], inputs: [], outputs: [], env: { ZA: "a", Z1: "b", Z_2: "c", _A: "d" } },
    },
  });
  // AJV `type: "integer"`/`"object"` reject `null` — inner-field null is
  // not a `??` fallback inside a validated policy (only `input ?? {}`
  // itself coalesces).
  for (const [label, input] of [
    ["null maxConcurrent", { maxConcurrent: null }],
    ["null tasks", { tasks: null }],
    ["null heartbeat interval", { heartbeat: { interval: null } }],
    ["null task env value", { tasks: { t: { command: ["cc"], inputs: [], outputs: [], env: { K: null } } } }],
  ])
    push(`invalid ${label}`, "resolveProcessPolicy", input);
  // win32 axis: Path/PATH collide by case-insensitive identity.
  push("env case collision win32", "resolveProcessPolicy", {
    tasks: {
      t: { command: ["cc"], inputs: [], outputs: [], env: { Path: "a", PATH: "b" } },
    },
  }, {}, "win32");
  push("env case collision posix", "resolveProcessPolicy", {
    tasks: {
      t: { command: ["cc"], inputs: [], outputs: [], env: { Path: "a", PATH: "b" } },
    },
  }, {}, "posix");
  // compute expected for each axis by stubbing the platform
  for (const c of cases) {
    if (c.axis === "win32") {
      c.expected = withPlatform("win32", () => {
        try {
          return { ok: core.resolveProcessPolicy(c.input) };
        } catch (e) {
          return { error: cap(e) };
        }
      });
    } else if (c.axis === "posix") {
      c.expected = withPlatform("linux", () => {
        try {
          return { ok: core.resolveProcessPolicy(c.input) };
        } catch (e) {
          return { error: cap(e) };
        }
      });
    }
  }
  for (const exe of [
    "sh",
    "bash",
    "cmd.exe",
    "powershell",
    "pwsh",
    "bin/zsh",
    "x.PS1",
    "run.sh",
    "a.BAT",
    "busybox",
    "shell",
    "command",
    "pwsh-tool",
    "a.shx",
  ])
    push(`isShell ${exe}`, "isShellProcessExecutable", exe);

  const resolved = core.resolveProcessPolicy({
    tasks: { build: { command: ["make"], inputs: ["src/**"], outputs: ["out.txt"] } },
  });
  push("resolveProcessTask hit", "resolveProcessTask", "build", { policy: resolved });
  push("resolveProcessTask miss", "resolveProcessTask", "nope", { policy: resolved });
  push("processTaskResources", "processTaskResources", "build", { policy: resolved });
  writeFixture("policy/process.json", { platform: PLATFORM, cases });
}

// ===================================================================
// policy/workflow.json — resolveWorkflowPolicy
// ===================================================================

{
  const cases = [];
  const push = (name, input) =>
    cases.push({
      name,
      input,
      expected: (() => {
        try {
          return { ok: core.resolveWorkflowPolicy(input) };
        } catch (e) {
          return { error: cap(e) };
        }
      })(),
    });
  push("defaults", null);
  push("empty", {});
  push("explicit", {
    maxJobs: 4,
    artifacts: { maxArtifactBytes: 1024, maxTotalBytes: 4096, maxPreviewBytes: 256, maxReadBytes: 512 },
  });
  for (const [label, input] of [
    ["unknown field", { bogus: 1 }],
    ["zero maxJobs", { maxJobs: 0 }],
    ["artifact over cap", { artifacts: { maxArtifactBytes: 16777217 } }],
    // JS `??` semantics: `null` falls back to the default; the port must
    // not treat it as an explicit value.
    ["null maxJobs", { maxJobs: null }],
    ["null artifacts field", { artifacts: { maxArtifactBytes: null } }],
    ["negative artifacts field", { artifacts: { maxTotalBytes: -1 } }],
    // `{...obj}` spreads enumerable own keys: each unknown field is
    // validated as a byte limit, in insertion order.
    ["unknown artifact field type", { artifacts: { bogus: "x" } }],
    ["artifact fields checked in order", { artifacts: { maxArtifactBytes: 4, zzz: -1, aaa: "x" } }],
    // `{..."ab"}` spreads indexed keys → `"0"` is validated → invalid.
    ["string artifacts spreads indices", { artifacts: "ab" }],
    ["maxJobs fractional", { maxJobs: 1.5 }],
    ["maxJobs string", { maxJobs: "4" }],
    ["zero maxReadBytes (min is 1)", { artifacts: { maxReadBytes: 0 } }],
  ])
    push(`invalid ${label}`, input);
  // `{...<primitive>}` spreads nothing: `true`/`null`/`0`/`""` all fall
  // back to the artifact defaults (same as `??` on the spread source).
  for (const input of [{ artifacts: true }, { artifacts: null }, { artifacts: 0 }, { artifacts: "" }])
    push(`artifacts ${JSON.stringify(input.artifacts)} → defaults`, input);
  // Risk: TS spreads `input.artifacts` so unknown keys pass through the
  // direct call; the schema-validated pipeline cannot reach that state,
  // and the Rust typed `WorkflowArtifactPolicy` drops unknown keys.
  // The replay strips unknown keys from the expected value.
  for (const [name, input] of [
    ["invalid unknown artifact field", { artifacts: { bogus: 1 } }],
    // `{...[1,2]}` spreads indexed keys → `{ "0": 1, "1": 2, ... }`.
    ["array artifacts spread indexed keys", { artifacts: [1, 2] }],
    ["unknown artifact fields keep insertion order", { artifacts: { zebra: 9, alpha: 8, maxArtifactBytes: 4 } }],
  ])
    cases.push({
      name,
      input,
      rustDropUnknownArtifactKeys: true,
      expected: (() => {
        try {
          return { ok: core.resolveWorkflowPolicy(input) };
        } catch (e) {
          return { error: cap(e) };
        }
      })(),
    });
  writeFixture("policy/workflow.json", { platform: PLATFORM, cases });
}

// ===================================================================
// model/classify.json — classify + tier + pricing + exceptions
// ===================================================================

{
  const m = (provider, id, cost) => {
    const info = { provider, id };
    if (cost !== undefined) info.cost = cost;
    return info;
  };
  const EMPTY = { overrides: {}, preferences: {} };
  const cases = [];
  const push = (name, op, input, expected) => cases.push({ name, op, input, expected });

  // tier primitives
  push("tier order", "tiers", null, { ok: { tiers: [...core.TIERS] } });
  push("compareTier", "compareTier", ["low", "medium"], { ok: core.compareTier("low", "medium") });
  push("compareTier high>low", "compareTier", ["high", "low"], { ok: core.compareTier("high", "low") });
  push("compareTier equal", "compareTier", ["medium", "medium"], { ok: core.compareTier("medium", "medium") });
  for (const [v, exp] of [
    ["low", true],
    ["high", true],
    ["ultra", false],
    [3, false],
    [null, false],
  ])
    push(`isTier ${JSON.stringify(v)}`, "isTier", v, { ok: exp });
  for (const [t, exp] of [
    ["low", "medium"],
    ["medium", "high"],
    ["high", null],
  ])
    push(`nextTier ${t}`, "nextTier", t, { ok: exp });

  // classify
  const classify = (name, models, config = EMPTY, outputPerInput) => {
    const out = core.classifyModels(models, config, {
      ...(outputPerInput !== undefined ? { outputPerInput } : {}),
    });
    push(name, "classify", { models: models.map(dumpModel), config, outputPerInput: outputPerInput ?? null }, {
      ok: models.map((mm) => dumpClassification(out.get(mm))),
    });
  };
  const t = (id, cost) => m("test", id, cost);
  classify("override wins", [t("claude-opus-4")], {
    overrides: { "test/claude-opus-4": "low" },
    preferences: {},
  });
  classify("override keyed to provider", [m("b", "x")], {
    overrides: { "a/x": "high" },
    preferences: {},
  });
  classify("curated exception", [m("ollama", "qwen3")]);
  classify("override beats exception", [m("ollama", "qwen3")], {
    overrides: { "ollama/qwen3": "high" },
    preferences: {},
  });
  classify("three tiers", [t("pricey", { input: 10, output: 10 }), t("cheap", { input: 0.1, output: 0.1 }), t("mid", { input: 1, output: 1 })]);
  classify("six models", [1, 2, 3, 4, 5, 6].map((i) => t(`m${i}`, { input: i, output: 0 })));
  classify("two models low/high", [t("a", { input: 1, output: 0 }), t("b", { input: 2, output: 0 })]);
  classify("single → medium", [t("a", { input: 5, output: 5 })]);
  classify("all tied → medium", [t("a", { input: 1, output: 1 }), t("b", { input: 1, output: 1 })]);
  classify("partial tie", [t("a", { input: 1, output: 0 }), t("b", { input: 1, output: 0 }), t("c", { input: 2, output: 0 }), t("d", { input: 3, output: 0 })]);
  classify("workload reader", [t("input-heavy", { input: 10, output: 1 }), t("output-heavy", { input: 1, output: 10 })], EMPTY, 0);
  classify("workload writer", [t("input-heavy", { input: 10, output: 1 }), t("output-heavy", { input: 1, output: 10 })], EMPTY, 10);
  classify("zero-price excluded", [m("z", "free", { input: 0, output: 0 }), t("paid", { input: 1, output: 1 })]);

  const SNAPSHOT = {
    version: "test-v1",
    updatedAt: "2026-09-01",
    models: [
      ["snap/exact", { input: 1, output: 2, cacheRead: 0.1 }],
      ["snap-wild/*", { input: 3, output: 4 }],
    ],
  };
  const snapshotObj = { ...SNAPSHOT, models: Object.fromEntries(SNAPSHOT.models) };
  const pricingCase = (name, model, snapshot) =>
    push(name, "resolvePricing", { model: dumpModel(model), snapshot }, {
      ok: { pricing: dumpPricing(core.resolvePricing(model, snapshot === null ? undefined : snapshotObj)) },
    });
  pricingCase("runtime catalog wins", m("p", "m", { input: 5, output: 25, cacheRead: 0.5 }), SNAPSHOT);
  pricingCase("zero cost is a valid observation", m("p", "m", { input: 0, output: 0 }), null);
  pricingCase("snapshot exact fills missing cost", m("snap", "exact"), SNAPSHOT);
  pricingCase("snapshot wildcard", m("snap-wild", "anything"), SNAPSHOT);
  pricingCase("unpriced → null", m("no", "pe"), SNAPSHOT);

  const EXC_TABLE = [
    ["p/exact", { tier: "high", reason: "r", source: "s", checkedAt: "2026-09-19" }],
    ["p/*", { tier: "low", reason: "r", source: "s", checkedAt: "2026-09-19" }],
  ];
  const excObj = Object.fromEntries(EXC_TABLE);
  for (const [prov, id] of [
    ["p", "exact"],
    ["p", "other"],
    ["q", "exact"],
    ["ollama", "qwen3"],
  ])
    push(`exception ${prov}/${id}`, "findModelException", { provider: prov, id, builtin: prov === "ollama" }, {
      ok: core.findModelException(prov, id, prov === "ollama" ? core.MODEL_EXCEPTIONS : excObj) ?? null,
    });
  for (const [prov, id, pattern] of [
    ["a", "m", "a/m"],
    ["a", "m", "a/*"],
    ["a", "m", "b/*"],
    ["a", "m", "a/n"],
  ])
    push(`matchModelPattern ${prov}/${id} ~ ${pattern}`, "matchModelPattern", [prov, id, pattern], {
      ok: core.matchModelPattern(prov, id, pattern),
    });
  writeFixture("model/classify.json", { platform: PLATFORM, cases });
}

// ===================================================================
// model/resolve.json + model/config.json
// ===================================================================

{
  const m = (provider, id, cost) => {
    const info = { provider, id };
    if (cost !== undefined) info.cost = cost;
    return info;
  };
  const NO_PREF = { overrides: {}, preferences: {} };
  const cfg = (overrides, preferences = {}, exclude) => ({
    overrides,
    preferences,
    ...(exclude !== undefined ? { exclude } : {}),
  });
  const cases = [];
  const push = (name, op, input, expected) => cases.push({ name, op, input, expected });

  const resolveCase = (name, available, requestedTier, policy, config, outputPerInput) => {
    const expected = (() => {
      try {
        const r = core.resolveModel({
          available,
          requestedTier,
          policy,
          config,
          ...(outputPerInput !== undefined ? { outputPerInput } : {}),
        });
        return { ok: dumpResolved(r) };
      } catch (e) {
        return { error: cap(e) };
      }
    })();
    push(name, "resolveModel", {
      available: available.map(dumpModel),
      requestedTier,
      policy,
      config,
      outputPerInput: outputPerInput ?? null,
    }, expected);
  };
  const explicitCase = (name, available, ref, maxTier, config, outputPerInput) => {
    const expected = (() => {
      try {
        const r = core.resolveExplicitModel({
          available,
          ref,
          policy: { maxTier },
          config,
          ...(outputPerInput !== undefined ? { outputPerInput } : {}),
        });
        return { ok: dumpResolved(r) };
      } catch (e) {
        return { error: cap(e) };
      }
    })();
    push(name, "resolveExplicitModel", {
      available: available.map(dumpModel),
      ref,
      maxTier,
      config,
      outputPerInput: outputPerInput ?? null,
    }, expected);
  };

  resolveCase("empty available → unavailable", [], "low", { maxTier: "high", tierFallback: "higher" }, NO_PREF);
  resolveCase("requested > maxTier", [m("p", "x")], "high", { maxTier: "medium", tierFallback: "none" }, NO_PREF);
  resolveCase("cheapest of tier wins", [m("p", "expensive", { input: 1, output: 9 }), m("p", "mid", { input: 1, output: 4 }), m("p", "cheap", { input: 0.5, output: 1 })], "low", { maxTier: "high", tierFallback: "none" }, cfg({ "p/expensive": "low", "p/cheap": "low", "p/mid": "low" }));
  resolveCase("unpriced sorts last, provider/id tiebreak", [m("z", "nocost"), m("b", "same", { input: 1, output: 1 }), m("a", "same", { input: 1, output: 1 })], "low", { maxTier: "high", tierFallback: "none" }, cfg({ "b/same": "low", "a/same": "low", "z/nocost": "low" }));
  resolveCase("preferences before cost", [m("p", "cheap", { input: 0, output: 0.1 }), m("q", "wild", { input: 0, output: 0.2 }), m("q", "exact", { input: 100, output: 100 })], "low", { maxTier: "high", tierFallback: "none" }, cfg({ "p/cheap": "low", "q/wild": "low", "q/exact": "low" }, { low: ["q/exact", "p/*"] }));
  resolveCase("fallback none → candidatesPerTier", [m("p", "highm"), m("p", "medm")], "low", { maxTier: "high", tierFallback: "none" }, cfg({ "p/highm": "high", "p/medm": "medium" }));
  resolveCase("fallback higher climbs", [m("p", "medm"), m("p", "highm")], "low", { maxTier: "high", tierFallback: "higher" }, cfg({ "p/medm": "medium", "p/highm": "high" }));
  resolveCase("fallback stops at maxTier", [m("p", "highm")], "low", { maxTier: "medium", tierFallback: "higher" }, cfg({ "p/highm": "high" }));
  resolveCase("resolved metadata", [m("p", "x", { input: 0.1, output: 0.5 })], "medium", { maxTier: "high", tierFallback: "none" }, NO_PREF);
  resolveCase("workload changes pick", [m("p", "input-heavy", { input: 10, output: 1 }), m("p", "output-heavy", { input: 1, output: 10 })], "low", { maxTier: "high", tierFallback: "none" }, NO_PREF, 0);
  resolveCase("workload writer pick", [m("p", "input-heavy", { input: 10, output: 1 }), m("p", "output-heavy", { input: 1, output: 10 })], "low", { maxTier: "high", tierFallback: "none" }, NO_PREF, 10);
  resolveCase("exclude exact", [m("p", "excluded", { input: 0.01, output: 0.01 }), m("p", "kept", { input: 5, output: 1 })], "low", { maxTier: "high", tierFallback: "none" }, cfg({ "p/excluded": "low", "p/kept": "low" }, {}, ["p/excluded"]));
  resolveCase("exclude wildcard", [m("p", "x"), m("q", "y")], "low", { maxTier: "high", tierFallback: "none" }, cfg({ "p/x": "low", "q/y": "low" }, {}, ["p/*"]));
  resolveCase("all excluded", [m("p", "x")], "low", { maxTier: "high", tierFallback: "none" }, cfg({}, {}, ["p/*"]));
  resolveCase("never falls back lower", [m("p", "lowm")], "medium", { maxTier: "high", tierFallback: "higher" }, cfg({ "p/lowm": "low" }));
  // localeCompare: base letters first — "anthropic" < "OpenAI" (byte
  // order would reverse it).
  resolveCase("provider tiebreak uses localeCompare", [m("OpenAI", "m", { input: 1, output: 1 }), m("anthropic", "m", { input: 1, output: 1 })], "low", { maxTier: "high", tierFallback: "none" }, cfg({ "OpenAI/m": "low", "anthropic/m": "low" }));
  // localeCompare primary weight puts punctuation before digits, and
  // `_` before `-`: "a_1" < "a-2" < "a0" — the UTF-16 code order is
  // different (`-`(0x2D) < `0` < `_`), so the tiebreak result differs.
  resolveCase("id tiebreak punct before digit", [m("p", "a0", { input: 1, output: 1 }), m("p", "a_1", { input: 1, output: 1 }), m("p", "a-2", { input: 1, output: 1 })], "low", { maxTier: "high", tierFallback: "none" }, cfg({ "p/a0": "low", "p/a_1": "low", "p/a-2": "low" }));
  resolveCase("provider tiebreak underscore before digit", [m("z0", "m", { input: 1, output: 1 }), m("z_1", "m", { input: 1, output: 1 })], "low", { maxTier: "high", tierFallback: "none" }, cfg({ "z0/m": "low", "z_1/m": "low" }));
  // Digit < letter at the provider level (same order in UTF-16, kept
  // for coverage).
  resolveCase("provider tiebreak digit before letter", [m("z0", "m", { input: 1, output: 1 }), m("za", "m", { input: 1, output: 1 })], "low", { maxTier: "high", tierFallback: "none" }, cfg({ "z0/m": "low", "za/m": "low" }));

  explicitCase("explicit picks model", [m("p", "x", { input: 1, output: 30 }), m("q", "cheap", { input: 0.1, output: 0.1 }), m("q", "mid", { input: 1, output: 1 })], { provider: "p", id: "x" }, "high", NO_PREF);
  explicitCase("explicit excluded → violation", [m("openai", "gpt-5")], { provider: "openai", id: "gpt-5" }, "high", cfg({}, {}, ["openai/*"]));
  explicitCase("explicit unavailable", [m("p", "x")], { provider: "foo", id: "bar" }, "high", NO_PREF);
  explicitCase("explicit over maxTier", [m("p", "x", { input: 1, output: 30 }), m("q", "cheap", { input: 0.1, output: 0.1 }), m("q", "mid", { input: 1, output: 1 })], { provider: "p", id: "x" }, "medium", NO_PREF);
  explicitCase("override tier over maxTier", [m("p", "x", { input: 0.1, output: 0.5 })], { provider: "p", id: "x" }, "medium", cfg({ "p/x": "high" }));
  writeFixture("model/resolve.json", { platform: PLATFORM, cases });

  // model/config.json — loadModelConfig
  const cfgCases = [];
  let idx = 0;
  const cfgPush = async (name, files) => {
    const c = await caseDir(`mc${idx++}`, files);
    const expected = await attempt(async () => {
      const config = await core.loadModelConfig({
        userConfigDir: path.join(c.dir, "user"),
        projectDir: path.join(c.dir, "proj", ".cheaper"),
      });
      return {
        overrides: Object.fromEntries(
          Object.entries(config.overrides ?? {}).sort(([a], [b]) => a.localeCompare(b)),
        ),
        preferences: config.preferences,
        ...(config.exclude !== undefined ? { exclude: config.exclude } : {}),
      };
    });
    cfgCases.push({
      name,
      files,
      opts: { userConfigDir: "user", projectDir: "proj/.cheaper" },
      expected: scrub(expected, c.needles),
    });
  };
  await cfgPush("no files → empty", { user: {}, "proj/.cheaper": {} });
  await cfgPush("user models.yaml", {
    "user/models.yaml":
      "version: 1\noverrides:\n  openai/gpt-5-mini: low\npreferences:\n  low: [anthropic/claude-haiku-4-5, \"openai/*\"]\n",
  });
  await cfgPush("merge user then project", {
    "user/models.yaml":
      "version: 1\noverrides:\n  openai/gpt-5-mini: low\n  anthropic/claude-sonnet-4: medium\npreferences:\n  low: [\"openai/*\"]\n  medium: [a/x]\n",
    "proj/.cheaper/models.yaml":
      "version: 1\noverrides:\n  anthropic/claude-sonnet-4: high\npreferences:\n  low: [anthropic/claude-haiku-4-5]\n",
  });
  await cfgPush("exclude union", {
    "user/models.yaml": 'version: 1\nexclude: [anthropic/claude-opus-4-5, "dup/x"]\n',
    "proj/.cheaper/models.yaml": 'version: 1\nexclude: [openai/gpt-5-pro, "dup/x"]\n',
  });
  await cfgPush("missing version", { "user/models.yaml": "overrides: {}\n" });
  await cfgPush("unknown top-level", { "proj/.cheaper/models.yaml": "version: 1\nbogus: 1\n" });
  await cfgPush("invalid tier", {
    "user/models.yaml": "version: 1\noverrides:\n  p/m: ultra\n",
  });
  await cfgPush("unknown preferences tier", {
    "user/models.yaml": 'version: 1\npreferences:\n  extreme: ["p/*"]\n',
  });
  await cfgPush("unreadable models.yaml (dir)", {
    "user/models.yaml/marker.txt": "x",
  });
  await cfgPush("non-object yaml", { "user/models.yaml": "- a\n- b\n" });
  await cfgPush("yaml parse error", { "user/models.yaml": "version: [unclosed\n" });
  writeFixture("model/config.json", { platform: PLATFORM, cases: cfgCases });
}

// ===================================================================
// token/count.json
// ===================================================================

{
  const cases = [];
  let idx = 0;
  const pushText = (name, text) =>
    cases.push({
      name,
      op: "countText",
      text,
      expected: (() => {
        try {
          return { ok: { tokens: core.countTextTokens(text) } };
        } catch (e) {
          return { error: cap(e) };
        }
      })(),
    });
  pushText("empty", "");
  pushText("ascii", "hello world\n");
  pushText("unicode", "日本語のテキスト。🚀 emoji と éàü 混在。\n");
  pushText("long ascii", "x".repeat(1024));
  pushText("special token rejected", "a <|endoftext|> b");
  pushText("endofprompt rejected", "<|endofprompt|>");

  const pushFile = async (name, { files, input, policy = "deny", maxBytes }) => {
    const c = await caseDir(`t${idx++}`, files, ".");
    const expected = await attempt(async () => {
      const r = await core.countFileTokens({
        root: c.root,
        path: input,
        symlinkPolicy: policy,
        ...(maxBytes !== undefined ? { maxBytes } : {}),
      });
      // tokenizer identity excluded from comparison by design.
      return { path: r.path, tokens: r.tokens, bytes: r.bytes };
    });
    cases.push({
      name,
      op: "countFile",
      files,
      input,
      policy,
      maxBytes: maxBytes ?? null,
      expected: scrub(expected, c.needles),
    });
  };
  await pushFile("small file", { files: { "a.txt": "hello world\n" }, input: "a.txt", policy: "within-workspace" });
  await pushFile("empty file", { files: { "empty.txt": "" }, input: "empty.txt" });
  await pushFile("unicode file", { files: { "uni.txt": "日本語のテキスト。🚀 emoji と éàü 混在。\n" }, input: "uni.txt" });
  await pushFile("binary lossy", { files: { "bin.dat": { base64: "iVBORwAA//4B" } }, input: "bin.dat" });
  await pushFile("special token file rejected", { files: { "s.txt": "x <|endoftext|> y" }, input: "s.txt" });
  await pushFile("escape rejected", { files: { "in.txt": "x" }, input: "../outside.txt" });
  await pushFile("absolute outside rejected", { files: { "in.txt": "x" }, input: "/etc/passwd" });
  await pushFile("missing file", { files: {}, input: "missing.txt" });
  await pushFile("over maxBytes", { files: { "big.bin": "x".repeat(64) }, input: "big.bin", maxBytes: 16 });
  await pushFile("exactly maxBytes", { files: { "exact.txt": "x".repeat(16) }, input: "exact.txt", maxBytes: 16 });
  writeFixture("token/count.json", { platform: PLATFORM, cases });
}

// ===================================================================
// workspace/paths.json
// ===================================================================

{
  const cases = [];
  let idx = 0;
  const push = async (name, { files = {}, symlinks, input, policy = "deny", op = "resolve", cwdRel }) => {
    const c = await caseDir(`p${idx++}`, files, ".", symlinks);
    if (c === undefined) {
      // symlinks unavailable on this platform → skip the case
      return;
    }
    const realInput = unScrub(input, c.needles);
    const expected = await attempt(async () => {
      if (op === "resolve")
        return { path: await core.resolveWorkspacePath(c.root, realInput, policy) };
      if (op === "canonicalize") {
        const cwd = path.join(c.dir, cwdRel);
        return { root: await core.canonicalizeWorkspaceRoot(cwd) };
      }
      throw new Error(op);
    });
    cases.push({
      name,
      op,
      files,
      symlinks: symlinks ?? null,
      input: input ?? null,
      cwdRel: cwdRel ?? null,
      policy,
      expected: scrub(expected, c.needles),
    });
  };
  await push("NUL rejected", { input: "a\0b" });
  await push("escape rejected", { input: "../outside.txt" });
  await push("interior escape rejected", { input: "sub/../../outside.txt", policy: "within-workspace" });
  await push("dot normalization", {
    files: { "sub/file.txt": "x" },
    input: "sub/../sub/./file.txt",
  });
  await push("dot root", { input: "." });
  await push("tilde literal", { input: "~/x" });
  await push("nonexistent tail", {
    files: { "sub/deep/.keep": "" },
    input: "sub/deep/new-file.txt",
  });
  await push("fully nonexistent subtree", { input: "a/b/c.txt" });
  await push("absolute inside accepted", {
    files: { "sub/file.txt": "x" },
    input: "<ROOT>/sub/file.txt",
  });
  await push("absolute outside rejected", { input: "<OUT>/definitely-outside.txt" });
  await push("canonicalize missing dir", {
    op: "canonicalize",
    files: {},
    cwdRel: "nope",
  });
  await push("deny rejects in-workspace symlink", {
    files: { "real/in.txt": "in" },
    symlinks: [{ target: "real", link: "link-in" }],
    input: "link-in/in.txt",
    policy: "deny",
  });
  await push("within rejects symlink to outside dir", {
    files: { "real/in.txt": "in" },
    symlinks: [{ target: "<OUT>", link: "link-out" }],
    input: "link-out/out.txt",
    policy: "within-workspace",
  });
  await push("within allows symlink to inside dir", {
    files: { "real/in.txt": "in" },
    symlinks: [{ target: "real", link: "link-in" }],
    input: "link-in/in.txt",
    policy: "within-workspace",
  });
  await push("within allows nonexistent tail under inside symlink", {
    files: { "real/in.txt": "in" },
    symlinks: [{ target: "real", link: "link-in" }],
    input: "link-in/new-file.txt",
    policy: "within-workspace",
  });

  // isInsideWorkspaceRoot — pure string comparison, both axes.
  const insideCases = [];
  for (const [name, root, cand, ci, expected] of [
    ["root itself", "tmp/work", "tmp/work", false, true],
    ["descendant", "tmp/work", "tmp/work/a", false, true],
    ["prefix-confusable sibling", "tmp/work", "tmp/workx/a", false, false],
    ["other sibling", "tmp/work", "tmp/other", false, false],
    ["case differs strict", "tmp/work", "tmp/Work/secret", false, false],
    ["case differs insensitive", "tmp/work", "tmp/Work/secret", true, true],
  ])
    insideCases.push({
      name,
      root,
      candidate: cand,
      caseInsensitive: ci,
      expected: {
        ok: core.isInsideWorkspaceRoot(
          path.join(...root.split("/")),
          path.join(...cand.split("/")),
          { caseInsensitive: ci },
        ),
      },
    });
  cases.push(...insideCases.map((c) => ({ ...c, op: "inside" })));
  writeFixture("workspace/paths.json", { platform: PLATFORM, cases });
}

// ===================================================================
// workspace/deny.json — deny patterns (both platform axes)
// ===================================================================

{
  const cases = [];
  for (const axis of ["win32", "linux"]) {
    const run = (fn) => withPlatform(axis, fn);
    for (const [pattern, display] of [
      [".env", ".env"],
      [".env", "config/.env"],
      [".env", ".env.local"],
      [".env", "envfile"],
      [".env.*", ".env.local"],
      [".env.*", "a/b/.env.production"],
      [".env.*", ".env"],
      ["*.pem", "certs/server.pem"],
      ["*.pem", "certs/pem.txt"],
      [".ssh/**", ".ssh"],
      [".ssh/**", ".ssh/id_rsa"],
      [".ssh/**", "home/user/.ssh/config"],
      [".ssh/**", ".sshd_config"],
      [".ssh/**", "ssh/file"],
      ["/config/secrets.json", "config/secrets.json"],
      ["/config/secrets.json", "sub/config/secrets.json"],
      ["a/**/b.txt", "a/x/y/b.txt"],
      ["a/**/b.txt", "a/b.txt"],
      ["secret?.txt", "secret1.txt"],
      ["secret?.txt", "secret12.txt"],
      [".env", ".ENV"],
      ["*.pem", "certs/KEY.PEM"],
    ])
      cases.push({
        name: `pattern ${pattern} vs ${display} @${axis}`,
        axis: axis === "win32" ? "win32" : "posix",
        op: "denyPattern",
        pattern,
        display,
        caseInsensitive: axis === "win32",
        expected: { ok: run(() => core.denyPatternToRegex(pattern).test(display)) },
      });
    const list = run(() => core.createPathDenylist(["Blocked", "/Private/**"]));
    for (const display of [
      ".ENV",
      "certs/KEY.PEM",
      "blocked",
      "PRIVATE/data.json",
      "Blocked",
      "Private/data.json",
      "sub/Private/data.json",
      "src/index.ts",
      "environment.ts",
      "./.env",
    ])
      cases.push({
        name: `denylist ${display} @${axis}`,
        axis: axis === "win32" ? "win32" : "posix",
        op: "denylist",
        extra: ["Blocked", "/Private/**"],
        display,
        caseInsensitive: axis === "win32",
        expected: { ok: list.isDenied(display) },
      });
  }
  const defaults = core.createPathDenylist();
  cases.push({
    name: "DEFAULT_DENY_PATHS count + patterns",
    op: "defaults",
    expected: { ok: { patterns: [...core.DEFAULT_DENY_PATHS] } },
  });
  for (const display of [
    ".env",
    "config/.env.local",
    "certs/server.pem",
    "keys/private.key",
    "id_rsa",
    "a/b/id_ed25519",
    ".netrc",
    "pkg/.npmrc",
    "secrets.yaml",
    ".ssh/config",
    "app/.aws/credentials",
    "src/index.ts",
    "docs/guide.md",
    "environment.ts",
    "README.md",
    "src/secrets-manager.ts",
    "keynote.md",
  ])
    cases.push({
      name: `default denylist ${display}`,
      op: "denylist",
      extra: [],
      display,
      axis: "any",
      caseInsensitive: PLATFORM === "win32",
      expected: { ok: defaults.isDenied(display) },
    });
  writeFixture("workspace/deny.json", { platform: PLATFORM, cases });
}

// ===================================================================
// resources.json — normalization + predicates
// ===================================================================

{
  const cases = [];
  let idx = 0;

  const push = async (name, { files = {}, symlinks, input, policy = "deny", axis = "any" }) => {
    const c = await caseDir(`rs${idx++}`, files, ".", symlinks);
    if (c === undefined) return;
    const platform = axis === "posix" ? "linux" : axis === "win32" ? "win32" : process.platform;
    const realInput = unScrub(input, c.needles);
    const expected = await withPlatform(platform, () =>
      attempt(async () => ({
        manifest: dumpManifest(
          await core.normalizeResourceManifest(
            c.root,
            realInput === undefined ? undefined : realInput,
            policy,
          ),
        ),
      })),
    );
    cases.push({
      name,
      op: "normalize",
      files,
      symlinks: symlinks ?? null,
      input: input === undefined ? null : input,
      // `undefined` (omitted) and explicit `null` are different inputs but
      // both serialize to JSON `null`; the flag preserves the distinction.
      omitted: input === undefined,
      policy,
      axis,
      expected: scrub(expected, c.needles),
    });
  };

  await push("omitted manifest → unknown access", { input: undefined });
  await push("empty manifest → unknown access", { input: {} });
  await push("explicit empty buckets", { input: { reads: [], writes: [], produces: [] } });
  await push("null bucket = unknown", { input: { reads: null, writes: [], produces: [] } });
  await push("omitted bucket = unknown", { input: { writes: [] } });
  await push("declaration exact", { input: { reads: [{ path: "src/./a/../a/future.ts", scope: "exact" }], writes: [] } });
  await push("declaration subtree", { input: { reads: [{ path: "src/a", scope: "subtree" }], writes: [] } });
  await push("writes+produces modes", {
    input: {
      reads: [{ path: "future", scope: "exact" }],
      writes: [{ path: "future", scope: "exact" }],
      produces: [{ path: "future", scope: "exact" }],
    },
  });
  for (const [label, input] of [
    ["null manifest", null],
    ["array manifest", []],
    ["produces null", { produces: null }],
    ["reads string", { reads: "unknown" }],
    ["writes object", { writes: {} }],
    ["unknown bucket", { unknown: [] }],
    ["declaration with mode", { reads: [{ path: "x", scope: "exact", mode: "shared" }] }],
    ["declaration extra key", { reads: [{ path: "x", scope: "exact", extra: true }] }],
    ["mixed reps", { reads: [{ path: "x", scope: "exact" }, { resource: { kind: "workspace", path: "/abs", scope: "exact" }, mode: "shared" }] }],
    ["wrong mode claim", { writes: [{ resource: { kind: "workspace", path: "/abs", scope: "exact" }, mode: "shared" }] }],
    ["reads exclusive claim", { reads: [{ resource: { kind: "workspace", path: "/abs", scope: "exact" }, mode: "exclusive" }] }],
    ["artifact kind", { reads: [{ resource: { kind: "artifact", path: "x", scope: "exact" }, mode: "shared" }] }],
    ["relative claim path", { reads: [{ resource: { kind: "workspace", path: "relative", scope: "exact" }, mode: "shared" }] }],
    ["resource extra key", { reads: [{ resource: { kind: "workspace", path: "/abs", scope: "exact", extra: true }, mode: "shared" }] }],
  ])
    await push(`reject ${label}`, { input });
  for (const [label, decl] of [
    ["empty path", { path: "", scope: "exact" }],
    ["NUL path", { path: "x\0y", scope: "exact" }],
    ["bad scope", { path: "x", scope: "invalid" }],
    ["path number", { path: 123, scope: "exact" }],
    ["missing path", { scope: "exact" }],
    ["missing scope", { path: "x" }],
    ["drive-relative", { path: "C:relative", scope: "exact" }],
    ["absolute", { path: "/absolute", scope: "exact" }],
  ])
    await push(`reject declaration ${label}`, { input: { reads: [decl] } });
  for (const input of ["../escape", "../escape/*.json", "src/../../escape"])
    await push(`escape ${input}`, { input: { reads: [{ path: input, scope: "exact" }] } });
  await push("canonical claim outside → escape", {
    input: { reads: [{ resource: { kind: "workspace", path: "<OUT>/outside", scope: "exact" }, mode: "shared" }] },
  });
  await push("idempotent canonical input", {
    input: {
      reads: [{ resource: { kind: "workspace", path: "<ROOT>/(literal).json", scope: "exact" }, mode: "shared" }],
      writes: [{ resource: { kind: "workspace", path: "<ROOT>/new", scope: "subtree" }, mode: "exclusive" }],
      produces: [{ resource: { kind: "workspace", path: "<ROOT>/(output).json", scope: "exact" }, mode: "exclusive" }],
    },
  });
  await push("dedupe + sort stable", {
    input: {
      reads: [
        { path: "z", scope: "exact" },
        { path: "a/x", scope: "exact" },
        { path: "a", scope: "subtree" },
        { path: "a/./x", scope: "exact" },
        { path: "a", scope: "subtree" },
      ],
      writes: [],
    },
  });
  // `deduplicate` sorts by `resourceKey` via JS `<` — UTF-16 code-unit
  // order (`a-2` < `a0` < `a_1`), NOT localeCompare (`a_1` first).
  await push("dedupe sort UTF-16 code units", {
    input: {
      reads: [
        { path: "a_1", scope: "exact" },
        { path: "a-2", scope: "exact" },
        { path: "a0", scope: "exact" },
        { path: "b", scope: "exact" },
      ],
      writes: [],
    },
  });
  await push("win32 case dedupe", {
    input: { reads: [{ path: "Future/File", scope: "exact" }, { path: "future/file", scope: "exact" }], writes: [] },
    axis: "win32",
  });
  await push("posix case distinct", {
    input: { reads: [{ path: "Case/f", scope: "exact" }, { path: "case/f", scope: "exact" }], writes: [] },
    axis: "posix",
  });
  await push("glob safety deny", {
    input: {
      reads: [{ path: "dir/*.json", scope: "exact" }],
      writes: [{ path: "dir/**/output", scope: "exact" }],
      produces: [{ path: "dir/**", scope: "subtree" }],
    },
  });
  await push("glob safety within-workspace widens", {
    input: {
      reads: [{ path: "dir/*.json", scope: "exact" }],
      writes: [{ path: "dir/**/output", scope: "exact" }],
      produces: [{ path: "dir/**", scope: "subtree" }],
    },
    policy: "within-workspace",
  });
  for (const pattern of ["*", "dir/?", "dir/[abc]", "dir/{a,b}", "dir/+(a|b)", "!dir/*.json"])
    await push(`glob approx ${pattern}`, { input: { reads: [{ path: pattern, scope: "exact" }], writes: [] } });
  await push("win32 backslash glob", {
    input: { reads: [{ path: "dir\\*.json", scope: "exact" }] },
    axis: "win32",
  });
  await push("posix backslash escape cut", {
    input: { reads: [{ path: "d\\ir/*.json", scope: "exact" }] },
    axis: "posix",
  });
  for (const pattern of ["dir/*.json", "dir/**/file", "dir/{a,b}", "!dir/**"])
    await push(`reject producer ${pattern}`, { input: { produces: [{ path: pattern, scope: "subtree" }] } });
  await push("producer dir glob requires subtree", {
    input: { produces: [{ path: "dir/**", scope: "exact" }] },
  });
  await push("producer root glob", { input: { produces: [{ path: "**", scope: "subtree" }] } });
  for (const pattern of ["dir/*/../file", "dir/**/../../other", "!dir/../file"])
    await push(`reject traversal glob ${pattern}`, { input: { reads: [{ path: pattern, scope: "exact" }] } });

  // symlink cases
  await push("within-workspace resolves alias", {
    files: { "actual/.keep": "" },
    symlinks: [{ target: "actual", link: "alias" }],
    input: { writes: [{ path: "alias/future", scope: "exact" }, { path: "actual/future", scope: "exact" }], reads: [] },
    policy: "within-workspace",
  });
  await push("deny rejects alias", {
    files: { "actual/.keep": "" },
    symlinks: [{ target: "actual", link: "alias" }],
    input: { writes: [{ path: "alias/future", scope: "exact" }] },
    policy: "deny",
  });
  await push("within rejects outside alias", {
    files: { "actual/.keep": "" },
    symlinks: [{ target: "<OUT>", link: "outside-alias" }],
    input: { writes: [{ path: "outside-alias/future", scope: "exact" }] },
    policy: "within-workspace",
  });
  await push("glob widens over alias", {
    files: { "dir/.keep": "", "other/target.json": "{}" },
    symlinks: [{ target: "other", link: "dir/link" }],
    input: { writes: [{ path: "dir/**/*.json", scope: "exact" }], reads: [] },
    policy: "within-workspace",
  });
  await push("literal ancestor checked before glob widening", {
    files: {},
    symlinks: [{ target: "<OUT>", link: "outside" }],
    input: { reads: [{ path: "outside/*.json", scope: "exact" }] },
    policy: "within-workspace",
  });

  // pure predicates — synthetic absolute base bound at replay time via
  // the <ABS> token (both sides resolve "golden-pred-base").
  const absBase = path.resolve("golden-pred-base");
  const abs = (p) => path.join(absBase, ...p.split("/"));
  const resource = (p, scope = "exact") => ({ kind: "workspace", path: abs(p), scope });
  const claimAt = (p, scope = "exact", mode = "shared") => ({ resource: resource(p, scope), mode });
  // `resourceKey` lowercases the identity on win32 → scrub the
  // lowercased base variant too (harmless on POSIX).
  const tok = (v) =>
    scrub(v, [
      ["<ABS>", absBase],
      ["<ABS>", absBase.toLowerCase()],
    ]);
  const pCases = [];
  for (const [a, as, b, bs, expected] of [
    ["a", "exact", "a", "exact", true],
    ["a", "exact", "a", "subtree", false],
    ["a", "subtree", "a", "exact", true],
    ["a", "subtree", "a", "subtree", true],
    ["a", "subtree", "a/b", "exact", true],
    ["a", "subtree", "a/b", "subtree", true],
    ["a", "subtree", "ab", "exact", false],
    ["a/b", "subtree", "a", "exact", false],
    ["a", "exact", "a/b", "exact", false],
  ]) {
    pCases.push({
      name: `contains ${a}/${as} ~ ${b}/${bs}`,
      op: "resourceContains",
      a: tok({ path: `<ABS>/${a}`, scope: as }),
      b: tok({ path: `<ABS>/${b}`, scope: bs }),
      expected: { ok: core.resourceContains(resource(a, as), resource(b, bs)) },
    });
  }
  for (const [a, b, contains, conflict] of [
    ["shared", "shared", true, false],
    ["shared", "exclusive", false, true],
    ["exclusive", "shared", true, true],
    ["exclusive", "exclusive", true, true],
  ]) {
    const ca = claimAt("a", "subtree", a);
    const cb = claimAt("a/b", "exact", b);
    pCases.push({
      name: `mode ${a}→${b}`,
      op: "claimPredicates",
      a: tok(ca),
      b: tok(cb),
      expected: {
        ok: { contains: core.claimContains(ca, cb), conflict: core.claimsConflict(ca, cb) },
      },
    });
  }
  pCases.push({
    name: "overlap + conflict symmetric",
    op: "predicatesBundle",
    expected: {
      ok: {
        overlap: core.resourcesOverlap(resource("a", "subtree"), resource("a/b", "exact")),
        overlapSymmetric: core.resourcesOverlap(resource("a/b", "exact"), resource("a", "subtree")),
        conflict: core.claimsConflict(claimAt("a", "subtree", "exclusive"), claimAt("a/b", "exact")),
        conflictSymmetric: core.claimsConflict(claimAt("a/b", "exact"), claimAt("a", "subtree", "exclusive")),
        siblingConflict: core.claimsConflict(claimAt("a", "subtree", "exclusive"), claimAt("ab", "subtree", "exclusive")),
      },
    },
  });
  pCases.push({
    name: "resourceKey differs for different paths",
    op: "resourceKeys",
    a: tok({ path: "<ABS>/a", scope: "subtree" }),
    b: tok({ path: "<ABS>/b", scope: "subtree" }),
    expected: {
      ok: {
        keyA: tok(core.resourceKey(resource("a", "subtree"))),
        keyB: tok(core.resourceKey(resource("b", "subtree"))),
      },
    },
  });
  {
    const read = { reads: [claimAt("a/b")], writes: [], produces: [] };
    const write = { reads: [], writes: [claimAt("a", "subtree", "exclusive")], produces: [] };
    const produce = { reads: [], writes: [], produces: [claimAt("a", "subtree", "exclusive")] };
    const disjoint = { reads: [], writes: [claimAt("ab", "subtree", "exclusive")], produces: [] };
    const empty = { reads: [], writes: [], produces: [] };
    pCases.push({
      name: "manifest buckets conflict",
      op: "manifestsConflict",
      a: tok(read),
      b: tok(write),
      c: tok(produce),
      d: tok(disjoint),
      e: tok(empty),
      expected: {
        ok: {
          readVsRead: core.manifestsConflict(read, read),
          readVsWriteParent: core.manifestsConflict(read, write),
          readVsProduceParent: core.manifestsConflict(read, produce),
          disjoint: core.manifestsConflict(read, disjoint),
          emptyVsRead: core.manifestsConflict(empty, read),
        },
      },
    });
  }
  pCases.push({
    name: "filesystem root contains descendants",
    op: "resourceContains",
    a: { path: "<FSROOT>", scope: "subtree" },
    b: tok({ path: "<ABS>/a", scope: "exact" }),
    expected: {
      ok: core.resourceContains(
        { kind: "workspace", path: path.parse(absBase).root, scope: "subtree" },
        resource("a"),
      ),
    },
  });
  cases.push(...pCases);
  writeFixture("resources.json", { platform: PLATFORM, cases });
}

// ===================================================================
// number-format.json — JS number formatting
// ===================================================================

{
  // The expected strings are the JS `String(number)` outputs computed
  // in-process; the Rust `format_number` port must match verbatim.
  const cases = [];
  for (const input of [
    0, -0, 1, -1, 1.5, -1.5, 0.1 + 0.2, 1e21, 1e-6, 1e-7, 123456789, 9007199254740991,
    2 ** 53, 0.0001, 1234.5, 1e20, 1e22, -1e21, 1.23e-8, 5e-324,
  ]) {
    cases.push({ input, expected: { ok: String(input) } });
  }
  for (const input of [Number.POSITIVE_INFINITY, Number.NEGATIVE_INFINITY, Number.NaN])
    cases.push({ input: String(input), expected: { ok: "null" } });
  writeFixture("number-format.json", { platform: PLATFORM, cases });
}

// ---------- summary ----------

await rm(TMP_BASE, { recursive: true, force: true });
console.log(`wrote ${written.length} fixture files:`);
for (const w of written) console.log(`  ${w}`);
