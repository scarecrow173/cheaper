# Regression snapshot harness

`tests/differential/` の Rust-only 後継。binary replay の golden 比較 —
`crates/cheaper-runtime` の fixture DSL(`RunFixture` JSON)を canonical
input とし、同一 case を Rust binary(`cheaper`)で replay して、normalize
後の stdout/exit code を `expected/<case>.json` の固定 snapshot と比較する。

旧 differential harness は TS binary(`packages/cli/bin/cheaper.js` via
node)を oracle に両実装の観測一致を検証していた。Phase 6 で TS 実装が
撤去されたため、最終 differential green 状態の Rust 側 normalized 出力を
snapshot として固定し、以後の Rust 実装の回帰を検出する。

fixture DSL(`RunFixture`)を使うため `dev-seam` cargo feature が必須
(`required-features` で gate されており、feature 無しでは skip される):

```bash
cargo test -p cheaper-cli --features dev-seam --test regression
```

snapshot の再 capture(意図的な挙動変更時のみ — diff を必ず review する):

```bash
CHEAPER_REGRESSION_CAPTURE=1 cargo test -p cheaper-cli --features dev-seam --test regression
```

## 構成

- `main.rs` — test entry。`cases/*.json` を列挙し 1 test が全 case を
  実行、per-case pass/fail matrix を stderr に出す。不一致があれば
  test を fail させる。
- `harness.rs` — case schema・workspace/config dir の materialize・
  subprocess spawn・env 構成・path scrub root の提供。
- `normalize.rs` — **唯一の** canonical normalization(下記「比較規約」)。
- `cases/*.json` — 1 ファイル 1 case(differential の corpus を継承)。
- `expected/*.json` — case 毎の固定 snapshot(normalize 適用済み)。

## case schema

```jsonc
{
  "name": "…",
  "argv": ["…"],              // argv 系 case(fixture 無し)
  "extraArgs": ["…"],         // run argv 末尾の追加 args
  "stdin": "…" /* または JSON 値 */,
  "files": { "a.txt": "…" /* または {"repeat": [unit, n]} */ },
  "config": { "usageJsonl": […], "files": {…} },
  "fixture": { /* RunFixture JSON(crates/cheaper-runtime/src/dev/fixture.rs) */ },
  "mode": "envelope" /* または "raw"(byte 完全一致 — hook/version/help 系) */,
  "compareStderr": false,      // true で stderr も path scrub 後に比較
                             // (hook deny は stderr が contract — hook case で opt-in)
  "expectDivergence": "…",    // TS oracle 時代の divergence note(履歴情報、
                             // replay 比較では使わない)
  "timeoutMs": 120000          // 子プロセス watchdog
}
```

`fixture` 有りの case は argv を自動構成する:
`run <fixture.worker.name> --input - --model <fixture.explicit_model>` +
`extraArgs`。stdin は `fixture.input` の compact JSON。worker 定義は
`worker_markdown()` で `<ws>/.cheaper/workers/<name>.md` へ生成し、
`policy` 節は `.cheaper/policy.yaml` へ YAML 化する。

## snapshot schema(`expected/<case>.json`)

```jsonc
{
  "code": 0,                  // exit code(timeout 時は null)
  "timedOut": false,
  "stdout": { /* mode=envelope: normalize 済み envelope Value */
              /* mode=raw: scrub 済み stdout 文字列 */ },
  "stderr": "…"               // compareStderr=true の case のみ
}
```

## 比較規約(normalize)

除外(ignore)対象は nondeterminism に限定する:

- 時刻/実行時間: `meta.durationMs`、`meta.execution.*Ms`
- 生成物の ephemeral 値: `tokenizerSource`(cached tokenizer の provenance)
- error prose: `error.message`(code/status/retryable/details は厳密比較)
- 実装依存 path: workspace/config/home/tmp/repo path → token
- JSON object key 順序

厳密比較を維持するもの:

- envelope shape / `ok` / `error.code` / exit code / `result` 全体
- `meta.usage` 6 fields(`costUsd` 含む)、`turns`、`toolCalls`、
  `transientRetries`、`repairs`、`reason` 等の counter/分類
- `worker`/`meta.model`(id/provider/tier/effort/contextWindow/
  effectiveCost/effectiveCostPercentile)
- `meta.execution` の状態遷移 counter と `changedFiles`/`files`/
  `loadedRefs`
- `cause` envelope には再帰適用(message は中でも除外)
