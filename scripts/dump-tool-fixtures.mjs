// Dumps TypeScript `packages/tool-runtime` (+ runtime-pi injected handlers)
// behavior into deterministic JSON differential fixtures consumed by the
// Rust `cheaper-tools` replay harness
// (crates/cheaper-tools/tests/differential.rs). Run after
// `pnpm --filter @cheaper/core --filter @cheaper/tool-runtime
//  --filter @cheaper/runtime-pi build`; re-running must produce no diff.
//
// Format follows scripts/dump-core-golden.mjs: each fixture file is
// { "platform", "cases": [...] }. Cases carry `files`/`symlinks` setup
// written under a per-case tmp dir, a `runtime` spec (ToolRuntimeOptions)
// or a `tool` spec (tool-level execute with injected caps), an `ops`
// sequence (executions plus abort/drain/touch effects), and `expected` —
// one entry per `ops` element (`{ok: result}` or `{error: cap}`; a
// `parallel` op yields `{ok: {calls: [...], then: [...]}}`). Absolute
// paths are replaced by `<ROOT>`/`<OUT>`/`<BASE>`/`<NODE>` tokens;
// `axis` marks platform-sensitive cases. `expectRust` (sparse, keyed by
// op index) pins documented intentional divergences.
//
// Only timestamps, PIDs and temp-name randomness are normalized away —
// everything else is recorded verbatim.

