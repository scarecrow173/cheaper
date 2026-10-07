// 一時的な fixture 値計算スクリプト(V-A/V-B/V-C 追加 case の expected/
// state を dump-tool-fixtures.mjs と同じ経路で算出する)。生成後は削除。
import { mkdirSync, realpathSync, writeFileSync } from "node:fs";
import { mkdtemp, mkdir } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const core = await import(new URL("../../../packages/core/dist/index.js", import.meta.url).href);
const rt = await import(
  new URL("../../../packages/tool-runtime/dist/index.js", import.meta.url).href
);
const pi = await import(
  new URL("../../../packages/runtime-pi/dist/index.js", import.meta.url).href
);

function normalizeDetails(v) {
  if (Array.isArray(v)) return v.map(normalizeDetails);
  if (v !== null && typeof v === "object") {
    const o = {};
    for (const [k, val] of Object.entries(v)) {
      if (k === "errno") continue;
      if (k === "errors" && Array.isArray(val)) {
        const paths = [...new Set(val.map((e2) => e2?.instancePath ?? ""))].sort();
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
    return { kind: "escape", input: e.input, reason: e.message.replace(/^workspace escape: /, "") };
  }
  if (e instanceof core.FileTooLargeError) {
    return { kind: "tooLarge", path: e.path, bytes: e.bytes, maxBytes: e.maxBytes };
  }
  if (e instanceof RangeError) return { kind: "range", message: e.message };
  if (e instanceof Error && typeof e.code === "string") return { kind: "io" };
  return { kind: "other", name: e?.name ?? typeof e, message: String(e?.message ?? e) };
}

function canonicalResult(r) {
  const out = { content: r.content };
  if (r.details !== undefined) out.details = r.details;
  return out;
}

const TMP_BASE = await mkdtemp(path.join(os.tmpdir(), "cheaper-tools-mini-"));

async function writeFiles(dir, files) {
  for (const [rel, content] of Object.entries(files ?? {})) {
    const p = path.join(dir, ...rel.split("/"));
    await mkdir(path.dirname(p), { recursive: true });
    writeFileSync(p, String(content), "utf8");
  }
}

function scrub(v, needles) {
  const list = [...needles].sort((a, b) => b[1].length - a[1].length);
  const sub = (s) => {
    for (const [token, p] of list) {
      s = s.split(p).join(token).split(p.replaceAll("\\", "/")).join(token).split(p.replaceAll("\\", "\\\\")).join(token);
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

function projectState(runtime) {
  return {
    budget: {
      toolCalls: runtime.budget.toolCalls,
      rejectedToolCalls: runtime.budget.rejectedToolCalls,
      limitHit: runtime.budget.limitHit,
    },
    meter: { sourceTokens: runtime.meter.sourceTokens },
    journal: runtime.journal.finalize(),
    operations: runtime.operations().map((op) => {
      const out = {
        toolName: op.toolName ?? null,
        state: op.state,
        settlement: op.settlement,
      };
      if (op.resourceClaims !== undefined) out.resourceClaims = op.resourceClaims;
      if (op.cancellation) out.cancellation = { state: op.cancellation.state, reason: op.cancellation.reason };
      if (op.error) out.error = op.error;
      return out;
    }),
  };
}

const CASES = [
  {
    name: "overlong pattern throws through execute",
    files: { "a.txt": "" },
    ops: [{ input: { pattern: "a".repeat(65537) }, tool: "find" }],
    runtime: { names: ["find"] },
  },
  {
    name: "overlong pattern is lazy on empty tree",
    files: {},
    ops: [{ input: { pattern: "a".repeat(65537) }, tool: "find" }],
    runtime: { names: ["find"] },
  },
  {
    name: "grep overlong glob propagates",
    files: { "a.txt": "foo\n" },
    ops: [{ input: { glob: "a".repeat(65537), pattern: "foo" }, tool: "grep" }],
    runtime: { names: ["grep"] },
  },
];

for (const [i, spec] of CASES.entries()) {
  const dir = path.join(TMP_BASE, `c${i}`);
  await mkdir(dir, { recursive: true });
  await writeFiles(dir, spec.files);
  const root = realpathSync(dir);
  const needles = [
    ["<ROOT>", root],
    ["<BASE>", realpathSync(TMP_BASE)],
  ];
  const journal = new rt.MutationJournal();
  const opts = {
    root,
    symlinkPolicy: "deny",
    names: spec.runtime.names,
    maxToolCalls: 100,
    onLimit: () => {},
    journal,
  };
  let runtime;
  const observed = { ...opts, observeAccess: (...args) => runtime.observeAccess(...args) };
  runtime = rt.createToolRuntime({ ...opts, handlers: pi.createPiToolHandlers(observed) });
  const expected = [];
  for (const op of spec.ops) {
    const ctl = new AbortController();
    try {
      const result = await runtime.execute(op.tool, op.input ?? {}, { signal: ctl.signal });
      expected.push({ ok: canonicalResult(result) });
    } catch (e) {
      expected.push({ error: cap(e) });
    }
  }
  const state = projectState(runtime);
  await runtime.dispose();
  console.log(
    `=== ${spec.name} ===\nexpected: ${JSON.stringify(scrub(expected, needles))}\nstate: ${JSON.stringify(scrub(state, needles))}\n`
  );
}
