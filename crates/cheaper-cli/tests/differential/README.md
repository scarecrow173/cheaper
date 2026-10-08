# Differential golden harness(T8)

binary-to-binary の golden 比較 — `crates/cheaper-runtime` の fixture DSL
(`RunFixture` JSON)を canonical input とし、同一 case を Rust binary
(`cheaper`)と canonical TS entry(`packages/cli/bin/cheaper.js` via node)
で実行して、normalize 後の stdout/exit code を比較する。

```bash
cargo test -p cheaper-cli --test differential
```

前提: `packages/cli/dist` が `pnpm build` 済みで、node binary が解決できる
こと(`CHEAPER_DIFF_NODE` → `mise where node` → PATH の非 shim node の順)。
node/dist が無い場合は rust-only で skip する(fail しない)。

## 構成

- `main.rs` — test entry。`cases/*.json` を列挙し 1 test が全 case を実行、
  per-case pass/divergence/fail の matrix を stderr に出す。unpinned diff が
  あれば test 自体を fail させる。
- `harness.rs` — case schema・workspace/config dir の materialize・両 binary
  の subprocess spawn・env 構成・path scrub root の提供。
- `normalize.rs` — **唯一の** canonical normalization(下記「比較規約」)。
- `adapter.mjs` — TS 側 `CHEAPER_TEST_FAKE_STREAM` の module-form seam。
  `CHEAPER_DIFF_FIXTURE` の RunFixture JSON を読み、pi の per-call stream
  列へ翻訳する(fixture step 意味論の mirror — 詳細はファイル先頭の
  コメント)。
- `cases/*.json` — 1 ファイル 1 case。

## case schema

```jsonc
{
  "name": "…",
  "argv": ["…"],              // argv 系 case(fixture 無し)
  "extraArgs": ["…"],         // run argv 末尾の追加 args
  "stdin": "…" /* または JSON 値 */,
  "files": { "a.txt": "…" /* または {"repeat": [unit, n]} */ },
  "config": { "usageJsonl": […], "files": {…} },
  "fixture": { /* RunFixture JSON(crates/cheaper-runtime/src/fixture.rs) */ },
  "mode": "envelope" /* または "raw"(byte 完全一致 — hook/version 系) */,
  "compareStderr": false,      // true で stderr も path scrub 後 byte 比較
                               // (hook deny は stderr が contract の一部 — hook case で opt-in)
  "expectDivergence": "既知 divergence の pin note(差分が出た場合のみ有効)",
  "timeoutMs": 120000          // 子プロセス watchdog
}
```

`fixture` 有りの case は argv を自動構成する:
`run <fixture.worker.name> --input - --model <fixture.explicit_model>` +
`extraArgs`。stdin は `fixture.input` の compact JSON。worker 定義は
`worker_markdown()` で `<ws>/.cheaper/workers/<name>.md` へ生成し、
`policy` 節は `.cheaper/policy.yaml` へ YAML 化して両 binary が同一 policy
file を読む(Rust 側は加えて fixture `policy.apply_to` が同じ値を重ねる)。

## step → pi call 変換規約(adapter)

pi の 1 model call = 1 assistant message = 1 `turn_end`。corpus の記述規約:

- call 境界は `tool_calls` と終端 step(`finish`/`error`/`panic`/
  `hang*`/`abort_response`)のみ。
- **call 毎に対応する `turn` step を 1 つ宣言する**(tool_calls 終端の
  call は `{"turn":{"tool_calls":true}}`、それ以外は `{"turn":{}}`)。
  Rust fake の turns は `turn` step 計上のみで、errored/aborted message
  にも pi は `turn_end` を立てるため、retry される error call・終端
  error call・abort される call にも `turn` が必要。
- `turn` step は adapter 側では emitted flag の reset(call 境界 marker)
  としてのみ働き、`emitted` 済み call の `error` は post-item =
  retry しない(Rust `emitted_in_call` と同じ位置)。
- usage は `usage_delta` の宣言値を message.usage にそのまま載せる
  (faux provider の token heuristic は使わない)。`cost.total` は fake
  model cost($/1M)× token を単一式 `/1e6` で算出して埋める(TS の
  `costUsd` は message.usage.cost.total の合算で、Rust の
  `usage_cost_usd`(meta.rs)は合算 token × rate の 1 回除算 — corpus の
  rate(1.0/2.0/0.5)では積和が exact で bit 一致する)。