import { chmodSync, mkdirSync, readFileSync, realpathSync, symlinkSync, writeFileSync } from "node:fs";
import { mkdtemp, mkdir, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = fileURLToPath(new URL("..", import.meta.url));
const core = await import(new URL("../packages/core/dist/index.js", import.meta.url).href);
const rt = await import(new URL("../packages/tool-runtime/dist/index.js", import.meta.url).href);
const pi = await import(new URL("../packages/runtime-pi/dist/index.js", import.meta.url).href);
// `exports["."]` は `import` condition のみのため、runtime-pi の依存解決先
// (pnpm symlink)を経由して dist entry を直接 import する。
const piAgent = await import(
  new URL(
    "../packages/runtime-pi/node_modules/@earendil-works/pi-coding-agent/dist/index.js",
    import.meta.url,
  ).href
);

const OUT_DIR = path.join(repoRoot, "crates", "cheaper-tools", "tests", "fixtures");
const PLATFORM = process.platform;
const NODE = process.execPath;

// Pi session layer applies `prepareArguments` before schema validation;
// the canonical registry does not (the Rust port folds it into
// `execute`). The dump replays the same position: prepare → execute.
const editPrepare = piAgent.createEditToolDefinition(".", {}).prepareArguments;

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

/** Same normalization contract as dump-core-golden.mjs. */
function normalizeDetails(v) {
  if (Array.isArray(v)) return v.map(normalizeDetails);
  if (v !== null && typeof v === "object") {
    const o = {};
    for (const [k, val] of Object.entries(v)) {
      if (k === "errno") continue;
      if (k === "errors" && Array.isArray(val)) {
        // ajv は union branch ごとに重複 error を出すため、engine 差の
        // ない形(sorted unique instancePath)に正規化する。
        const paths = [
          ...new Set(val.map((e2) => e2?.instancePath ?? "")),
        ].sort();
        o[k] = paths.map((p) => ({ instancePath: p }));
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
  // AbortError DOMException / plain Error / abort reasons land here —
  // `name`+`message` are the parity contract (AbortReason::msg → "Error").
  return { kind: "other", name: e?.name ?? typeof e, message: String(e?.message ?? e) };
}

async function attempt(fn) {
  try {
    return { ok: await fn() };
  } catch (e) {
    return { error: cap(e) };
  }
}

/** CanonicalToolResult → plain JSON (drop undefined `details`). */
function canonicalResult(r) {
  const out = { content: r.content };
  if (r.details !== undefined) out.details = r.details;
  return out;
}

// ---------- tmp dirs, files, scrub ----------

const TMP_BASE = await mkdtemp(path.join(os.tmpdir(), "cheaper-tools-golden-"));

async function writeFiles(dir, files) {
  for (const [rel, content] of Object.entries(files ?? {})) {
    const p = path.join(dir, ...rel.split("/"));
    await mkdir(path.dirname(p), { recursive: true });
    if (content !== null && typeof content === "object") {
      if ("base64" in content) {
        writeFileSync(p, Buffer.from(content.base64, "base64"));
        continue;
      }
      if ("repeat" in content) {
        const [unit, times] = content.repeat;
        writeFileSync(p, unit.repeat(times), "utf8");
        continue;
      }
      if ("mode" in content) {
        // `{content, mode}`: 書き込み後に chmod する(read-only 権限の
        // access(2) fixture 用。mode は 8 進数 permission bits)。
        writeFileSync(p, String(content.content ?? ""), "utf8");
        chmodSync(p, content.mode);
        continue;
      }
    }
    writeFileSync(p, String(content), "utf8");
  }
}

/** `repeatFiles`: `{dir, prefix, count, content}` → `dir/prefixNNNN.txt`. */
async function writeRepeatFiles(dir, spec) {
  for (const [key, r] of Object.entries(spec ?? {})) {
    void key;
    for (let i = 0; i < r.count; i++) {
      const p = path.join(dir, ...r.dir.split("/"), `${r.prefix}${String(i).padStart(4, "0")}.txt`);
      await mkdir(path.dirname(p), { recursive: true });
      writeFileSync(p, r.content ?? "", "utf8");
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

function scrub(v, needles) {
  const list = [...needles].sort((a, b) => b[1].length - a[1].length);
  const sub = (s) => {
    for (const [token, p] of list) {
      s = s
        .split(p)
        .join(token)
        .split(p.replaceAll("\\", "/"))
        .join(token)
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
async function caseDir(index, spec) {
  const dir = path.join(TMP_BASE, `c${index}`);
  await mkdir(dir, { recursive: true });
  await writeFiles(dir, spec.files);
  await writeRepeatFiles(dir, spec.repeatFiles);
  const outDir = path.join(TMP_BASE, "outside");
  await mkdir(outDir, { recursive: true });
  const root = realpathSync(dir);
  try {
    makeSymlinks(dir, outDir, spec.symlinks);
  } catch {
    return undefined; // symlinks unavailable → skip the case entirely
  }
  return {
    dir: root,
    root,
    needles: [
      ["<ROOT>", root],
      ["<OUT>", realpathSync(outDir)],
      ["<BASE>", realpathSync(TMP_BASE)],
      ["<NODE>", NODE],
    ],
  };
}

// ---------- deterministic artifact ports ----------

/**
 * Fixture reader/publisher. Both sides implement the same slicing and the
 * same reference shape so artifact paths are comparable without a real
 * store. `artifactBlob` in the case spec is the reader's source bytes.
 */
function fixtureArtifacts(blob) {
  const data = typeof blob === "string" ? Buffer.from(blob, "utf8") : blob;
  return {
    reader: {
      read(_source, req) {
        const start = Math.min(req.offset, data.length);
        const end = Math.min(start + req.maxBytes, data.length);
        return {
          data: data.subarray(start, end),
          offset: start,
          nextOffset: end,
          eof: end >= data.length,
        };
      },
    },
    publisher: {
      maxArtifactBytes: 128,
      put({ producer, data: putData }) {
        const bytes =
          typeof putData === "string" ? Buffer.from(putData, "utf8") : putData;
        return {
          artifact: `fixture/${bytes.length}`,
          runId: "fixture-run",
          producer,
          bytes: bytes.length,
          preview: bytes.subarray(0, 16).toString("utf8"),
          truncated: bytes.length > 16,
        };
      },
    },
  };
}

// ---------- runtime construction ----------

function runtimeOptions(spec, ctx) {
  const scopeController = new AbortController();
  const journal = new rt.MutationJournal();
  const opts = {
    root: ctx.root,
    symlinkPolicy: spec.symlinkPolicy ?? "deny",
    names: spec.names ?? ["read"],
    maxToolCalls: spec.maxToolCalls ?? 100,
    onLimit: () => {},
    journal,
  };
  if (spec.writeScope !== undefined) opts.writeScope = new Set(spec.writeScope);
  if (spec.resourceManifest !== undefined) opts.resourceManifest = spec.resourceManifest;
  if (spec.denylist !== undefined) opts.denylist = core.createPathDenylist(spec.denylist);
  if (spec.concurrency !== undefined) opts.concurrency = spec.concurrency;
  if (spec.process !== undefined) opts.process = unScrub(spec.process, ctx.needles);
  if (spec.abortGraceMs !== undefined) opts.abortGraceMs = spec.abortGraceMs;
  if (spec.readStrictness !== undefined) opts.readStrictness = spec.readStrictness;
  if (spec.artifacts) {
    const blob =
      spec.artifactBlob?.base64 !== undefined
        ? Buffer.from(spec.artifactBlob.base64, "base64")
        : Buffer.from(spec.artifactBlob ?? "", "utf8");
    opts.artifacts = fixtureArtifacts(blob);
  }
  if (spec.signal) opts.signal = scopeController.signal;
  return { opts, scopeController, journal };
}

/** `{...input}` prepared for `runtime.execute` (edit prepareArguments). */
function prepareInput(tool, input) {
  const cloned = input === undefined ? {} : structuredClone(input);
  if (tool === "edit" && editPrepare) return editPrepare(cloned);
  return cloned;
}

// ---------- operation runners ----------

async function runCall(runtime, spec, needles) {
  const ctl = new AbortController();
  if (spec.preAborted !== undefined) ctl.abort(new Error(spec.preAborted));
  const input = unScrub(prepareInput(spec.tool, spec.input), needles);
  try {
    const result = await runtime.execute(spec.tool, input, { signal: ctl.signal });
    return { ok: canonicalResult(result) };
  } catch (e) {
    return { error: cap(e) };
  }
}

async function runParallel(runtime, spec, needles, ctx) {
  const members = spec.parallel;
  const ctls = members.map(() => new AbortController());
  const pending = members.map((m, i) => {
    if (m.preAborted !== undefined) ctls[i].abort(new Error(m.preAborted));
    const input = unScrub(prepareInput(m.tool, m.input), needles);
    return runtime
      .execute(m.tool, input, { signal: ctls[i].signal })
      .then((r) => ({ ok: canonicalResult(r) }))
      .catch((e) => ({ error: cap(e) }));
  });
  // 送信順を保った上で member-level abort を適用する。
  members.forEach((m, i) => {
    if (m.abortReason !== undefined) ctls[i].abort(new Error(m.abortReason));
  });
  const then = [];
  for (const step of spec.then ?? []) {
    then.push(await runEffect(runtime, step, ctx));
  }
  const calls = await Promise.all(pending);
  return { ok: { calls, then } };
}

async function runEffect(runtime, op, ctx) {
  switch (op.op) {
    case "ready":
      return await attempt(async () => {
        await runtime.ready();
        return null;
      });
    case "abort-runtime":
      runtime.abort(op.reason === undefined ? undefined : new Error(op.reason));
      return { ok: null };
    case "abort-scope":
      ctx.scopeController.abort(new Error(op.reason));
      return { ok: null };
    case "drain":
      return await attempt(async () => {
        await runtime.drain();
        return null;
      });
    case "touch": {
      const p = path.join(ctx.dir, ...op.path.split("/"));
      await mkdir(path.dirname(p), { recursive: true });
      writeFileSync(p, op.content ?? "go", "utf8");
      return { ok: null };
    }
    default:
      throw new Error(`unknown op ${JSON.stringify(op)}`);
  }
}

// ---------- post-state projection ----------

function projectOperations(runtime) {
  return runtime.operations().map((op) => {
    const out = {
      toolName: op.toolName ?? null,
      state: op.state,
      settlement: op.settlement,
    };
    if (op.resourceClaims !== undefined) out.resourceClaims = op.resourceClaims;
    if (op.cancellation) {
      out.cancellation = {
        state: op.cancellation.state,
        reason: op.cancellation.reason,
      };
    }
    if (op.error) out.error = op.error;
    return out;
  });
}

function projectAccesses(runtime) {
  const snap = runtime.accesses();
  return {
    total: snap.total,
    dropped: snap.dropped,
    observations: snap.observations.map((o) => {
      const out = {
        path: o.path,
        claim: o.claim,
        declared: o.declared,
        allowed: o.allowed,
        support: o.support,
      };
      if (o.operationId !== undefined) out.operationId = "<OP>";
      return out;
    }),
  };
}

async function projectState(runtime, spec) {
  const state = {
    budget: {
      toolCalls: runtime.budget.toolCalls,
      rejectedToolCalls: runtime.budget.rejectedToolCalls,
      limitHit: runtime.budget.limitHit,
    },
    meter: { sourceTokens: runtime.meter.sourceTokens },
    journal: await runtime.journal.finalize(),
    operations: projectOperations(runtime),
  };
  if (spec.accesses) state.accesses = projectAccesses(runtime);
  return state;
}

// ---------- tool-level runners ----------

async function runToolLevel(caseSpec, ctx) {
  const { name, opts = {} } = caseSpec.tool;
  let tool;
  const common = {
    root: ctx.root,
    symlinkPolicy: caseSpec.symlinkPolicy ?? "deny",
    denylist:
      caseSpec.denylist === undefined
        ? undefined
        : core.createPathDenylist(caseSpec.denylist),
  };
  if (name === "grep") {
    tool = rt.createGrepTool({
      ...common,
      ...(opts.evalPerFileMs !== undefined ? { evalPerFileMs: opts.evalPerFileMs } : {}),
      ...(opts.evalTotalMs !== undefined ? { evalTotalMs: opts.evalTotalMs } : {}),
      ...(opts.scanDeadlineMs !== undefined ? { scanDeadlineMs: opts.scanDeadlineMs } : {}),
      ...(opts.chunkBytes !== undefined ? { chunkBytes: opts.chunkBytes } : {}),
      ...(opts.maxLineBytes !== undefined ? { maxLineBytes: opts.maxLineBytes } : {}),
      ...(opts.batchMaxLines !== undefined ? { batchMaxLines: opts.batchMaxLines } : {}),
      ...(opts.batchMaxBytes !== undefined ? { batchMaxBytes: opts.batchMaxBytes } : {}),
      ...(opts.outputMaxBytes !== undefined ? { outputMaxBytes: opts.outputMaxBytes } : {}),
      ...(opts.outputLineMaxChars !== undefined
        ? { outputLineMaxChars: opts.outputLineMaxChars }
        : {}),
      ...(opts.skipFooterMaxEntries !== undefined
        ? { skipFooterMaxEntries: opts.skipFooterMaxEntries }
        : {}),
      ...(opts.skipPathMaxChars !== undefined
        ? { skipPathMaxChars: opts.skipPathMaxChars }
        : {}),
      ...(opts.skipFooterMaxBytes !== undefined
        ? { skipFooterMaxBytes: opts.skipFooterMaxBytes }
        : {}),
    });
  } else if (name === "read") {
    const blob =
      caseSpec.artifactBlob?.base64 !== undefined
        ? Buffer.from(caseSpec.artifactBlob.base64, "base64")
        : Buffer.from(caseSpec.artifactBlob ?? "", "utf8");
    tool = rt.createReadTool({
      ...common,
      ...(opts.maxBytes !== undefined ? { maxBytes: opts.maxBytes } : {}),
      ...(opts.chunkBytes !== undefined ? { chunkBytes: opts.chunkBytes } : {}),
      ...(caseSpec.artifacts ? { artifacts: fixtureArtifacts(blob) } : {}),
    });
  } else {
    throw new Error(`unknown tool-level ${name}`);
  }
  const expected = [];
  for (const op of caseSpec.ops) {
    expected.push(
      await attempt(async () => {
        const input = unScrub(op.input ?? {}, ctx.needles);
        return canonicalResult(await tool.execute(input, undefined));
      }),
    );
  }
  return expected;
}

// ---------- case driver ----------

async function runCase(index, spec) {
  const ctx = await caseDir(index, spec);
  if (!ctx) return null; // symlink unavailable → drop the case
  if (spec.tool) {
    const expected = await runToolLevel(spec, ctx);
    return { ...spec, expected: scrub(expected, ctx.needles) };
  }
  const { opts, scopeController, journal } = runtimeOptions(spec.runtime ?? {}, ctx);
  let runtime;
  try {
    // `createSandboxedTools` と同じ配線: handlers の boundary
    // operations は runtime.observeAccess へ委譲する遅延 observer を持つ。
    const observed = {
      ...opts,
      observeAccess: (...args) => runtime.observeAccess(...args),
    };
    runtime = rt.createToolRuntime({
      ...opts,
      handlers: pi.createPiToolHandlers(observed),
    });
  } catch (e) {
    return {
      ...spec,
      expected: [{ error: cap(e) }],
      state: null,
    };
  }
  const ctx2 = { ...ctx, scopeController };
  const expected = [];
  for (const op of spec.ops ?? []) {
    if (op.parallel) expected.push(await runParallel(runtime, op, ctx.needles, ctx2));
    else if (op.op) expected.push(await runEffect(runtime, op, ctx2));
    else expected.push(await runCall(runtime, op, ctx.needles));
  }
  const state = spec.runtime ? await projectState(runtime, spec.runtime) : null;
  await runtime.dispose();
  void journal;
  return {
    ...spec,
    expected: scrub(expected, ctx.needles),
    state: state === null ? null : scrub(state, ctx.needles),
  };
}

async function dumpCases(file, specs) {
  const cases = [];
  for (let i = 0; i < specs.length; i++) {
    const c = await runCase(`${file.replace(/\W/g, "_")}_${i}`, specs[i]);
    if (c) cases.push(c);
  }
  writeFixture(file, { platform: PLATFORM, cases });
}

// ===================================================================
// helpers for the corpus
// ===================================================================

const HELLO = "alpha\nbeta\ngamma\ndelta\nepsilon\n";
const R_SUBTREE = {
  reads: [{ path: ".", scope: "subtree" }],
  writes: [{ path: ".", scope: "subtree" }],
  produces: [{ path: ".", scope: "subtree" }],
};

// node -e scripts (argv[0] is the script-less node path; extra args start
// at argv[1]). resolveProcessPolicy forbids `;` `|` `&` `<` `>` `` ` ``
// `$(` and newlines in argv — scripts use comma operators, `function` and
// ternaries instead.
const HOLD =
  "setInterval(function(){if(require('fs').existsSync(process.argv[1])){console.log('released'),process.exit(0)}},20)," +
  "setTimeout(function(){process.exit(3)},20000)";
const NODE_TASKS = {
  hi: { command: ["<NODE>", "-e", "console.log('hello process')"], inputs: [], outputs: [] },
  fail: {
    command: [
      "<NODE>",
      "-e",
      "console.log('before failure'),console.error('failure detail'),process.exitCode=7",
    ],
    inputs: [],
    outputs: [],
  },
  output: {
    command: ["<NODE>", "-e", "console.log('a'.repeat(1000))"],
    inputs: [],
    outputs: [],
  },
  sleep: { command: ["<NODE>", "-e", "setInterval(function(){},500)"], inputs: [], outputs: [] },
  inspect: {
    command: [
      "<NODE>",
      "-e",
      "console.log(JSON.stringify({cwdTail:process.cwd().replaceAll('\\\\','/').split('/').at(-1),args:process.argv.slice(1),explicit:process.env.ONLY_THIS??null,path:process.env.PATH?process.env.PATH:null}))",
      "arg1",
      "arg two",
    ],
    inputs: [],
    outputs: [],
    env: { ONLY_THIS: "yes" },
  },
  hold: { command: ["<NODE>", "-e", HOLD, "<ROOT>/gate.txt"], inputs: [], outputs: ["done.txt"] },
  holdn: { command: ["<NODE>", "-e", HOLD, "<ROOT>/gate.txt"], inputs: [], outputs: [] },
  node: { command: ["node", "-e", "console.log('bare node')"], inputs: [], outputs: [] },
};

// ===================================================================
// read.json
// ===================================================================

await dumpCases("read.json", [
  {
    name: "whole file",
    files: { "a.txt": HELLO },
    runtime: { names: ["read"] },
    ops: [{ tool: "read", input: { path: "a.txt" } }],
  },
  {
    name: "offset skips leading lines",
    files: { "a.txt": HELLO },
    runtime: { names: ["read"] },
    ops: [{ tool: "read", input: { path: "a.txt", offset: 3 } }],
  },
  {
    name: "limit truncates",
    files: { "a.txt": HELLO },
    runtime: { names: ["read"] },
    ops: [{ tool: "read", input: { path: "a.txt", limit: 2 } }],
  },
  {
    name: "offset beyond eof",
    files: { "a.txt": HELLO },
    runtime: { names: ["read"] },
    ops: [{ tool: "read", input: { path: "a.txt", offset: 99 } }],
  },
  {
    name: "byteInLine requires offset",
    files: { "a.txt": "x".repeat(100) + "\nnext\n" },
    runtime: { names: ["read"] },
    ops: [{ tool: "read", input: { path: "a.txt", byteInLine: 50 } }],
  },
  {
    name: "byteInLine continues an oversized line",
    files: { "a.txt": "x".repeat(100) + "\nnext\n" },
    runtime: { names: ["read"] },
    ops: [
      { tool: "read", input: { path: "a.txt", offset: 1, byteInLine: 40 } },
      { tool: "read", input: { path: "a.txt", offset: 1, byteInLine: 200 } },
    ],
  },
  {
    name: "binary rejected",
    files: { "bin.dat": { base64: "AAECAwQFBgc=" } },
    runtime: { names: ["read"] },
    ops: [{ tool: "read", input: { path: "bin.dat" } }],
  },
  {
    name: "image extension rejected",
    files: { "i.png": "not really a png" },
    runtime: { names: ["read"] },
    ops: [{ tool: "read", input: { path: "i.png" } }],
  },
  {
    name: "nonexistent",
    files: {},
    runtime: { names: ["read"] },
    ops: [{ tool: "read", input: { path: "missing.txt" } }],
  },
  {
    name: "directory rejected",
    files: { "sub/x.txt": "x" },
    runtime: { names: ["read"] },
    ops: [{ tool: "read", input: { path: "sub" } }],
  },
  {
    name: "byte cap reaches 50KB",
    files: { "big.txt": { repeat: ["0123456789\n", 6000] } },
    runtime: { names: ["read"] },
    ops: [{ tool: "read", input: { path: "big.txt" } }],
  },
  {
    name: "artifact source read",
    runtime: {
      names: ["read"],
      artifacts: true,
      artifactBlob: "0123456789abcdef---artifact-bytes---tail",
    },
    ops: [
      {
        tool: "read",
        input: {
          source: {
            kind: "artifact",
            reference: {
              artifact: "fixture/40",
              runId: "fixture-run",
              producer: { jobId: "tool-runtime", name: "blob" },
              bytes: 40,
              preview: "0123456789abcdef",
              truncated: true,
            },
          },
          offset: 8,
          maxBytes: 10,
        },
      },
    ],
  },
  {
    name: "artifact publication",
    files: { "a.txt": "publish me please, this is longer than sixteen bytes" },
    runtime: { names: ["read"], artifacts: true },
    ops: [{ tool: "read", input: { path: "a.txt", artifact: "pub" } }],
  },
  {
    name: "artifact reader unavailable",
    runtime: { names: ["read"] },
    ops: [
      {
        tool: "read",
        input: {
          source: {
            kind: "artifact",
            reference: {
              artifact: "fixture/1",
              runId: "fixture-run",
              producer: { jobId: "tool-runtime", name: "blob" },
              bytes: 1,
              preview: "x",
              truncated: false,
            },
          },
          offset: 0,
          maxBytes: 4,
        },
      },
    ],
  },
  {
    name: "invalid input is uncounted",
    files: { "a.txt": "x\n" },
    runtime: { names: ["read"] },
    ops: [{ tool: "read", input: { path: 5 } }],
    // 設計固定ルール差分: schema validator は ajv → jsonschema —
    // union branch から報告される instancePath 集合が異なる(ajv は
    // `""` + `/path`、jsonschema は `""` のみ)。
    expectRust: {
      ops: {
        "0": {
          error: {
            kind: "cheaper",
            code: "INVALID_INPUT",
            details: { errors: [{ instancePath: "" }], tool: "read" },
            exitCode: 2,
            retryable: false,
          },
        },
      },
    },
  },
  {
    name: "denylisted path",
    files: { ".env": "SECRET=1\n", "ok.txt": "ok\n" },
    runtime: { names: ["read"], denylist: [".env"] },
    ops: [
      { tool: "read", input: { path: ".env" } },
      { tool: "read", input: { path: "ok.txt" } },
    ],
  },
  {
    name: "workspace escape denied",
    runtime: { names: ["read"] },
    ops: [{ tool: "read", input: { path: "../outside.txt" } }],
  },
  {
    name: "tool-level byte cap injection",
    tool: { name: "read", opts: { maxBytes: 40, chunkBytes: 8 } },
    files: { "a.txt": "0123456789\n0123456789\n0123456789\n0123456789\n" },
    ops: [{ input: { path: "a.txt" } }, { input: { path: "a.txt", limit: 2 } }],
  },
]);

// ===================================================================
// grep.json
// ===================================================================

const GREP_FILES = {
  "a.txt": "foo bar\nnumber 42\nnothing\n",
  "sub/b.js": "FOO again\nx = 1 + 41\n",
  "sub/deep/c.md": "markdown foo\nnumber 007\n",
  "bin.dat": { base64: "Zm9vAAECAwQ=" }, // "foo" + NUL → binary
  ".git/ignored.txt": "foo in git\n",
  "node_modules/pkg.js": "foo in node_modules\n",
};

await dumpCases("grep.json", [
  {
    name: "literal match across tree",
    files: GREP_FILES,
    runtime: { names: ["grep"] },
    ops: [{ tool: "grep", input: { pattern: "foo" } }],
  },
  {
    name: "regex digit class",
    files: GREP_FILES,
    runtime: { names: ["grep"] },
    ops: [{ tool: "grep", input: { pattern: "\\d+" } }],
  },
  {
    name: "anchor and alternation",
    files: GREP_FILES,
    runtime: { names: ["grep"] },
    ops: [
      { tool: "grep", input: { pattern: "^foo" } },
      { tool: "grep", input: { pattern: "(foo|bar)" } },
    ],
  },
  {
    name: "character class + ignoreCase",
    files: GREP_FILES,
    runtime: { names: ["grep"] },
    ops: [
      { tool: "grep", input: { pattern: "[0-9a-f]{2,3}" } },
      { tool: "grep", input: { pattern: "foo", ignoreCase: true } },
    ],
  },
  {
    name: "path scope",
    files: GREP_FILES,
    runtime: { names: ["grep"] },
    ops: [{ tool: "grep", input: { pattern: "foo", path: "sub" } }],
  },
  {
    name: "glob filter",
    files: GREP_FILES,
    runtime: { names: ["grep"] },
    ops: [
      { tool: "grep", input: { pattern: "\\w+", glob: "*.js" } },
      { tool: "grep", input: { pattern: "foo|x = \\d+", glob: "**/*.js" } },
    ],
  },
  {
    name: "context merges contiguous blocks",
    files: { "ctx.txt": "a\nmatch one\nb\nc\nmatch two\nd\n" },
    runtime: { names: ["grep"] },
    ops: [{ tool: "grep", input: { pattern: "match", context: 1 } }],
  },
  {
    name: "context separates distant blocks",
    files: { "ctx.txt": "a\nmatch one\nb\nc\nd\ne\nmatch two\nf\n" },
    runtime: { names: ["grep"] },
    ops: [{ tool: "grep", input: { pattern: "match", context: 1 } }],
  },
  {
    name: "maxResults truncation",
    files: { "m.txt": { repeat: ["hit\n", 10] } },
    runtime: { names: ["grep"] },
    ops: [{ tool: "grep", input: { pattern: "hit", maxResults: 3 } }],
  },
  {
    name: "no matches",
    files: { "a.txt": "nothing here\n" },
    runtime: { names: ["grep"] },
    ops: [{ tool: "grep", input: { pattern: "zzz" } }],
  },
  {
    name: "binary file skipped in footer",
    files: GREP_FILES,
    runtime: { names: ["grep"] },
    ops: [{ tool: "grep", input: { pattern: "ZZZnomatch" } }],
  },
  {
    name: "denylisted file hidden",
    files: { "visible.txt": "foo\n", "secret.txt": "foo\n" },
    runtime: { names: ["grep"], denylist: ["secret.txt"] },
    ops: [{ tool: "grep", input: { pattern: "foo" } }],
  },
  {
    name: "invalid regex propagates",
    files: { "a.txt": "x\n" },
    runtime: { names: ["grep"] },
    ops: [{ tool: "grep", input: { pattern: "(" } }],
    // 設計固定ルール差分: regex engine は V8 → regress(error wording が
    // 異なる — semantic は同一: invalid pattern で fail)。
    expectRust: {
      ops: {
        "0": {
          error: {
            kind: "other",
            name: "Error",
            message: "invalid grep pattern: Unbalanced parenthesis",
          },
        },
      },
      state: {
        operations: [
          {
            error: {
              name: "Error",
              message: "invalid grep pattern: Unbalanced parenthesis",
            },
          },
        ],
      },
    },
  },
  {
    name: "invalid input is uncounted",
    files: { "a.txt": "x\n" },
    runtime: { names: ["grep"] },
    ops: [{ tool: "grep", input: { pattern: [] } }],
  },
  {
    name: "oversized line skip footer",
    tool: { name: "grep", opts: { maxLineBytes: 32 } },
    files: { "a.txt": "short foo\n" + "l".repeat(200) + " foo\n" },
    ops: [{ input: { pattern: "foo" } }],
  },
  {
    name: "per-file eval timeout skip footer",
    tool: { name: "grep", opts: { evalPerFileMs: 0 } },
    files: { "a.txt": "foo\nbar\n" },
    ops: [{ input: { pattern: "foo" } }],
  },
  {
    name: "total eval budget exhaustion",
    tool: { name: "grep", opts: { evalTotalMs: 0 } },
    files: { "a.txt": "foo\n" },
    ops: [{ input: { pattern: "foo" } }],
  },
  {
    name: "scan deadline exhaustion",
    tool: { name: "grep", opts: { scanDeadlineMs: 0 } },
    files: { "a.txt": "foo\n" },
    ops: [{ input: { pattern: "foo" } }],
  },
  {
    name: "output byte cap",
    tool: { name: "grep", opts: { outputMaxBytes: 40 } },
    files: { "m.txt": { repeat: ["match this line\n", 10] } },
    ops: [{ input: { pattern: "match" } }],
  },
  {
    name: "output line truncation",
    tool: { name: "grep", opts: { outputLineMaxChars: 40 } },
    files: { "a.txt": "hit " + "x".repeat(200) + "\n" },
    ops: [{ input: { pattern: "hit" } }],
  },
  {
    // match 本文 (`text.slice(0, outputLineMaxChars)`) と skip footer の
    // path (`entry.slice(0, skipPathMaxChars)`) は UTF-16 code unit 単位。
    // astral char (2 unit) で unit 境界が char 境界に一致する値を選ぶ
    // (半端 surrogate は Rust `String` で表現不能なため境界またぎは
    // 意図的に避ける — backoff は許容差異)。
    name: "UTF-16 truncation (line + skip path)",
    tool: {
      name: "grep",
      opts: { outputLineMaxChars: 4, skipPathMaxChars: 5, maxLineBytes: 8 },
    },
    files: {
      // units: 😀=2 + x=1 + ".txt"=4 → 7 units → slice(0,5) = "😀x.t"
      "😀x.txt": "toolongline hit\nok hit\n",
      // "a😀b hit" → units 1+2+1 → slice(0,4) = "a😀b"
      "b.txt": "a😀b hit\n",
    },
    ops: [{ input: { pattern: "hit" } }],
  },
  {
    name: "batchMaxLines truncation",
    tool: { name: "grep", opts: { batchMaxLines: 2 } },
    files: { "m.txt": "hit a\nhit b\nhit c\nhit d\n" },
    ops: [{ input: { pattern: "hit" } }],
  },
]);

// ===================================================================
// find.json
// ===================================================================

await dumpCases("find.json", [
  {
    name: "flat glob",
    files: { "a.txt": "", "b.md": "", "sub/c.txt": "" },
    runtime: { names: ["find"] },
    ops: [{ tool: "find", input: { pattern: "*.txt" } }],
  },
  {
    name: "recursive glob",
    files: { "a.txt": "", "sub/b.txt": "", "sub/deep/c.txt": "", "sub/deep/d.md": "" },
    runtime: { names: ["find"] },
    ops: [{ tool: "find", input: { pattern: "**/*.txt" } }],
  },
  {
    name: "nested prefix glob",
    files: { "src/a.js": "", "src/x/b.js": "", "other/c.js": "" },
    runtime: { names: ["find"] },
    ops: [{ tool: "find", input: { pattern: "src/**/*.js" } }],
  },
  {
    name: "question mark and class",
    files: { "f1.txt": "", "f2.txt": "", "fa.txt": "", "g1.txt": "" },
    runtime: { names: ["find"] },
    ops: [
      { tool: "find", input: { pattern: "f?.txt" } },
      { tool: "find", input: { pattern: "f[0-9].txt" } },
    ],
  },
  {
    name: "brace expansion",
    files: { "a.txt": "", "b.md": "", "c.rs": "" },
    runtime: { names: ["find"] },
    ops: [{ tool: "find", input: { pattern: "*.{txt,md}" } }],
  },
  {
    name: "path scope",
    files: { "sub/a.txt": "", "root.txt": "" },
    runtime: { names: ["find"] },
    ops: [
      { tool: "find", input: { pattern: "*.txt", path: "sub" } },
      { tool: "find", input: { pattern: "**/*.txt", path: "sub" } },
    ],
  },
  {
    name: "no matches",
    files: { "a.txt": "" },
    runtime: { names: ["find"] },
    ops: [{ tool: "find", input: { pattern: "*.zzz" } }],
  },
  {
    name: "maxResults truncation",
    files: { "a.txt": "", "b.txt": "", "c.txt": "", "d.txt": "" },
    runtime: { names: ["find"] },
    ops: [{ tool: "find", input: { pattern: "*.txt", maxResults: 2 } }],
  },
  {
    name: "git and node_modules skipped",
    files: {
      "ok.txt": "",
      ".git/hidden.txt": "",
      "node_modules/pkg/x.txt": "",
      "sub/.git/y.txt": "",
    },
    runtime: { names: ["find"] },
    ops: [{ tool: "find", input: { pattern: "**/*.txt" } }],
  },
  {
    name: "denylisted path hidden",
    files: { "a.txt": "", "secret.txt": "", "sub/private/x.txt": "" },
    runtime: { names: ["find"], denylist: ["secret.txt", "sub/private"] },
    ops: [{ tool: "find", input: { pattern: "**/*.txt" } }],
  },
  {
    name: "unreadable entry footer",
    files: { "a.txt": "", "real/t.txt": "" },
    symlinks: [{ target: "missing-target", link: "dangling", type: "junction" }],
    runtime: { names: ["find"], symlinkPolicy: "within-workspace" },
    ops: [{ tool: "find", input: { pattern: "**/*" } }],
  },
  {
    name: "invalid input is uncounted",
    runtime: { names: ["find"] },
    ops: [{ tool: "find", input: { pattern: 5 } }],
  },
]);

// ===================================================================
// ls.json
// ===================================================================

await dumpCases("ls.json", [
  {
    name: "lists files and directories",
    files: { "a.txt": "", "B.txt": "", "sub/x.txt": "", ".hidden": "" },
    runtime: { names: ["ls"] },
    ops: [{ tool: "ls", input: {} }],
  },
  {
    name: "limit reaches entry cap",
    files: { "a.txt": "", "b.txt": "", "c.txt": "", "d.txt": "" },
    runtime: { names: ["ls"] },
    ops: [{ tool: "ls", input: { limit: 2 } }],
  },
  {
    name: "byte cap notice",
    repeatFiles: { many: { dir: "many", prefix: "file", count: 2600, content: "x" } },
    runtime: { names: ["ls"] },
    ops: [{ tool: "ls", input: { path: "many" } }],
  },
  {
    name: "empty directory",
    files: { "empty/.keep": "" },
    runtime: { names: ["ls"] },
    ops: [{ tool: "ls", input: { path: "empty" } }],
  },
  {
    name: "missing path",
    files: {},
    runtime: { names: ["ls"] },
    ops: [{ tool: "ls", input: { path: "missing" } }],
  },
  {
    name: "not a directory",
    files: { "a.txt": "" },
    runtime: { names: ["ls"] },
    ops: [{ tool: "ls", input: { path: "a.txt" } }],
  },
  {
    name: "denylisted entries hidden",
    files: { "a.txt": "", "secret.txt": "", "ok/b.txt": "" },
    runtime: { names: ["ls"], denylist: ["secret.txt"] },
    ops: [{ tool: "ls", input: {} }],
  },
  {
    name: "workspace escape denied",
    runtime: { names: ["ls"] },
    ops: [{ tool: "ls", input: { path: ".." } }],
  },
  {
    name: "invalid input is uncounted",
    runtime: { names: ["ls"] },
    ops: [{ tool: "ls", input: { path: 3 } }],
  },
]);

// ===================================================================
// write.json
// ===================================================================

await dumpCases("write.json", [
  {
    name: "creates a new file",
    runtime: { names: ["write"] },
    ops: [{ tool: "write", input: { path: "new.txt", content: "created\n" } }],
  },
  {
    name: "creates parent directories",
    runtime: { names: ["write"] },
    ops: [{ tool: "write", input: { path: "a/b/c.txt", content: "nested\n" } }],
  },
  {
    name: "overwrites an existing file",
    files: { "a.txt": "old\n" },
    runtime: { names: ["write"] },
    ops: [{ tool: "write", input: { path: "a.txt", content: "new\n" } }],
  },
  {
    name: "write scope denial",
    runtime: {
      names: ["write"],
      writeScope: ["allowed.txt"],
      // manifest writes ⊆ writeScope の整合制約を満たす範囲宣言。
      resourceManifest: {
        reads: [{ path: ".", scope: "subtree" }],
        writes: [{ path: "allowed.txt", scope: "exact" }],
      },
    },
    ops: [
      { tool: "write", input: { path: "denied.txt", content: "x" } },
      { tool: "write", input: { path: "allowed.txt", content: "y" } },
    ],
  },
  {
    name: "denylisted path",
    runtime: { names: ["write"], denylist: [".env"] },
    ops: [{ tool: "write", input: { path: ".env", content: "SECRET=1" } }],
  },
  {
    name: "workspace escape denied",
    runtime: { names: ["write"] },
    ops: [{ tool: "write", input: { path: "../escape.txt", content: "x" } }],
  },
  {
    name: "no-change write excluded from journal",
    files: { "a.txt": "same\n" },
    runtime: { names: ["write"] },
    ops: [{ tool: "write", input: { path: "a.txt", content: "same\n" } }],
  },
  {
    name: "second write keeps first before",
    runtime: { names: ["write"] },
    ops: [
      { tool: "write", input: { path: "a.txt", content: "first\n" } },
      { tool: "write", input: { path: "a.txt", content: "second\n" } },
    ],
  },
  {
    name: "invalid input is uncounted",
    runtime: { names: ["write"] },
    ops: [{ tool: "write", input: { path: "a.txt" } }],
  },
]);

// ===================================================================
// edit.json
// ===================================================================

await dumpCases("edit.json", [
  {
    name: "single replacement",
    files: { "a.txt": "alpha\nbeta\ngamma\n" },
    runtime: { names: ["edit"] },
    ops: [
      {
        tool: "edit",
        input: { path: "a.txt", edits: [{ oldText: "beta", newText: "BETA" }] },
      },
    ],
  },
  {
    name: "multiple replacements",
    files: { "a.txt": "alpha\nbeta\ngamma\ndelta\n" },
    runtime: { names: ["edit"] },
    ops: [
      {
        tool: "edit",
        input: {
          path: "a.txt",
          edits: [
            { oldText: "alpha", newText: "ALPHA" },
            { oldText: "gamma", newText: "GAMMA" },
          ],
        },
      },
    ],
  },
  {
    name: "edits as JSON string",
    files: { "a.txt": "x = 1\n" },
    runtime: { names: ["edit"] },
    ops: [
      {
        tool: "edit",
        input: {
          path: "a.txt",
          edits: '[{"oldText":"x = 1","newText":"x = 2"}]',
        },
      },
    ],
  },
  {
    name: "single edit object",
    files: { "a.txt": "x = 1\n" },
    runtime: { names: ["edit"] },
    ops: [
      { tool: "edit", input: { path: "a.txt", edits: { oldText: "x = 1", newText: "x = 2" } } },
    ],
  },
  {
    name: "legacy oldText newText",
    files: { "a.txt": "x = 1\n" },
    runtime: { names: ["edit"] },
    ops: [{ tool: "edit", input: { path: "a.txt", oldText: "x = 1", newText: "x = 2" } }],
  },
  {
    name: "CRLF endings restored",
    files: { "a.txt": { base64: "YWxwaGENCmJldGENCmdhbW1hDQo=" } }, // alpha\r\nbeta\r\ngamma\r\n
    runtime: { names: ["edit"] },
    ops: [
      {
        tool: "edit",
        input: { path: "a.txt", edits: [{ oldText: "beta", newText: "BETA" }] },
      },
    ],
  },
  {
    name: "BOM preserved",
    files: { "a.txt": { base64: "77u/YWxwaGEKYmV0YQo=" } }, // \uFEFFalpha\nbeta\n
    runtime: { names: ["edit"] },
    ops: [
      {
        tool: "edit",
        input: { path: "a.txt", edits: [{ oldText: "beta", newText: "BETA" }] },
      },
    ],
  },
  {
    name: "overlapping edits rejected",
    files: { "a.txt": "abcdefghij\n" },
    runtime: { names: ["edit"] },
    ops: [
      {
        tool: "edit",
        input: {
          path: "a.txt",
          edits: [
            { oldText: "abcde", newText: "X" },
            { oldText: "cdefg", newText: "Y" },
          ],
        },
      },
    ],
  },
  {
    name: "duplicate oldText rejected",
    files: { "a.txt": "same\nsame\n" },
    runtime: { names: ["edit"] },
    ops: [
      {
        tool: "edit",
        input: { path: "a.txt", edits: [{ oldText: "same", newText: "diff" }] },
      },
    ],
  },
  {
    name: "oldText not found",
    files: { "a.txt": "present\n" },
    runtime: { names: ["edit"] },
    ops: [
      {
        tool: "edit",
        input: { path: "a.txt", edits: [{ oldText: "missing", newText: "x" }] },
      },
    ],
  },
  {
    name: "no-change edit",
    files: { "a.txt": "alpha\nbeta\n" },
    runtime: { names: ["edit"] },
    ops: [
      {
        tool: "edit",
        input: { path: "a.txt", edits: [{ oldText: "beta", newText: "beta" }] },
      },
    ],
  },
  {
    name: "fuzzy whitespace match",
    files: { "a.txt": "alpha\n  beta   spaced\ngamma\n" },
    runtime: { names: ["edit"] },
    ops: [
      {
        tool: "edit",
        input: {
          path: "a.txt",
          edits: [{ oldText: "beta   spaced", newText: "BETA" }],
        },
      },
    ],
  },
  {
    name: "missing file",
    runtime: { names: ["edit"] },
    ops: [
      {
        tool: "edit",
        input: { path: "no.txt", edits: [{ oldText: "x", newText: "y" }] },
      },
    ],
  },
  {
    // ops.access(absolutePath) = fs.access(R_OK|W_OK): read-only file は
    // EACCES で "Could not edit file" となる。mode は 8 進数(0o444=292)。
    name: "read-only file rejected",
    files: { "ro.txt": { content: "x\n", mode: 0o444 } },
    runtime: { names: ["edit"] },
    ops: [
      {
        tool: "edit",
        input: { path: "ro.txt", edits: [{ oldText: "x", newText: "y" }] },
      },
    ],
  },
  {
    // oldText " " は fuzzy normalize で空になるため countOccurrences は
    // `split("").length - 1` — UTF-16 unit 数(astral char は 2 unit)。
    name: "empty fuzzy needle counts UTF-16 units",
    files: { "a.txt": "😀😀 \n" },
    runtime: { names: ["edit"] },
    ops: [
      {
        tool: "edit",
        input: { path: "a.txt", edits: [{ oldText: " ", newText: "x" }] },
      },
    ],
  },
]);

// ===================================================================
// count_tokens.json
// ===================================================================

await dumpCases("count_tokens.json", [
  {
    name: "counts a file",
    files: { "a.txt": "hello world, this is a token count test\n" },
    runtime: { names: ["count_tokens"] },
    ops: [{ tool: "count_tokens", input: { path: "a.txt" } }],
    // 設計固定ルール差分: tokenizer identity は js-tiktoken →
    // tiktoken-rs(token 数は同一、crates/cheaper-core/src/token/count.rs)。
    expectRust: {
      ops: {
        "0": {
          ok: {
            content: [
              {
                type: "text",
                text: '{"path":"a.txt","tokens":10,"bytes":40,"tokenizer":{"name":"o200k_base","implementation":"tiktoken-rs","version":"0.12.1"}}',
              },
            ],
            details: {
              path: "a.txt",
              tokens: 10,
              bytes: 40,
              tokenizer: {
                name: "o200k_base",
                implementation: "tiktoken-rs",
                version: "0.12.1",
              },
            },
          },
        },
      },
      // meter は返却 text の token 数 — Rust 側の text が異なるため差分
      // (tiktoken-rs 実測値: `{"path":..,"tokenizer":tiktoken-rs}` の
      // JSON で 40)。
      state: { meter: { sourceTokens: 40 } },
    },
  },
  {
    name: "denylisted path",
    files: { "secret.txt": "s\n" },
    runtime: { names: ["count_tokens"], denylist: ["secret.txt"] },
    ops: [{ tool: "count_tokens", input: { path: "secret.txt" } }],
  },
  {
    name: "nonexistent file",
    runtime: { names: ["count_tokens"] },
    ops: [{ tool: "count_tokens", input: { path: "missing.txt" } }],
  },
  {
    name: "invalid input is uncounted",
    runtime: { names: ["count_tokens"] },
    ops: [{ tool: "count_tokens", input: { path: true } }],
  },
]);

// ===================================================================
// process.json
// ===================================================================

await dumpCases("process.json", [
  {
    name: "successful task",
    runtime: { names: ["process"], process: { tasks: NODE_TASKS } },
    ops: [{ tool: "process", input: { task: "hi" } }],
  },
  {
    name: "nonzero exit",
    runtime: { names: ["process"], process: { tasks: NODE_TASKS } },
    ops: [{ tool: "process", input: { task: "fail" } }],
  },
  {
    name: "output cap truncates",
    runtime: {
      names: ["process"],
      process: { tasks: NODE_TASKS, maxOutputBytes: 100 },
    },
    ops: [{ tool: "process", input: { task: "output" } }],
  },
  {
    name: "timeout kills the process",
    runtime: {
      names: ["process"],
      process: { tasks: NODE_TASKS, timeout: "400ms", killGrace: "500ms" },
    },
    ops: [{ tool: "process", input: { task: "sleep" } }],
  },
  {
    name: "unknown task rejected before admission",
    runtime: { names: ["process"], process: { tasks: NODE_TASKS } },
    ops: [{ tool: "process", input: { task: "missing" } }],
  },
  {
    name: "bare node task is an explicit Rust error",
    runtime: { names: ["process"], process: { tasks: NODE_TASKS } },
    ops: [{ tool: "process", input: { task: "node" } }],
    // 設計固定ルール: Rust には `process.execPath` fast-path 相当が無く
    // bare `node` は PATH 解決せず POLICY_VIOLATION を返す
    // (crates/cheaper-tools/src/tools/process_executable.rs)。
    expectRust: {
      ops: {
        "0": {
          error: {
            kind: "cheaper",
            code: "POLICY_VIOLATION",
            details: null,
            exitCode: 4,
            retryable: false,
          },
        },
      },
      state: {
        operations: [
          {
            toolName: "process",
            state: "failed",
            settlement: "settled",
            resourceClaims: [
              {
                mode: "shared",
                resource: { kind: "workspace", path: "<ROOT>", scope: "subtree" },
              },
            ],
            error: {
              name: "CheaperError",
              message:
                'process task "node" is unsupported: the Rust runtime has no Node.js host executable',
              code: "POLICY_VIOLATION",
            },
          },
        ],
        meter: { sourceTokens: 0 },
      },
    },
  },
  {
    name: "minimal environment",
    runtime: { names: ["process"], process: { tasks: NODE_TASKS } },
    ops: [{ tool: "process", input: { task: "inspect" } }],
  },
  {
    name: "task cwd subdir",
    files: { "sub/.keep": "" },
    runtime: {
      names: ["process"],
      process: {
        tasks: {
          inspect: { ...NODE_TASKS.inspect, cwd: "sub", env: undefined },
        },
      },
    },
    ops: [{ tool: "process", input: { task: "inspect" } }],
  },
  {
    name: "invalid input is uncounted",
    runtime: { names: ["process"], process: { tasks: NODE_TASKS } },
    ops: [{ tool: "process", input: { task: 5 } }],
  },
]);

// ===================================================================
// runtime.json — scheduler / budget / journal / accounting
// ===================================================================

await dumpCases("runtime.json", [
  {
    name: "counting map: executed, denied, invalid, limited",
    files: { "a.txt": "hello\n" },
    runtime: {
      names: ["read", "write"],
      maxToolCalls: 3,
      writeScope: ["a.txt"],
      resourceManifest: {
        reads: [{ path: "a.txt", scope: "exact" }],
        writes: [{ path: "a.txt", scope: "exact" }],
      },
    },
    ops: [
      { tool: "read", input: { path: "a.txt" } }, // toolCalls=1
      { tool: "write", input: { path: "denied.txt", content: "x" } }, // toolCalls=2, rejected=1
      { tool: "read", input: { path: 5 } }, // uncounted
      { tool: "read", input: { path: "a.txt" } }, // toolCalls=3
      { tool: "read", input: { path: "a.txt" } }, // rejected=2 (limit)
    ],
    // 設計固定ルール差分: ajv → jsonschema の union-branch
    // instancePath 報告差(read.json の "invalid input is uncounted" と
    // 同一)。
    expectRust: {
      ops: {
        "2": {
          error: {
            kind: "cheaper",
            code: "INVALID_INPUT",
            details: { errors: [{ instancePath: "" }], tool: "read" },
            exitCode: 2,
            retryable: false,
          },
        },
      },
    },
  },
  {
    name: "unknown tool is uncounted",
    runtime: { names: ["read"] },
    ops: [{ tool: "nope", input: {} }],
  },
  {
    name: "serialized mutations queue behind exclusive hold",
    runtime: {
      names: ["process", "write"],
      concurrency: { mutations: "serialized" },
      process: { tasks: NODE_TASKS, timeout: "1500ms", killGrace: "500ms" },
    },
    ops: [
      {
        parallel: [
          { tool: "process", input: { task: "hold" } },
          { tool: "write", input: { path: "gate.txt", content: "go" } },
        ],
      },
    ],
  },
  {
    name: "per-resource mutations proceed concurrently",
    runtime: {
      names: ["process", "write"],
      concurrency: { mutations: "per-resource" },
      resourceManifest: R_SUBTREE,
      process: { tasks: NODE_TASKS, timeout: "15s", killGrace: "500ms" },
    },
    ops: [
      {
        parallel: [
          { tool: "process", input: { task: "hold" } },
          { tool: "write", input: { path: "gate.txt", content: "go" } },
        ],
      },
    ],
  },
  {
    name: "process pool serializes tasks",
    runtime: {
      names: ["process"],
      process: { tasks: NODE_TASKS, maxConcurrent: 1 },
    },
    ops: [
      {
        parallel: [
          { tool: "process", input: { task: "holdn" } },
          { tool: "process", input: { task: "hi" } },
        ],
        then: [{ op: "touch", path: "gate.txt" }],
      },
    ],
  },
  {
    name: "queued operation aborted before start",
    runtime: {
      names: ["process", "write"],
      concurrency: { mutations: "serialized" },
      process: { tasks: NODE_TASKS },
    },
    ops: [
      {
        parallel: [
          { tool: "process", input: { task: "hold" } },
          {
            tool: "write",
            input: { path: "x.txt", content: "x" },
            abortReason: "stop queued",
          },
        ],
        then: [{ op: "touch", path: "gate.txt" }],
      },
    ],
  },
  {
    name: "drain aborts in-flight with scope reason",
    runtime: {
      names: ["process"],
      process: { tasks: NODE_TASKS },
      signal: true,
    },
    ops: [
      {
        parallel: [{ tool: "process", input: { task: "holdn" } }],
        then: [
          { op: "abort-scope", reason: "drain now" },
          { op: "drain" },
        ],
      },
    ],
  },
  {
    name: "journal: created, modified, no-change excluded",
    files: { "mod.txt": "before\n", "same.txt": "same\n" },
    runtime: { names: ["write", "edit"] },
    ops: [
      { tool: "write", input: { path: "new.txt", content: "created\n" } },
      { tool: "write", input: { path: "mod.txt", content: "after\n" } },
      { tool: "write", input: { path: "same.txt", content: "same\n" } },
      {
        tool: "edit",
        input: { path: "mod.txt", edits: [{ oldText: "after", newText: "edited" }] },
      },
    ],
  },
  {
    name: "aborted runtime rejects subsequent calls",
    files: { "a.txt": "x\n" },
    runtime: { names: ["read"] },
    ops: [
      { op: "abort-runtime", reason: "shut down" },
      { tool: "read", input: { path: "a.txt" } },
    ],
  },
  {
    // `controller.abort()` (reason 省略) の既定 reason は DOMException
    // `AbortError`/"This operation was aborted" — `cancelAll()` の
    // `Error("Operation cancelled")` とは別物。
    name: "reasonless abort uses AbortError",
    files: { "a.txt": "x\n" },
    runtime: { names: ["read"] },
    ops: [
      { op: "abort-runtime" },
      { tool: "read", input: { path: "a.txt" } },
    ],
  },
  {
    // `drain()` は内部で reason なし `abort()` を呼ぶため、in-flight
    // operation の cancellation reason も AbortError になる。
    name: "reasonless drain aborts in-flight with AbortError",
    runtime: { names: ["process"], process: { tasks: NODE_TASKS } },
    ops: [
      {
        parallel: [{ tool: "process", input: { task: "holdn" } }],
        then: [{ op: "drain" }],
      },
    ],
  },
  {
    name: "pre-aborted call signal",
    files: { "a.txt": "x\n" },
    runtime: { names: ["read"] },
    ops: [{ tool: "read", input: { path: "a.txt" }, preAborted: "call stopped" }],
  },
  {
    name: "ready scopes the runtime",
    files: { "a.txt": "x\n" },
    runtime: { names: ["read"] },
    ops: [{ op: "ready" }, { tool: "read", input: { path: "a.txt" } }],
  },
  {
    name: "read strictness rejects undeclared read",
    files: { "a.txt": "declared\n", "b.txt": "undeclared\n" },
    runtime: {
      names: ["read"],
      resourceManifest: { reads: [{ path: "a.txt", scope: "exact" }] },
      readStrictness: "reject",
    },
    ops: [
      { tool: "read", input: { path: "a.txt" } },
      { tool: "read", input: { path: "b.txt" } },
    ],
  },
]);

// ===================================================================
// glob.json
// ===================================================================
//
// `path.posix.matchesGlob` の直接 fixture。`{p, g, expected}` で、
// Node が SyntaxError を投げる pattern(minimatch が不正な正規表現を
// 生成する場合。例: negated class 内の `\p{...}`)は `expected: null`
// とし、Rust 側では任意の bool 結果を許容する(設計固定の divergence:
// Rust API は例外経路を持たない)。
{
  const GLOB_PAIRS = [
    // literals / stars / qmarks / classes
    ["a", "a"], ["a", "b"], ["abc", "a*c"], ["abc", "a?c"], ["abc", "?b?"],
    ["a", "[ab]"], ["c", "[ab]"], ["b", "[a-c]"], ["d", "[a-c]"],
    ["c", "[!ab]"], ["a", "[!ab]"], ["-", "[a-]"], ["-", "[-a]"],
    ["-", "[a\\-c]"], ["]", "[]]"], ["[", "[[]"], ["a", "[.]?"],
    [".x", "[.]?"], [".x", "[.][x]"], ["..", "[.][.]"], [".", "[.]"],
    ["0", "[[:digit:]]"], ["a", "[[:digit:]]"], ["a", "[[:alpha:]]"],
    ["A", "[[:upper:]]"], [" ", "[[:space:]]"], ["x", "[[:alpha:]0-9]"],
    // dot / traversal guards
    [".x", "?x"], [".x", "*"], [".x", "?"], ["a/.x", "a/*"],
    ["a/.x", "a/[.]x"], ["a/.x", "a/?x"], [".x", "*x"], ["x.x", "*x"],
    ["a/.b/c", "a/*/c"], ["a/.b/c", "a/.b/*"],
    // globstar
    ["a/b/c", "**"], ["a/b/c", "**/c"], ["a/b/c", "a/**"],
    ["a/b/c", "a/**/c"], ["a/x/b", "a/**/x/**/b"], ["a/x/y/z/b", "a/**/x/**/b"],
    ["a/b", "a/**/x/**/b"], ["a/x", "a/**/x/**"], ["a/x/y", "a/**/x/**"],
    ["x", "**/x/**"], ["x/", "**/x/**"], ["a/x", "**/x/**"],
    ["a/b/c/d", "**/b/**/d"], ["b/d", "**/b/**/d"], ["a/x", "a/**/x"],
    ["a/b/x", "a/**/x"], ["a/.h/x", "a/**/x"], ["a/b", "**/**/b"],
    ["a/b", "a/**/**/b"], ["x/y", "*/**"], ["x/y", "**/*"],
    ["a", "**"], ["", "**"], [".", "**"], ["..", "**"],
    // path-side optimization(`.`, `..`, 連続スラッシュ)
    ["a/./b", "a/b"], ["a/x/../b", "a/b"], ["./a/b", "a/b"],
    ["a/b/", "a/b"], ["a//b", "a/b"], [".", "."], ["./.", "."],
    ["a/../b", "b"], ["a/b", "a/../b"], ["a/b", "a/./b"],
    // trailing slash
    ["a/", "a"], ["a/b/", "a/*"], ["a/b/", "a/b"], ["a", "a/"],
    ["a/b", "a/b/"], ["x/", "**/x/**"],
    // backslash(pattern 側のみ `/` 正規化)
    ["a/b", "a\\b"], ["ab", "a\\b"], ["a\\b", "ab"], ["a*b", "a\\*b"],
    ["a/b/c", "a\\b/c"], ["a\\b", "a\\\\b"],
    // braces
    ["a", "{a,b}"], ["b", "{a,b}"], ["c", "{a,b}"], ["ab", "a{b,c}"],
    ["ad", "a{b,c}d"], ["ac", "{a,b}{c,d}"], ["a1", "{a,b}{1,2}"],
    ["a2", "{a,b}{1,2}"], ["a3", "{1..5}"], ["a7", "{1..5}"],
    ["03", "{01..5}"], ["a-1", "a{-2..2}"], ["a0", "a{-2..2}"],
    ["ac", "{a..e}"], ["az", "{a..e}"], ["x{a,b}", "x{a,b}"],
    ["{a,b}", "{a,b}"], ["a", "{a}"], ["{a}", "{a}"],
    // extglobs
    ["a", "*(a|b)"], ["aa", "*(a|b)"], ["ab", "*(a|b)"], ["ba", "*(a|b)"],
    ["abc", "*(a|b)"], ["", "*(a|b)"], ["a", "+(a|b)"], ["aa", "+(a)"],
    ["", "+(a)"], ["b", "@(a|b)"], ["ab", "@(a|b)"], ["", "?(a|b)"],
    ["a", "?(a|b)"], ["ab", "?(a|b)"],
    // negative extglob(fillNegs で接尾が alternative へ copyIn される)
    ["c", "!(a|b)"], ["a", "!(a|b)"], ["b", "!(a|b)"], ["", "!(a|b)"],
    ["a", "!(a|b)*"], ["b", "!(a|b)*"], ["c", "!(a|b)*"], ["", "!(a|b)*"],
    ["xby", "x!(a)y"], ["xay", "x!(a)y"], ["xy", "x!(a)y"],
    ["ad", "a!(b|c)"], ["ab", "a!(b|c)"], ["a", "a!(b|c)"],
    ["abc", "a!(b|c)"], ["acx", "a!(b|c)x"],
    // nested extglob(flatten / adopt / usurp)
    ["aa", "+(*(a|b))"], ["ac", "*(+(a|b))"], ["ab", "*(*(a|b))"],
    ["aa", "@(*(a|b))"], ["ac", "?(*(a|b)|c)"], ["abc", "*(a|*(b|c))"],
    ["x", "+(*(a|b)|c)"], ["y", "!(*(a|b))"], ["abc", "!(a|b)*"],
    // `!` が sequence start 扱いになるケース
    [".x", "!(a)*"], ["a", "!(a)*"], ["aa", "!(a)*"],
    // 複合
    ["foo.test.js", "*.test.js"], ["foo.test.js", "*.js"],
    ["src/deep/a.ts", "src/**/*.ts"], ["src/a.ts", "src/**/*.ts"],
    ["src/a.ts", "src/*.ts"], ["a.b/c.d", "*.*/*.*"],
    // unicode / UTF-16
    ["あ", "?"], ["💩", "?"], ["💩", "*"], ["a💩b", "a*b"],
    ["a💩b", "a?b"], ["é", "[é]"], ["𝐀", "?"],
    // misc edge
    ["a", "!a"], ["a", "!!a"], ["a", "#a"], ["#", "#"], ["a|b", "a|b"],
    ["(a)", "(a)"], ["a", "(a"], ["", ""], ["a", ""],
    ["a/**b", "a/**b"], ["a/**/b", "a/**/b"], ["ab", "a**b"],
    ["a/b", "a**/b"], ["a//b", "a//b"],
  ];
  const cases = [];
  for (const [p, g] of GLOB_PAIRS) {
    let expected = null;
    try {
      expected = path.posix.matchesGlob(p, g);
    } catch {
      expected = null; // SyntaxError — Rust 側は bool を返す(上記参照)
    }
    cases.push({ p, g, expected });
  }
  writeFixture("glob.json", { platform: PLATFORM, cases });
}

// ---------- summary ----------

for (const w of written) console.log(`wrote ${w}`);
await rm(TMP_BASE, { recursive: true, force: true });
