---
id: "TASK-IoIMnYeXx4TFTREaOLTquP"
type: "task"
status: "done"
title: "Implement benchmark record/aggregate schema v2 — claim invariants, median/p95, armExpect, immutable store, comparisons, estimated layer"
created: "2026-10-25"
updated: "2026-10-25"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/183"
  changes:
    added:
      - type: "file"
        path: "crates/cheaper-bench/src/record.rs"
      - type: "file"
        path: "crates/cheaper-bench/src/aggregate.rs"
      - type: "file"
        path: "crates/cheaper-bench/src/report.rs"
      - type: "file"
        path: "crates/cheaper-bench/tests/schema.rs"
      - type: "file"
        path: "crates/cheaper-bench/tests/aggregate.rs"
    modified:
      - type: "file"
        path: "crates/cheaper-bench/src/cli.rs"
      - type: "file"
        path: "crates/cheaper-bench/src/lib.rs"
      - type: "file"
        path: "crates/cheaper-bench/src/measure.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-rust-native-benchmark-harness-issue-183.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-rust-native-benchmark-harness-issue-183.md"
    - "docs/tasks/implement-bench-manifest-v2-scenarios-and-observation.md"
  blocks:
    - "docs/tasks/wire-bench-check-pipeline-and-sync-docs.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-rust-native-benchmark-harness-issue-183.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/cheaper-bench-harness-contract-issue-183.md"
  references:
    - "docs/designs/benchmark-harness-redesign-issue-183.md"
  defers: []
  deferred-by: []
---

# Implement benchmark record/aggregate — schema v2, claim invariants, aggregation, immutable store

## Work

T2 の観測層の上に、result record/aggregate の schemaVersion 2 型・
validation invariant・armExpect 照合・median/p95/stableValue 集計・
comparisons/reduction・estimated layer・immutable result store を実装
する(design §3・§4)。

1. `src/record.rs` — schema v2 型 + validate + armExpect 照合:
   - observation record(design §3.3): `schemaVersion`・`runId`・
     `scenarioId`/`scenarioTitle`・`arm`/`repetition`/`warmup`・`mode`
     (v1 mode 名を継承 — `direct-worker-tool-call` arm は
     `single-direct-worker-with-one-tool-call`、rename しない)・
     `parentUsage{status,reason|source}`・`fixtureRevision`・
     `environment` block(§3.2 — `os`/`osRelease`/`arch`/`cpuCount`/
     `ambientRustc`/`harnessProfile`、`node` field は存在しない)・
     `command`・`wallTimeMs`・`success`・`exitCode`・`envelopeOk`・
     `errorCode`・`directUsageRecordCount`・
     `directInvocationTurnsProxy`・workflow 系 field・`route` block・
     `metrics`(v1 comparable dims + `usage.source:"scripted"`・
     `repairs`/`transientRetries`・`postprocessFailures`・
     `logRefetches`・`materializedPromptBytes`/`workerStreamConcurrency`
     は引き続き null = 未計測)。
   - result top-level(design §3.1): `schemaVersion:2`・`runId`・
     `createdAt`・`fixtureRevision:"seeded-v2"`・
     `repositoryRevision`・`conditions`・`measuredBinary{path,profile,
     devSeamProbe}`・`environment`・`pricing`・`observations[]`・
     `aggregate`。
   - validation invariant(design §3.4 — fail-fast で強制):
     1. `schemaVersion === 2` 完全一致(v1 record は reject)。
     2. `parentUsage.status ∈ {observed, unavailable}`・unavailable ⇒
        `reason` 必須・observed ⇒ `source` 必須。
     3. `metrics.usage` を持つ observation は `usage.source ===
        "scripted"` 必須。
     4. `environment.node` 存在は reject・`ambientRustc`・
        `measuredBinary.profile` は必須。
     5. comparisons entry: `claim ∈ {measured, estimated,
        unavailable}` — `unavailable` ⇒ `deltas` 全値 null +
        `reason` 必須、`estimated` ⇒ `assumptions` record 必須
        (`{pricing:{model,catalogRevision?,source,inputUsdPerMTok,
        outputUsdPerMTok,cachedInputUsdPerMTok?},tokenizer:
        "o200k_base",boundary:"scripted-usage"}`)、`measured` ⇒
        `assumptions`/`reason` は key を emit し両方 null(null 以外
        は mislabel として fail-fast)。v1 の `status:"incomplete"`
        は `claim:"unavailable"` + reason へ fold。
     6. record は `parentUsage` block 以外に parent usage の数値
        field を持たない。
   - armExpect 照合(§2.3/§3.3): 宣言 key のみ等値比較(数値・bool・
     文字列・null)・不一致項目を持つ observation は `success:false`
     で記録(**即時 abort しない**)・run 終了時に mismatch obs を
     抱える scenario を失敗報告(exit≠0)。`armExpect` 未宣言の補助
     arm のみ fallback で `exitCode==0`(envelope arm は加えて
     `envelopeOk===true`)を success とする。