- `error` の retry 判定は pi `isRetryableAssistantError` の message
  pattern 経由のため、adapter が errorMessage を決定的に encode する
  (transient: `HTTP <status>`/`socket hang up`、terminal: `quota
  exceeded` prefix)。fixture error message に non-retryable/retryable の
  語を混ぜないこと。
- `abort.after_ms` は `process.emit("SIGINT")` で CLI の signal 経路を
  踏む。abort timing は session 構築(~数百 ms)をまたぐ必要があるため
  `after_ms >= 500` を目安にする(早すぎると stream 未開始の race で
  meta null/非 null が揺れる — 実測で確認済み)。
- `abort_response` は provider 側 cancel 応答として signal 非依存で即
  aborted message を emit する。
- `hang` は signal を観測しない永久 pending(TS 側は abortGraceMs 既定
  5000ms が掛かるため case 実行が ~5s 遅い)。
- `panic` は TS 側に stream 内 panic の等価物が無いため streamFn の
  同期 throw に写像する — `INTERNAL_ERROR` vs `PROVIDER_FAILURE` の
  divergence として case が pin する。

## 比較規約(normalization)

`normalize.rs::normalize` が両側 stdout の JSON envelope に適用される
唯一の canonical 実装(`docs/test-specs/rust-migration-differential-
golden.md` と `rust-migration-test-spec.md` の「JSON 比較規約」に対応)。
`mode: "raw"` の case(hook/version 系)は byte 完全一致 + path scrub のみ。
`compareStderr: true` の case(hook 系)は stderr も同規約で byte 比較する
— deny guidance・deny JSON 内の token 数(tiktoken-rs vs js-tiktoken の
一致が前提)まで厳密に比較する T9 の hook byte-parity matrix。

除外(ignore)対象 — 厳密比較対象として spec に無い nondeterminism のみ:

- `meta.durationMs`、`meta.execution.*Ms`(`*Ms` suffix の timing field)
- `tokenizer` object(`implementation`/`version`/`tokenizerSource` —
  tiktoken-rs vs js-tiktoken の実装識別子。`tokens`/`bytes` 数値は厳密比較)
- error prose: `message`/`errorMessage`(error subtree 内再帰)、
  string 形の `cause`(OS IO error 本文)
- `details.searched`(builtin dir 展開先が実装依存の path 列)
- 実装依存 path: workspace/config/home/tmp/repo 内 path → token
- JSON object key 順序

厳密比較を維持するもの:

- envelope shape / `ok` / `error.code`/`retryable`/exit code /
  `details.path`/`details.errors[].instancePath`(sort/dedup 後)
- `result` 全体
- `meta.usage` 6 fields(`costUsd` 含む)、`turns`/`toolCalls`/
  `rejectedToolCalls`/`repairs`/`transientRetries`
- `meta.model`/`worker`/`meta.execution` の counter/`changedFiles`/
  `savings`
- `error.cause` object の `code`(PARTIAL_MUTATION wrap 等)

stderr は既定で実装依存の診断出力として比較対象外(exit code/stdout
のみ)。hook 系 case のみ `compareStderr` で opt-in 比較する(上記)。

## fixture inventory(55 cases)

run 系(envelope 比較、fake-stream 経路):

- 成功系: `run-success-basic`(usage+costUsd+fence 抽出)、
  `run-success-two-turns`(tool call → 2 turn)、
  `run-success-write-tool`(write → changedFiles)、
  `run-usage-accumulation`(3 call の usage 合算)、
  `run-zero-usage`(usage 0 埋め)
- repair: `run-repair-success`(2 repair 後成功)、
  `run-repair-exhausted`(INVALID_OUTPUT + usage-bearing meta)
- 制限/timeout: `run-timeout-delay`(TIMEOUT)、
  `run-max-turns`・`run-max-tool-calls`(pinned divergence)
- abort: `run-signal-abort`、`run-partial-mutation`(PARTIAL_MUTATION
  wrap)、`run-hang-grace`(grace 待ち切り)、`run-abort-response`
  (provider cancel)、`run-abort-during-retry`(backoff 中 abort)
- provider error: `run-error-429-retry`・`run-error-retryable-transient`
  (transient retry → 成功)、`run-error-429-exhausted`・
  `run-error-5xx-exhausted`・`run-error-permanent`・
  `run-emit-then-error`(post-item error は retry しない —
  `PROVIDER_FAILURE.details` は `{errorMessage}` で一致済み)、
  `run-panic`(pinned)
