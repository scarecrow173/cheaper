// Differential golden harness — TS 側 fake-stream adapter(T8)。
//
// `CHEAPER_TEST_FAKE_STREAM` の module-form seam として読み込まれ、
// `CHEAPER_DIFF_FIXTURE` が指す RunFixture JSON(Rust `fixture.rs` DSL)
// を pi の per-call stream 列へ翻訳する。fixture schema・step 意味論の
// canonical 定義は Rust 側 `crates/cheaper-runtime/src/fixture.rs` —
// ここでは「外部から観測可能な挙動が一致する replay」だけを担う。
//
// step → pi call 変換規則(docs/impl/ir/phase-4-port-cli-and-workflow.md
// T8 節と同一ルール):
// - pi の 1 model call = 1 assistant message = 1 turn_end。call 境界は
//   `tool_calls`(toolUse 終了)と終端 step(finish/error/panic/hang*/
//   abort_response)のみ。`turn`/`usage_delta`/`emit`/`delay_ms` は現在
//   call への積算で、call 分割には使わない。
// - `turn` は Rust fake の record_turn marker — adapter 側で何もしない。
//   pi は errored/aborted な call でも turn_end を emit するため、終端
//   step を持つ call は対応する `turn` step を corpus 側が宣言する
//   (fixture inventory の記述規約)。
// - `emit` は emitted_in_call を立てる(Rust と同じ規則: turn/tool_calls
//   で reset)。emit 済み call の `error` は post-item = retry しない。
// - `error` の retry 判定は Rust `classify_scripted_error` と同一表
//   (429→rate-limit / >=500→server / 他 status→permanent / status 無し
//   は retryable flag→transport)。pi の `isRetryableAssistantError` が
//   errorMessage pattern で判定するため、transient 系は status 数字や
//   "socket hang up" を埋め、terminal 系は NON_RETRYABLE 語("quota
//   exceeded")で必ず非 retry 化する。
// - `abort.after_ms` は `process.emit("SIGINT")` で CLI の signal 経路を
//   踏む(Rust 側の fixture abort → CLI signal relay と同じ入口)。
// - usage は `usage_delta` の宣言値を message.usage へそのまま載せる
//   (pi faux provider の `withUsageEstimate` heuristic は使わない —
//   token field の厳密比較が spec の厳密比較対象のため)。

import { readFileSync } from "node:fs";

// workspace pnpm link の実体を直接 file URL で import する(adapter は
// package 外に置くため bare specifier の解決根が無い)。
const { createFakePi } = await import(new URL(
  "../../../../packages/cli/node_modules/@cheaper/runtime-pi/dist/testsupport.js",
  import.meta.url,
).href);
const { createAssistantMessageEventStream } = await import(new URL(
  "../../../../packages/runtime-pi/node_modules/@earendil-works/pi-ai/dist/index.js",
  import.meta.url,
).href);

const fixture = JSON.parse(readFileSync(process.env.CHEAPER_DIFF_FIXTURE, "utf8"));

// ---------------------------------------------------------------------------
// sessions[].steps → call program
// ---------------------------------------------------------------------------

const ZERO_USAGE = () => ({
  input: 0,
  output: 0,
  cacheRead: 0,
  cacheWrite: 0,
  totalTokens: undefined,
});

function newCall() {
  return {
    usage: ZERO_USAGE(),
    texts: [],
    thinkings: [],
    toolCalls: [],
    delays: [],
    emitted: false,
    terminal: null,
  };
}

/**
 * session steps を call 列へ畳む。`tool_calls` step はその位置で call を
 * toolUse 終了させる(後続 `turn{tool_calls:true}` は counter marker で
 * 無視)。未知 step でも止まらず次へ進む(未対応 primitive は corpus 側が
 * 使わない前提 — steps の消費順序だけは Rust と一致させる)。
 */
