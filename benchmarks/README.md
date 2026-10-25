# Workflow orchestration benchmark

この harness は seeded fixture と fake Worker stream を使い、実 provider を
呼ばずに workflow arm と scripted direct arm・Route arm・失敗 arm を
比較する。harness 本体は Rust crate `crates/cheaper-bench`
(publish=false の dev-only workspace member)で、計測対象の `cheaper`
binary を subprocess として spawn する。設計の正本は
[`docs/designs/benchmark-harness-redesign-issue-183.md`](../docs/designs/benchmark-harness-redesign-issue-183.md)
(以下 design §N 参照)。

旧 Node harness(`benchmarks/run.mjs`・`lib/*.mjs`・
`fixtures/fake-worker-stream.mjs`・`test/*.mjs`)は **deprecated** —
manifest v2 + Rust harness への移行(issue #183)で check pipeline
から外れており、file 自体は T6 で削除予定の未参照 file として残る。
v1 時代の計測 artifact
([`results/e45d35b7-2fa6-416b-946a-70ca844b9c80.json`](./results/e45d35b7-2fa6-416b-946a-70ca844b9c80.json)、
TS CLI 時代の記録)は `benchmarks/results/` に不変で残るが、
v2 harness は `schemaVersion:1` の file を load しない
(「baseline 同等性」節参照)。

## 前提(2 段: build → run)

計測対象 binary は `dev-seam` cargo feature 有効で build する
(fake-stream seam `crates/cheaper-cli/src/fake_stream.rs` は `dev-seam`
で gate — ADR-034DyGyITd0YaATg8fkld7)。harness は計測開始前に
preflight probe(最小 workspace + bulk-reader fixture session +
`CHEAPER_TEST_FAKE_STREAM` 下の `cheaper run bulk-reader`)で dev-seam
有無を検証し、seam 無し binary では fail-fast する。

```powershell
# 1. 計測対象 binary(dev-seam 必須)
mise exec -- cargo build --release -p cheaper-cli --features dev-seam
# 時間優先なら debug profile: mise exec -- cargo build -p cheaper-cli --features dev-seam

# 2. harness 実行
mise exec -- cargo run --release -p cheaper-bench -- `
  --scenario <id> [--scenario <id>…] `
  --warmups 2 --repetitions 15 `
  [--output <path>] [--binary <path>] [--pricing <file>]
```

既定値は arm ごとに warm-up 2 回・測定 15 回、`--scenario` 省略時は
manifest 全 scenario を実行する。`--binary` 省略時は
`target/release/cheaper`(win32 は `cheaper.exe`)→ `target/debug/` の
順に解決する。smoke 実行の例:

```powershell
mise exec -- cargo run --release -p cheaper-bench -- `
  --scenario single-simple-tool-call --warmups 0 --repetitions 1
```

`--output` 指定・既定の `benchmarks/results/<runId>.json`(UUID 名)の
いずれも、既存 file を上書きしない非上書き保存(`create_new`)で、
results dir は append-only・commit 対象。`--pricing <file>` を与えた
場合のみ estimated-cost entry を emit する(claim 境界の節を参照)。

## scenario manifest v2(`benchmarks/scenarios/*.json`)

- `schemaVersion: 2`・`fixtureRevision: "seeded-v2"`。required 12
  scenario(継続 7 件 + Route 系 4 件 + 失敗 matrix 1 件)の exact
  coverage を manifest validation が fail-fast で強制する。
- `processPolicy.tasks.*.command` の placeholder は `{fixtureChild}` —
  v1 の `{node}`(node emit.mjs spawn)を置換し、bench bin 自身を指す。
  workspace 生成時に `current_exe()` の abs path へ展開する。
- scenario `policy` overlay・`armEnv`・`armFakeStream`・`armExpect`・
  `comparisons` 宣言で arm ごとの env・fixture・期待形・比較 pair を
  manifest 側が所有する(code-side hardcode なし)。

### fixture child(自己再実行)

`cheaper-bench --fixture-child <mode>` は bench bin 自身の自己再実行
で、`main()` が arg parse より先にこの marker で分岐する。process
workload・route grant 対象・nonzero exit の供給源(node emit.mjs の
置換)。mode 語彙:

- `emit` — `CHEAPER_BENCH_CHILD_BYTES`(既定 4096)byte の seeded
  高エントロピ印字可能列を stdout へ書き exit 0(condense の
  `outputTokens > tokenThreshold` 評価の決定性を担保する既知 token
  密度)。
- `fail` — stdout/stderr marker を書き `CHEAPER_BENCH_CHILD_EXIT`
  (既定 7)で exit(nonzero exit・exit-code mirror 計測用)。
- `sleep`・`envdump` — 予約語彙(required 12 scenario は消費しない)。

route 経路では typed command `benchrun --fixture-child <mode>` の
argv[1..] がそのまま child argv になる(grant の `argv` は match
pattern であり、spawn argv への注入器ではない — design §1.4)。

### Route・失敗 scenario

- Route scenario(`route-*` 4 件): manifest `route` block が
  routes.yaml・policy grant を materialize し、`route-resolve`・
  `route-rewrite`・`route-run`・`route-logs`・`direct-child-exec` の
  arm 語彙で route orchestration overhead・condense/refetch・
  truncation・exit code mirror を計測する。
- `worker-failure-modes`: `armFakeStream` が error/hang/schema 違反
  session を供給し、`armExpect` で失敗形(envelopeOk・exitCode 等)
  を pin する。workflow arm の timeout/repair は scenario `policy`
  overlay が担う(fixture DSL の `policy`/`abort`/`expect` は `run`
  経路専用で workflow には効かない)。
- `armExpect` 不一致を持つ observation は `success:false` で記録
  され、run 終了時に scenario 単位で失敗報告される(exit≠0)。

## fake stream seam(計測対象 binary 側)

seam 自体は v1 と同じ — `CHEAPER_TEST_FAKE_STREAM` が RunFixture
JSON の path を指し、`NODE_ENV=test` と併せて activation する
(`fake_stream.rs` の `NODE_ENV === "test"` 規則。env 名は node 命名の
vestige で、rename は本移行の範囲外)。fixture JSON
(`sessions[].steps` — `turn`/`tool_calls`/`delay_ms`/`usage_delta`/
`finish` DSL)は harness が scenario の `fakeStream`/`armFakeStream`/
`manual` 宣言から spawn ごとに生成する。spawn env は v1 parity の
全置換(`PATH`/`SystemRoot`/`TEMP`/`TMP`/`CHEAPER_LOG*` passthrough +
`HOME`/`CHEAPER_CONFIG_DIR` 等)。

## result schema v2 と claim 境界

`benchmarks/results/<runId>.json` は `schemaVersion: 2` — top-level
に `runId`・`createdAt`・`fixtureRevision`・`repositoryRevision`・
`conditions`・`measuredBinary`(path/profile/devSeamProbe)・
`environment`(os/osRelease/arch/cpuCount/ambientRustc/
harnessProfile — v1 の `environment.node` field は削除・再混入は
schema validation が reject)・`observations[]`・`aggregate` を持つ。

`aggregate.comparisons[]` entry の `claim` enum:

- `measured` — 測定 fact(`assumptions`・`reason` は両方 null)。
- `estimated` — `--pricing` 指定時のみ emit。`assumptions.pricing`
  (model・source・単価)+ `boundary:"scripted-usage"` を必須 echo。
  scripted usage 由来の推計であり、実料金比較として表示しない。
- `unavailable` — delta 全値 null + `reason` 必須(v1 の
  `status:"incomplete"` の arm 欠落もここへ fold)。

`parentUsage.status` が `unavailable`(現行は常に — 親 agent の
token telemetry は未接続)の場合、結果から親 token・費用削減を
主張しない。observation の usage は全て `source:"scripted"` の
fixture 宣言値で、未標識の scripted usage は validation が
fail-fast で拒否する。

## baseline 同等性(migration note)

v1(`schemaVersion:1`)artifact の値と v2 の値は直接比較しない:

- `process-*` 系 scenario の v1 wall-time は node 起動 cost
  (emit.mjs spawn)を含んでいた — fixture child 化で絶対値がずれる。
- `direct-filesystem` arm も `node:fs` spawn から `std::fs::read`
  (harness 内の in-process 計測)へ実装変更。
- `environment` の値 domain も異なる(v1 `os:"win32 10.0.x"` +
  `node` field → v2 `os:"windows"`/`osRelease` 分離・
  `arch:"x86_64"` 等の Rust 命名)。
- 区別は `fixtureRevision: "seeded-v2"` で行う。v1 file は
  `benchmarks/results/` に不変で残るが、v2 の load/aggregate は
  `schemaVersion` を確認して v1 を reject する(混在 run を作らない)。

## 計測と会計

- workflow の Worker usage は成功 envelope の `jobs[].meta` から
  一度だけ集計する(workflow run は `usage.jsonl` に記録されない)。
- direct arm は各 CLI envelope + usage.jsonl 行数を別系統で記録し、
  token 集計へ再加算しない(v1 parity)。route arm は
  `kind:"route"`/`"route-refetch"` record の件数を
  `directUsageRecordCount`・`logRefetches` として記録し、route
  envelope の usage と二重計上しない。
- usage は fixture の `usage_delta` で宣言した deterministic な
  固定値。median/p95/stableValue は v1 parity algorithm(補間しない
  nearest-rank)を `aggregate.rs` の純粋関数が pin する。

scenario 追加時は manifest(`benchmarks/scenarios/scenarios.json`)の
required coverage・arm impl 語彙・`armExpect` を design §2 に従って
揃える。harness の unit test(`crates/cheaper-bench/tests/` —
manifest/fixture/schema/aggregate)は `cargo test -p cheaper-bench`
で実行し、`mise run check` の `cargo test --workspace` に含まれる。
benchmark 実測(`cargo run -p cheaper-bench`)は引き続き手動実行で、
spawn 測定を `cargo test` へ封入しない。