2. `src/aggregate.rs` — v1 parity の集計関数(design §4.1 の algorithm
   を pin):
   - `median`: 昇順 sort → 奇数 `sorted[floor(n/2)]`・偶数は
     `sorted[n/2-1]` と `sorted[n/2]` の平均。
   - `p95`(`percentile(xs,0.95)`): 昇順 sort → nearest-rank
     `sorted[max(0,ceil(n*f)-1)]`(補間しない)。
   - `stableValue`: 全 obs で finite number かつ全同一ならその値、
     変動・欠測・非数を含むなら null。
   - warmup observation は集計除外。arm 集計は scenario×arm group 化
     → `medianWallTimeMs`/`p95WallTimeMs`/`stableMetrics`/
     `observationCount`/`parentUsage` 集約 + v2 追加の `successRate`・
     exitCode/errorCode・postprocessFailures/logRefetches/route dims
     の stableValue。
   - comparisons: manifest `comparisons` 宣言の `{left,right}` pair +
     optional `reduction{left,right}` を処理 — delta 対象 dim は
     design §4.2 の確定集合(median と p95 の双方)、`reductionBytes`
     /`reductionRatio`(どちらか null・left==0 なら null)を emit。
   - estimated layer: `--pricing <file>` で `{model,inputUsdPerMTok,
     outputUsdPerMTok,cachedInputUsdPerMTok?,source,catalogRevision?}`
     を与えたときのみ emit — file は load 時 fail-fast(malformed・
     必須 key 欠落・非数値・未知 key は即 error)。estimated entry は
     comparisons に別 entry(`claim:"estimated"` + assumptions echo
     (model・catalogRevision 必ず含む)+ `deltas.estimatedCostUsd`)。
     `--pricing` 無しでは一切 emit しない。

3. `src/report.rs` — result JSON 組立て + 書込み:
   `benchmarks/results/<runId>.json`(UUIDv4)へ
   `OpenOptions::create_new(true)` — 既存 file・`--output` 明示 path
   ともに非上書き契約。v2 の load/aggregate は `schemaVersion` を確認
   して v1 file を reject する(混在 run を作らない)。

4. `src/cli.rs` / `src/measure.rs` — observation → record 組立て →
   validate → 全 observation 集めて aggregate → report 書込みの配線。
   run 終了時の mismatch scenario 失敗報告(exit≠0)を含む。

5. `tests/schema.rs` — record/aggregate v2 validation(上記 6
   invariant・claim 別形状・scripted marker・environment.node 拒否・
   parent usage 数値 field 不存在・armExpect 照合の等値/null 期待/
   fallback 意味論)の unit test。

6. `tests/aggregate.rs` — median/p95/stableValue/comparisons/
   reduction/estimated layer の純粋関数 boundary case(単一要素・
   偶数/奇数・全同一・変動/欠減混在・nearest-rank 非補間・
   `--pricing` 有無の emit 差異)の unit test。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-rust-native-benchmark-harness-issue-183.md (T3 slice)