function buildCalls(steps) {
  const calls = [];
  let cur = newCall();
  const flush = (terminal) => {
    cur.terminal = terminal;
    calls.push(cur);
    cur = newCall();
  };
  for (const step of steps ?? []) {
    const key = typeof step === "string" ? step : Object.keys(step)[0];
    const v = typeof step === "string" ? true : step[key];
    switch (key) {
      case "emit":
        if (v.text_delta !== undefined) cur.texts.push(String(v.text_delta));
        if (v.reasoning_delta !== undefined) cur.thinkings.push(String(v.reasoning_delta));
        cur.emitted = true;
        break;
      case "delay_ms":
        cur.delays.push(Number(v));
        break;
      case "usage_delta": {
        const u = cur.usage;
        u.input += v.input ?? 0;
        u.output += v.output ?? 0;
        u.cacheRead += v.cacheRead ?? 0;
        u.cacheWrite += v.cacheWrite ?? 0;
        if (v.total !== undefined) u.totalTokens = (u.totalTokens ?? 0) + v.total;
        break;
      }
      case "tool_calls":
        for (const c of v) {
          cur.toolCalls.push({
            type: "toolCall",
            id: `call-${calls.length}-${cur.toolCalls.length}`,
            name: c.name,
            arguments: c.input ?? {},
          });
        }
        flush({ kind: "toolUse" });
        break;
      case "turn":
        // call 境界 marker: 次の `error` を pre-item 失敗として扱う
        // (Rust `emitted_in_call` reset と同じ位置)。
        cur.emitted = false;
        break;
      case "error":
        flush({ kind: "error", error: v ?? {}, emitted: cur.emitted });
        break;
      case "panic":
        flush({ kind: "panic", message: typeof v === "string" ? v : "panic" });
        break;
      case "finish":
        flush({ kind: "finish", text: typeof v === "string" ? v : (v?.text ?? "") });
        break;
      case "hang_until_abort":
        flush({ kind: "hangUntilAbort" });
        break;
      case "hang":
        flush({ kind: "hang" });
        break;
      case "abort_response":
        flush({ kind: "abortResponse" });
        break;
      default:
        break;
    }
  }
  // 残存 payload(terminal 無し)は未到達に近いが、defensive に積む。
  if (cur.texts.length || cur.thinkings.length || cur.toolCalls.length || cur.delays.length) {
    calls.push(cur);
  }
  return calls;
}

// Rust `classify_scripted_error`(fake_provider.rs)と同一表。
function classifyError(error) {
  if (error?.status === 429) return "TransientRateLimit";
  if (typeof error?.status === "number" && error.status >= 500) return "TransientServer";
  if (error?.status !== undefined && error?.status !== null && error?.status !== undefined) {
    return "Permanent";
  }
  return error?.retryable === true ? "TransientTransport" : "Permanent";
}
function isTransient(cls) {
  return cls !== "Permanent";
}

// pi `isRetryableAssistantError` の pattern 写像で retry 可否を制御する
// errorMessage encoding。message 本文は比較対象外(error.message 本文除外)
// なので、分類クラスだけを決定的に encode する。
function encodeErrorMessage(error, retry) {
  const msg = error.message ?? "fixture provider error";
  if (retry) {
    if (error.status !== undefined && error.status !== null) {
      return `HTTP ${error.status} ${msg}`;
    }
    return `socket hang up: ${msg}`;
  }
  return `quota exceeded fixture-error: ${msg}`;
}