- model/auth: `run-unknown-model`、`run-auth-env-template`

argv 系(fixture 無し):

- `cli-empty-argv`・`cli-help-command`・`cli-unknown-command`
  (INVALID_REQUEST 経路)、`cli-version`(raw byte 比較)
- `cli-run-worker-not-found`・`cli-run-bad-input-json`・
  `cli-run-missing-input-file`・`cli-run-worker-file`・`cli-bulk-read`
  (worker/input/model 解決 error 経路 — fake stream 無しで両 binary が
  同一の解決失敗 envelope を出すことを確認)
- `cli-count-tokens`・`cli-count-tokens-missing`(token 数値は厳密比較)
- `cli-usage-empty`・`cli-usage-seeded`(usage.jsonl 集計)
- `cli-models`(catalog 列挙)、`cli-auth-status`(oauth 宣言 provider の
  `methods`/`subscription` は declaration ベースで一致済み — oauth
  login 自体は非 github-copilot provider で flow 未実装のため deferred
  (ISS-23))
- hook(raw byte 比較、全 case `compareStderr` で stderr も厳密比較 —
  `packages/cli/test/hook/hook-contract.test.ts` の target×event
  matrix を binary レベルで再現した T9 の parity matrix):
  - command path: `cli-hook-no-args`・`cli-hook-cursor-fallback`
    (event 無し → schema-valid allow JSON)・`cli-hook-malformed`
    (未知 event → silent fail-open)・`cli-hook-corrupt-policy`
    (user policy.yaml 破損 → fail-open allow)
  - windsurf: `cli-hook-windsurf-allow`・`cli-hook-windsurf-deny`
    (pre-read-code → stderr guidance + exit 2)・
    `cli-hook-windsurf-shell-deny`(pre-run-command `cat big.txt`)・
    `cli-hook-windsurf-shell-cd-allow`(cwd 遷移後 read → fail-open)
  - copilot: `cli-hook-copilot-deny`(`permissionDecision` deny JSON)・
    `cli-hook-copilot-allow`(silent)・`cli-hook-copilot-foreign-allow`
    (foreign shape → dual-schema allow)
  - pretooluse: `cli-hook-pretooluse-deny`(`hookSpecificOutput`
    deny JSON)・`cli-hook-pretooluse-allow`(silent)
  - cursor: `cli-hook-cursor-allow`・`cli-hook-cursor-deny`
    (before-read-file → permission deny JSON)・
    `cli-hook-cursor-shell-deny`(before-shell-execution)

  残りの contract cell(`--target` on non-hook command の拒否・
  `cheaper.mjs` bundled entry・symlinked cwd・FIFO・usage.jsonl
  非記録・panic → allow/exit 0)は binary 間差の出ない経路または
  OS/spawn 依存で、case schema では表現できない —
  `tests/hook.rs`・`tests/args.rs` が in-process で担保する
  (差分が出得る全経路は上記 case で網羅済み)。

## pinned divergences(実測で確認・解消済みか監視対象)

`expectDivergence` を持つ case は normalized diff が非空のときだけ通る
(解消された場合は "divergence resolved" として report する)。
現行 matrix: `cases=55 pass=52 divergence=3 fail=0 rust-only=0`
(follow-up fix 適用後)。pin は以下の 2 系統:

1. **panic 写像**(`run-panic`): Rust INTERNAL_ERROR(70)+runMeta 無し vs
   TS PROVIDER_FAILURE(5)+retryable=true+runMeta 付き。fixture の panic
   は Rust 側では実際の panic であり、TS 側に stream 内 panic の等価
   input が存在しない seam 限界。
2. **in-flight aborted call の turn 計上**(`run-max-turns`/
   `run-max-tool-calls`): pi agent loop は abort 済みでも次の streamFn
   call を発行し aborted message に `turn_end` を立てる(+1)。Rust fake
   DSL は abort 後の `turn` step を消化できず表現不能。`run-max-tool-calls`
   はさらに reject tool の operation 状態(cancelled vs completed)/
   `maxConcurrency`/`savings.sourceTokens` の計上位置が異なる。

解消済み: `PROVIDER_FAILURE.details` shape(TS `{errorMessage}` のみに
揃えた — `run-error-*` 4 case)と `cli-auth-status` の oauth
`methods`/`subscription`(declaration ベース化)は binary 一致を確認して
pin 解除済み。
