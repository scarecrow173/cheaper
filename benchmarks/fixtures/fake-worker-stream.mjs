// `CHEAPER_TEST_FAKE_STREAM` seam 用の RunFixture JSON generator。
//
// TS 版では本 module が CLI process 内へ import され、任意 JS で fake
// stream を組み立てていた。Rust seam(`crates/cheaper-cli/src/fake_stream.rs`)
// は declarative な RunFixture JSON の path を取るため、本 module は
// benchmark harness 側で fixture JSON を生成し、生成 file の path を
// `CHEAPER_TEST_FAKE_STREAM` へ載せる。DSL schema は
// `crates/cheaper-runtime/src/dev/fixture.rs`・`tests/fixtures/*.json` 参照。
//
// 旧 env パラメータとの対応(全て env ではなく generator 引数へ移行):
// - `CHEAPER_BENCH_RESPONSE_PROFILES` → `sessions[]` を profile 1 件
//   ごとに生成する。旧実装は process 全体で共有される stream call
//   queue だったが、DSL は `create_session` 単位の script のため、
//   worker job が N 件の workflow には profile も N 件必要
//   (不足分は TS と同じく "no scripted response" で失敗する)。
// - `CHEAPER_BENCH_RESPONSE_DELAY_MS` → 各 session の `delay_ms` step。
//   旧実装は tool call step には delay を掛けず profile response にのみ
//   掛けていたため、`tool_calls` step より後に置く。
// - `CHEAPER_BENCH_TOOL_CALL_PATH` → session 0 先頭の `tool_calls`
//   step(旧実装は queue 先頭の 1 call のみ = 最初の stream call)。
// - `CHEAPER_BENCH_PROMPT_LOG` / `CHEAPER_BENCH_TIMING_LOG` → DSL では
//   表現不能(旧 module seam が process 内で書いていた観測 log)。
//   呼出し側は materializedPromptBytes / workerStreamConcurrency を
//   null(未計測)として報告する。
//
// usage: 旧 fake は実 prompt から token 数を導出していたが、DSL には
// その hook が無いため固定の `usage_delta` を script する。workflow /
// direct 両 arm が同じ script を replay するため comparison の意味論
// (両 arm 同値 → delta 0)は変わらない。

import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";

// `usage_delta` の scripted 値。実測ではなく deterministic な宣言値。
const PROFILE_USAGE = {
  compact: { input: 1000, output: 30 },
  large: { input: 1000, output: 300 },
};
const DEFAULT_USAGE = { input: 1000, output: 50 };

// 旧 fake が返していた bulk-reader 応答 payload(output schema 適合)。
export function workerResponseText(profile, index) {
  const large = profile === "large";
  return JSON.stringify({
    summary: large ? `seeded compact summary ${index}` : "seeded compact summary",
    findings: large
      ? [
          {
            title: `fixture finding ${index}`,
            detail: "x".repeat(1000),
            confidence: "high",
            locations: [{ path: "README.md", lines: { start: 1, end: 1 } }],
          },
        ]
      : [],
    relevantFiles: [],
    suggestedReads: [],
    openQuestions: [],
  });
}

function sessionSteps(profile, index, options) {
  const steps = [];
  if (index === 0 && options.toolCallPath) {
    steps.push({
      tool_calls: [{ name: "read", input: { path: options.toolCallPath } }],
    });
    steps.push({ turn: { tool_calls: true } });
  }
  if (options.responseDelayMs > 0) steps.push({ delay_ms: options.responseDelayMs });
  steps.push({ usage_delta: PROFILE_USAGE[profile] ?? DEFAULT_USAGE });
  steps.push({ turn: {} });
  steps.push({ finish: { text: workerResponseText(profile, index) } });
  return steps;
}

// `options`: `{ responseProfiles?: string[], responseDelayMs?: number, toolCallPath?: string }`。
export function buildFakeStreamFixture(options = {}) {
  const profiles =
    Array.isArray(options.responseProfiles) && options.responseProfiles.length
      ? options.responseProfiles
      : ["compact"];
  const normalized = {
    responseDelayMs: Number(options.responseDelayMs ?? 0),
    toolCallPath: options.toolCallPath,
  };
  return {
    name: "cheaper-benchmark-fake-stream",
    description: "benchmark harness 生成の scripted session。session 数 = responseProfiles 数",
    sessions: profiles.map((profile, index) => ({
      steps: sessionSteps(profile, index, normalized),
    })),
  };
}

// fixture JSON を `filePath` へ書き、その path を返す。
export async function writeFakeStreamFixture(filePath, options = {}) {
  await mkdir(path.dirname(filePath), { recursive: true });
  const fixture = buildFakeStreamFixture(options);
  await writeFile(filePath, `${JSON.stringify(fixture, null, 2)}\n`, "utf8");
  return filePath;
}