- docs/designs/benchmark-harness-redesign-issue-183.md (§3 result schema v2・§3.4 claim invariant・§3.5 保存・§4 aggregation)

## Dependencies

- docs/plans/implement-rust-native-benchmark-harness-issue-183.md
- docs/tasks/implement-bench-manifest-v2-scenarios-and-observation.md

## Affected Files

- `crates/cheaper-bench/src/{record,aggregate,report}.rs`(新設)
- `crates/cheaper-bench/src/{cli,measure}.rs`(record→aggregate→report 配線)
- `crates/cheaper-bench/tests/{schema,aggregate}.rs`(新設)

対象外: manifest/scenario(T2)、doc sync(T4)、spawn 測定(T5/T7)、
`benchmarks/` の `.mjs` 系(T6)。

## Done When

- [x] record/aggregate の schema v2 型が design §3.1-§3.3 どおりに定義
      され、`environment` block に `node` field を持たない
      (`record.rs` — emit 実測の environment key は
      `os/osRelease/arch/cpuCount/ambientRustc/harnessProfile`)
- [x] validation が design §3.4 の 6 invariant を fail-fast で強制する
      (v1 reject・parentUsage status/理由・usage.source scripted・
      environment.node/ambientRustc/profile・comparisons claim enum +
      claim 別形状・数値 parent-usage field 不存在)—
      `record::validate_result` + `tests/schema.rs` 18 test
- [x] armExpect 照合が宣言 key のみ等値比較で、mismatch は
      `success:false` 記録(即時 abort なし)+ run 終了時に mismatch
      scenario が exit≠0 で報告される(`cli.rs` run 終端の failure
      集約 — record は書いた上で非 0 exit)
- [x] median/p95/stableValue が v1 parity algorithm(補間しない
      nearest-rank)で実装され、`tests/aggregate.rs` が boundary
      case を固定する(12 test)
- [x] comparisons が manifest 宣言から駆動され、delta が median/p95
      双方・`reduction` が `reductionBytes`/`reductionRatio` を emit
      する(sample emit で measured entry の両 delta + reduction
      block を確認)
- [x] `--pricing` 有無で estimated entry の emit が変わり、pricing
      file の load が fail-fast(malformed・必須 key 欠落・非数値・
      未知 key)する
- [x] `benchmarks/results/<runId>.json` への `create_new` 非上書き
      書込み・v1 file の reject が動作する(`report.rs` —
      `tests/schema.rs` で二重書込み拒否 + v1 load reject を pin)
- [x] `cargo test -p cheaper-bench` が pass(tests/schema.rs +
      tests/aggregate.rs を含む — 99 test 全 pass)

## Verification

- [x] `mise exec -- cargo test -p cheaper-bench` exits 0
      (40 lib + 12 aggregate + 11 fixture + 18 manifest + 18 schema)
- [x] `mise exec -- cargo test -p cheaper-bench --test schema` exits 0
      (18 passed)
- [x] `mise exec -- cargo test -p cheaper-bench --test aggregate` exits 0
      (12 passed)
- [x] `mise exec -- cargo fmt --all -- --check` exits 0
- [x] `mise exec -- cargo clippy -p cheaper-bench --all-targets -- -D warnings` exits 0
- [x] Manual: `grep -rn "serde_json::to_string\|to_string_pretty" crates/cheaper-bench/src/` が byte 計測経路に hit しない(byte 計測は `cheaper_core::json::stringify` 経由 — supplement)
      — hit は file 書出し・stdin payload 構築のみで、byte 計測の
      唯一経路 `measure::json_bytes` は `stringify().len()`
- [x] `mise run check` exits 0(canonical verification contract 全体)
- [x] Manual: sample emit(`--scenario single-simple-tool-call
      --warmups 0 --repetitions 1 --output <temp>`)の result JSON が
      §3.1 の全必須 key・`environment.node` 不在・claim 別 null/value
      規則・`metrics.usage.source:"scripted"`・非 envelope arm の
      `envelopeOk:null` を満たすことを目視確認。
      `benchmarks/results/**` は git status/diff で変更無しを確認