function makeMessage(model, call, content, stopReason, errorMessage) {
  const u = call.usage;
  // TS `meta.usage.costUsd` は message.usage.cost.total の合算 —
  // cheaper 側 pricing 層の再計算は無い。fake model の cost($/1M)から
  // Rust `default_cost` と同じ式で算出し adapter 内で埋める。
  const c = model.cost ?? { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 };
  const costInput = (u.input * c.input) / 1e6;
  const costOutput = (u.output * c.output) / 1e6;
  const costCacheRead = (u.cacheRead * c.cacheRead) / 1e6;
  const costCacheWrite = (u.cacheWrite * c.cacheWrite) / 1e6;
  // Rust `usage_cost_usd`(meta.rs)は合算 token × rate を 1 回 /1e6
  // する。per-message 合算と bit 一致させるため total も単一式の除算
  // (rate が二進分数の corpus では積和は exact)。
  return {
    role: "assistant",
    api: model.api,
    provider: model.provider,
    model: model.id,
    usage: {
      input: u.input,
      output: u.output,
      cacheRead: u.cacheRead,
      cacheWrite: u.cacheWrite,
      totalTokens: u.totalTokens ?? u.input + u.output + u.cacheRead + u.cacheWrite,
      cost: {
        input: costInput,
        output: costOutput,
        cacheRead: costCacheRead,
        cacheWrite: costCacheWrite,
        total: (u.input * c.input + u.output * c.output + u.cacheRead * c.cacheRead + u.cacheWrite * c.cacheWrite) / 1e6,
      },
    },
    stopReason,
    ...(errorMessage !== undefined ? { errorMessage } : {}),
    content,
    timestamp: 1700000000000,
  };
}

