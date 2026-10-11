# Workflow orchestration benchmark

この harness は seeded fixture と fake Worker stream を使い、実 provider を呼ばずに workflow arm と scripted direct arm を比較する。CLI は cargo build 済みの native binary を subprocess として起動し、`NODE_ENV=test` と `CHEAPER_TEST_FAKE_STREAM` を設定する。fake-stream seam は `dev-seam` cargo feature で gate されているため(ADR-034DyGyITd0YaATg8fkld7)、benchmark 対象 binary は feature 有効で build する必要がある。

実測済みの 7 scenario / 17 arm / warm-up 2 回 + 測定 15 回の集約 artifact は [結果 JSON](./results/e45d35b7-2fa6-416b-946a-70ca844b9c80.json)(TS CLI 時代の記録 — binary 表記等が現行と異なる)。対応する解釈は [Workflow の使い分け](../docs/guides/workflow-usage.md) を参照。

## 前提

spawn 対象は `target/release/cheaper`(win32 は `cheaper.exe`)で、無ければ `target/debug/cheaper` へ fallback する。`dev-seam` feature を有効化して build すること(無いと fake-stream seam が compile 対象外で fixture が読まれない)。

```powershell
mise exec -- cargo build --release -p cheaper-cli --features dev-seam
# 時間優先なら debug profile: mise exec -- cargo build -p cheaper-cli --features dev-seam
# あるいは workspace 全体: mise exec -- cargo build --workspace --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam
```

## 実行

```powershell
mise exec -- node benchmarks/run.mjs
```

既定値は arm ごとに warm-up 2 回、測定 15 回。シナリオを限定した smoke は次のように実行する。

```powershell
mise exec -- node benchmarks/run.mjs --scenario single-simple-tool-call --warmups 0 --repetitions 1
```

`--scenario` には `benchmarks/scenarios/scenarios.json` の ID を渡す。`--output <path>` は既存ファイルを上書きせず新規作成する。既定の保存先は UUID 名の `benchmarks/results/<run-id>.json`。各結果には source revision、fixture revision、seed、OS、architecture、Node、CPU 数、arm ごとの観測値、median/p95、比較差分が含まれる。

## fake stream seam(Rust 版)

`CHEAPER_TEST_FAKE_STREAM` は Rust 側 seam(`crates/cheaper-cli/src/fake_stream.rs`、`dev-seam` cargo feature で gate)の RunFixture JSON path を指す。`benchmarks/fixtures/fake-worker-stream.mjs` は CLI process 内で import される module ではなく、harness 側で fixture JSON(`sessions[].steps` — `turn`/`tool_calls`/`delay_ms`/`usage_delta`/`finish` 等の DSL、`crates/cheaper-runtime/tests/fixtures/*.json` 参照)を spawn ごとに生成する generator になった。

旧 module seam が env(`CHEAPER_BENCH_*`)で受けていたパラメータの対応:

- `CHEAPER_BENCH_RESPONSE_PROFILES` → `sessions[]` を profile 1 件ごとに生成(worker job が N 件の workflow には N 件必要)
- `CHEAPER_BENCH_RESPONSE_DELAY_MS` → 各 session の `delay_ms` step
- `CHEAPER_BENCH_TOOL_CALL_PATH` → session 0 先頭の `tool_calls` step
- `CHEAPER_BENCH_PROMPT_LOG` / `CHEAPER_BENCH_TIMING_LOG` → fixture DSL では表現不能なため廃止(下記の未計測 dimension 参照)
- Pi runtime 向けの offline env → Rust binary は参照しないため除去

## 計測と会計

- workflow の Worker usage は成功 envelope の `jobs[].meta` から一度だけ集計する。workflow run は `usage.jsonl` に記録されない。
- direct Worker sequence は各 direct CLI envelope から計測し、workflow aggregate と加算しない。JSONL の件数は記録しているが、aggregate の token source として再加算しない。
- usage は fixture の scripted `usage_delta` で宣言した deterministic な固定値。旧 fake は実 prompt から導出していたが、DSL には導出 hook が無い。両 arm が同じ script を replay するため comparison 意味論は変わらない。
- retained bytes は workflow の返却 result と artifact metadata、direct arm では retained inputs と child result の byte 数。
- `materializedPromptBytes` と `workerStreamConcurrency` は Rust seam に観測 hook が無いため `null`(未計測)。`maxConcurrency` は envelope の `meta.execution.maxConcurrency`(tool operation 側)のみを反映する。
- compact interface bytes は CLI envelope 全体ではなく `result` object を JSON serialize したサイズ。可変な `meta.durationMs` は除く。
- 親 agent の token usage/turns telemetry は接続されていないため `parentUsage.status=unavailable`。direct CLI invocation 数は proxy としてのみ記録し、親 turn の実測や節約として扱わない。
- 結果の `comparisons` は左 arm − 右 arm。値が測定できないか反復間で変化した dimension は `null` のままにする。

シナリオ追加時は `scenarios.json` と `lib/scenarios.mjs` の必須 coverage を合わせ、schema、bindings、direct arm が同じ作業を表すことを検証する。`benchmarks/test/` の unit test は `mise run check` に配線済み、benchmark 実測(`run.mjs`)は手動実行。