export default async function fakeStreamFactory({ modelRuntime }) {
  const def = fixture.model ?? {};
  const cost = def.cost ?? { input: 1.0, output: 2.0, cacheRead: 0.5, cacheWrite: 0.5 };
  const fake = createFakePi({
    models: [{
      id: def.id ?? "fake-model",
      // Rust `default_model` は name="Fake Model"、id override 時は
      // name=id(fixture.rs `model_definition`)— 同じ写像。
      name: def.id ?? "Fake Model",
      cost: {
        input: cost.input,
        output: cost.output,
        cacheRead: cost.cacheRead,
        cacheWrite: cost.cacheWrite,
      },
      // Rust `CheaperFakePlugin::default_model` と揃える(faux default
      // は maxTokens=16384 で drift するため明示)。
      contextWindow: 128000,
      maxTokens: 8192,
    }],
  });
  modelRuntime.registerNativeProvider(fake.provider);
  await modelRuntime.refresh({ allowNetwork: false });

  // fixture `abort` 節 — Rust 側は CLI の集約 AbortController を after_ms
  // 後に fire する。TS では `process.emit("SIGINT")` が同一入口(cli.ts
  // の `process.on("SIGINT")` → ac.abort → runAbort reason "signal")。
  if (fixture.abort) {
    setTimeout(() => process.emit("SIGINT"), Number(fixture.abort.after_ms));
  }

  const callQueue = (fixture.sessions ?? []).flatMap((s) => buildCalls(s.steps));
  const model = fake.model;

  return function streamFn(_model, _context, options) {
    const call = callQueue.shift() ?? (() => {
      const c = newCall();
      c.terminal = {
        kind: "error",
        error: { message: "differential adapter: call queue exhausted" },
        emitted: false,
      };
      return c;
    })();
    const signal = options?.signal;
    const stream = createAssistantMessageEventStream();

    // `panic` step — Rust fake は stream 内部 panic → JoinError →
    // INTERNAL_ERROR(exit 70)。TS 側に stream 内 panic の等価物は無い —
    // 最も近い写像は streamFn 自体の同期 throw(prompt() reject →
    // PROVIDER_FAILURE)。error 分類の差異は corpus が divergence として
    // pin する。
    if (call.terminal?.kind === "panic") {
      throw new Error(`fake provider panic: ${call.terminal.message}`);
    }

    queueMicrotask(async () => {
      const emitAborted = (content) => {
        const aborted = makeMessage(model, call, content, "aborted", "Request was aborted");
        stream.push({ type: "error", reason: "aborted", error: aborted });
        stream.end(aborted);
      };
      try {
        const t = call.terminal ?? { kind: "finish", text: "" };

        // delay 群を逐次消化 — abort で abort error emit(abort 後の残
        // delay/内容は消化しない — Rust の delay abort 中断と同じ位置)。
        for (const ms of call.delays) {
          if (signal?.aborted) {
            emitAborted([]);
            return;
          }
          await new Promise((resolve) => {
            const timer = setTimeout(resolve, ms);
            signal?.addEventListener("abort", () => {
              clearTimeout(timer);
              resolve();
            }, { once: true });
          });
          if (signal?.aborted) {
            emitAborted([]);
            return;
          }
        }

        const content = [];
        for (const th of call.thinkings) content.push({ type: "thinking", thinking: th });
        for (const x of call.texts) content.push({ type: "text", text: x });
        if (t.kind === "finish") content.push({ type: "text", text: t.text });
        for (const c of call.toolCalls) content.push(c);

        if (t.kind === "hang") {
          // signal を観測しない永久 pending — grace 経路(Rust `hang`)。
          return;
        }
        if (t.kind === "abortResponse") {
          // provider 側 cancel 応答(Rust `abort_response` → PromptCancelled
          // → ABORTED 写像相当)— signal 非依存で即 aborted message。
          emitAborted(content);
          return;
        }
        if (t.kind === "hangUntilAbort") {
          if (signal?.aborted) {
            emitAborted(content);
            return;
          }
          signal?.addEventListener("abort", () => emitAborted(content), { once: true });
          return;
        }
        if (t.kind === "panic") {
          // Rust fake `panic` は stream 内部 panic → JoinError →
          // INTERNAL_ERROR。TS 側は streamFn throw が最も近い写像先
          // (prompt() reject → PROVIDER_FAILURE — 既知 divergence として
          // corpus が pin する)。
          throw new Error(`fake provider panic: ${t.message}`);
        }
        if (t.kind === "error") {
          const cls = classifyError(t.error);
          const retry = !t.emitted && isTransient(cls);
          const message = makeMessage(model, call, content, "error", encodeErrorMessage(t.error, retry));
          stream.push({ type: "error", reason: "error", error: message });
          stream.end(message);
          return;
        }

        const stopReason = t.kind === "toolUse" || call.toolCalls.length > 0 ? "toolUse" : "stop";
        const message = makeMessage(model, call, content, stopReason);
        const partial = { ...message, content: [], stopReason: "pending" };
        stream.push({ type: "start", partial: { ...partial } });
        for (let idx = 0; idx < content.length; idx++) {
          const block = content[idx];
          if (signal?.aborted) {
            emitAborted(content.slice(0, idx));
            return;
          }
          if (block.type === "text") {
            partial.content = [...partial.content, { type: "text", text: "" }];
            stream.push({ type: "text_start", contentIndex: idx, partial: { ...partial } });
            partial.content[idx].text += block.text;
            stream.push({ type: "text_delta", contentIndex: idx, delta: block.text, partial: { ...partial } });
            stream.push({ type: "text_end", contentIndex: idx, content: block.text, partial: { ...partial } });
          } else if (block.type === "thinking") {
            partial.content = [...partial.content, { type: "thinking", thinking: "" }];
            stream.push({ type: "thinking_start", contentIndex: idx, partial: { ...partial } });
            partial.content[idx].thinking += block.thinking;
            stream.push({ type: "thinking_delta", contentIndex: idx, delta: block.thinking, partial: { ...partial } });
            stream.push({ type: "thinking_end", contentIndex: idx, content: block.thinking, partial: { ...partial } });
          } else {
            partial.content = [
              ...partial.content,
              { type: "toolCall", id: block.id, name: block.name, arguments: {} },
            ];
            stream.push({ type: "toolcall_start", contentIndex: idx, partial: { ...partial } });
            stream.push({ type: "toolcall_delta", contentIndex: idx, delta: JSON.stringify(block.arguments), partial: { ...partial } });
            partial.content[idx].arguments = block.arguments;
            stream.push({ type: "toolcall_end", contentIndex: idx, toolCall: block, partial: { ...partial } });
          }
        }
        stream.push({ type: "done", reason: stopReason, message });
        stream.end(message);
      } catch (e) {
        // adapter 内部 bug / panic 経路 — retry されない error message で
        // 終端化する(envelope 観測面を保つため stream を正常終了させる)。
        const message = makeMessage(model, call, [], "error", `quota exceeded adapter-error: ${e instanceof Error ? e.message : String(e)}`);
        stream.push({ type: "error", reason: "error", error: message });
        stream.end(message);
      }
    });
    return stream;
  };
}
