---
id: "DISC-qgvOWFM6MtYgrq3viqbssW"
type: "discovery"
status: "resolved"
title: "Benchmark harness audit and redesign space (issue #183)"
created: "2026-10-25"
updated: "2026-10-25"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/183"
    - "benchmarks/scenarios/scenarios.json"
    - "benchmarks/results/e45d35b7-2fa6-416b-946a-70ca844b9c80.json"
    - "crates/cheaper-cli/src/fake_stream.rs"
    - "crates/cheaper-runtime/src/dev/fixture.rs"
    - "crates/cheaper-cli/tests/common/mod.rs"
    - "crates/cheaper-cli/tests/integration/route_run.rs"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/benchmark-redesign-scope-issue-183.md"
    - "../adr/rust-native-benchmark-harness.md"
    - "../designs/benchmark-harness-redesign-issue-183.md"
  verifies: []
  verified-by: []
  references:
    - "../discovery/node-dev-tooling-inventory-issue-182.md"
    - "../specs/node-dev-tooling-scope-issue-182.md"
    - "../adr/keep-node-js-as-the-pinned-dev-tooling-runtime.md"
    - "../designs/node-dev-tooling-boundary-contract-issue-182.md"
    - "../discovery/workflow-orchestration-benchmark.md"
    - "../specs/workflow-orchestration-benchmark.md"
    - "../designs/workflow-orchestration-benchmark.md"
    - "../adr/benchmark-harness-placement-and-accounting.md"
    - "../plans/implement-workflow-orchestration-benchmarks.md"
    - "benchmarks/README.md"
    - "docs/guides/workflow-usage.md"
    - "mise.toml"
  defers: []
  deferred-by: []
---

# Benchmark harness audit and redesign space (issue #183)

## Exploration Goal

issue #183 は 3 つの作業を要求する:

1. **現行 Node harness の監査と移行判断** — `benchmarks/run.mjs` +
   `lib/*.mjs` + `test/*.mjs` + `scenarios/scenarios.json` の意味論・
   検証項目の棚卸しと、#182 の toolchain 決定(Option A — Node 維持)
   を踏えた harness architecture の採択。`.mjs` 継続は要件ではなく、
   製品に Node 依存を追加しない。
2. **測定拡張** — direct canonical tool/CLI vs Worker vs saved
   Workflow、fan-out/fan-in、large intermediate output、Route の
   rewrite→routed-exec→condense を実行可能なローカル fixture
   (明示 grant・fake Worker・子コマンド実行)で分けて測る。
3. **主張境界** — measured / estimated / unavailable の 3 層を
   record schema と文書で厳格に分離する。

本 doc は (1) の監査結果と決定空間を固定する。(2)(3) の要件化は
`docs/specs/benchmark-redesign-scope-issue-183.md`、architecture
採択は `docs/adr/rust-native-benchmark-harness.md` が担う。

## Key Issues and Assumptions

- harness 自体は Node だが spawn 対象は Rust binary — harness
  言語と計測対象 runtime は別物として評価する。
- #182 は Option A(Node を dev-tooling runtime として pin 維持)を
  採択済み。**どの選択肢を取っても `mise.toml` の node pin は残る**
  (doc-suite `.js` が upstream 所有)ため、harness 言語の決定は
  node removal payoff ではなく harness 自体の適合性で評価する。
- issue #183 は scenario・計測面の変更を必須とするため、
  #182 design の trigger T2(benchmark harness の大改修機会)は
  すでに発火している。F2(benchmark harness の Rust 化)は「issue
  #183 への結合が前提」と明示的に本 issue へ deferred されている。
- workflow-usage.md の掲載値は TS CLI 時代の記録であり、Rust
  現在形の baseline 再測定は言語選択とは独立して必須。

## 現行 harness の監査

### File 別責務(benchmarks/ 全体 ≈1,973 行)

| File | 行数 | 責務 |
| --- | ---: | --- |
| `run.mjs` | 74 | CLI entry。`--scenario`/`--repetitions`(15)/`--warmups`(2)/`--output` を parse し scenario×arm×(warmups+reps) を逐次実行。`git rev-parse HEAD` を repositoryRevision として採取し、UUID 名で `benchmarks/results/` へ `wx` flag(非上書き)書込み |
| `lib/cli.mjs` | 165 | binary 解決(`target/release` → `target/debug` fallback + win32 `.exe`)、`prepareWorkspace`(seeded files・`.cheaper/models.yaml`・`processPolicy` の `{node}` → `process.execPath` 置換した policy.yaml)、`benchmarkEnvironment`(PATH passthrough・HOME/USERPROFILE=workspace・`NODE_ENV=test`・`CHEAPER_CONFIG_DIR`・`CHEAPER_TEST_FAKE_STREAM`・SystemRoot/TEMP/TMP・`CHEAPER_LOG*` ambient passthrough・per-arm env override)、`runCli`(fixture JSON を spawn 直前に再生成 → spawn → 単一 JSON envelope parse・wall-time 計測)、`countUsageRecords`(usage.jsonl 行数) |
| `lib/run-scenario.mjs` | 461 | 全 arm 実装。`workflow`/`dynamic-workflow`/`saved-workflow`(definition 生成 + `workflow interface` + `workflow run` + envelope 計測)、`direct-filesystem`(`node:fs/promises.readFile`)、`direct-worker-tool-call`(`run bulk-reader` + 1 read tool call)、`direct-worker-sequence`(file read → hints 化 → `run bulk-reader` 逐次 + `processScript` を `process.execPath` で spawn)、`full-definition`(workflow.json の readFile)、`compact-interface`(`workflow interface` envelope)。temp workspace 確保・armBase/armEnv 解決・`workflowMaxJobs` → policy.yaml・record 検証・cleanup |
| `lib/measurements.mjs` | 92 | `interfacePayloadBytes`(envelope.result serialize byte)、`criticalPathMs`(needs + `$from` binding の依存 graph から job `meta.durationMs` の最長路)、`measureWorkflowEnvelope`(retainedBytes = result bytes + `meta.artifacts[].bytes` 和)。`materializedPromptBytes`・`workerStreamConcurrency` は Rust seam に観測 hook が無く常に `null`(未計測) |
| `lib/result-schema.mjs` | 117 | envelope 検証(`validateWorkflowEnvelope` — schemaVersion/workflow discriminator/ok/jobs[])、`summarizeWorkflowEnvelope`(jobs[].meta の turns/toolCalls/usage 集計)、`summarizeWorkerEnvelope`(direct run envelope から同等集計)、`validateBenchmarkRecord`(schemaVersion 1・scenarioId/arm・`parentUsage.status ∈ {observed,unavailable}` + reason/source 必須化・metrics 存在) |
| `lib/aggregate.mjs` | 175 | arm 別集計 — median/p95 wallTime・`stableValue`(全反復で同一なら値、変動または欠測なら `null`)・usage/size field の flatten・`parentUsage` 集約。arm 間差分は 9 ペアを COMPARISONS に hardcode(left − right)、`definition-vs-compact-interface` には `parentVisibleReduction{Bytes,Ratio}` の特例計算。warmup observation は集計除外 |
| `lib/scenarios.mjs` | 66 | manifest 検証 — schemaVersion 1・id 一意・`REQUIRED_SCENARIO_IDS` の exact coverage(missing/unexpected 双方拒否)・title・workflow 非空 jobs・arm に workflow 系を含むこと・`armEnv`/`armBase` の宣言 arm 整合 |
| `fixtures/fake-worker-stream.mjs` | 102 | `CHEAPER_TEST_FAKE_STREAM` 用 RunFixture JSON generator。`sessions[].steps` に `tool_calls`/`turn`/`delay_ms`/`usage_delta`/`finish` を組立て、bulk-reader output schema 適合の応答 text を生成。DSL 全 11 種の step のうち 5 種のみ使用 |
| `scenarios/scenarios.json` + `saved-workflow.json` | 392+17 | 7 scenario の declarative manifest(下記) |
| `test/*.test.mjs` | 312 | 4 suite — aggregate・measurements・result-schema・scenario-manifest。`mise run check` の `node --test benchmarks/test/` に配線済み(mise.toml:73) |
| `results/*.json` + `results/README.md` | — | append-only 結果置場(UUID file・上書き禁止・commit 対象) |

### 7 scenario / arm の意味論

issue 記載は「7 scenario・17 arm」だが、現行 manifest の declared
arm は **20** — `single-simple-tool-call` の `run-logging-{off,file,
debug}` 3 arm が observability 導入時(commit 80e8eee)に追加された
分で、17 は追加前の数。3 arm は `armBase` → `direct-worker-tool-call`
への alias + `CHEAPER_LOG` env matrix(off-state overhead 計測
arm、DESIGN-034DV9ypzmxdAobGSJ6Cm6)。

| Scenario | Arms | 測るもの |
| --- | --- | --- |
| `single-simple-tool-call` | workflow, direct-filesystem, direct-worker-tool-call, run-logging-{off,file,debug} | 単一 read tool job の workflow vs 同一作業を delegate しない基準(fs read) vs Worker 1 call + tool call。CLI 起動 overhead と orchestration overhead の分離点。logging arms は `CHEAPER_LOG` off/file/debug の overhead |
| `independent-fanout-worker-fanin` | workflow, direct-worker-sequence | 独立 read 2 件 + Worker fan-in(`workflowMaxJobs:3`)。並列化余地と fan-in で親へ返す量の違い |
| `tool-to-worker-chain` | workflow, direct-worker-sequence | read → analyze Worker の直列 chain。hint として file を hint 化する direct 側との retained bytes 差 |
| `process-and-analysis` | workflow, direct-worker-sequence | managed process(`emit.mjs`)→ Worker。`processPolicy.tasks.emit.command = ["{node}","emit.mjs"]` で **`process.execPath` (node) を計測対象 workload として spawn** |
| `large-intermediate-process-analysis` | workflow, direct-worker-sequence | 同上・32 KiB 出力。大きな中間値を hint として materialize する cost |
| `dynamic-vs-saved-workflow` | dynamic-workflow, saved-workflow, direct-worker-sequence | definition 生成 vs `saved-workflow.json` copy の discovery 差。`definitionGenerationMs`・`workflowDefinitionBytes`・`parentVisibleInterfaceBytes` |
| `definition-vs-compact-interface` | workflow, full-definition, compact-interface | private definition 全文 read vs `workflow interface` の公開 payload。`parentVisibleReduction` を特例計算 |

### Result schema v1(observation + aggregate)

- top-level: `schemaVersion:1`・`runId`・`createdAt`・
  `fixtureRevision:"seeded-v1"`・`repositoryRevision`・
  `warmupCount`・`repetitionCount`・`scenarioIds`・`observations[]`・
  `aggregate`。
- observation: `arm`・`mode`(workflow/direct-tool-operation/
  scripted-direct-cli-sequence/full-definition-read/
  public-interface-discovery 等)・`parentUsage{status,reason|source}`
  (常に unavailable — parent telemetry 未接続)・
  `environment{os,architecture,node,cpuCount}` — **`node` field は
  `process.version` を記録するが `validateBenchmarkRecord` では
  未宣言・未検証**(#182 design §1.2 #8 で指摘済み)・`command`・
  `workflowDefinitionBytes`・`parentVisibleInterfaceBytes`・
  `interfaceDiscoveryWallTimeMs`・`discoveryMode`・
  `definitionGenerationMs`・`wallTimeMs`・`directUsageRecordCount`・
  `directInvocationTurnsProxy`・`metrics{}`。
- `metrics` 内の null(未計測)dimension: `materializedPromptBytes`・
  `workerStreamConcurrency`(Rust seam に観測 hook 無し)。
- aggregate: `arms[]`{observationCount・parentUsage・
  medianWallTimeMs・p95WallTimeMs・stableMetrics}+
  `comparisons[]`{status: measured|incomplete・deltas}。
- 会計規則: workflow の Worker usage は envelope `jobs[].meta`
  から一度だけ集計(workflow run は usage.jsonl に記録されない)、
  direct arm は各 CLI envelope + usage.jsonl 行数を別系統で記録
  (aggregate の token source には再加算しない)。

### TS 時代 vs 現行の境界

- `e45d35b7-….json`(2026-10-05・rev `b3213f8c`)は TS CLI 時代の
  全 7 scenario run(`node packages/cli/dist/main.js`、workflow
  median ≈2,795 ms)。`docs/guides/workflow-usage.md` は本 artifact
  のみを引用しており **Rust 現在形の baseline ではない**。
- その他 7 file(いずれも 2026-10-05)も TS 時代の単一 scenario run。
- `4eda3caf-….json`(2026-10-11・rev `80e8eee`)は Rust binary
  (`target\release\cheaper.exe`)の **部分 run** —
  `single-simple-tool-call` 1 scenario のみ(workflow median
  ≈154 ms)。全 scenario の Rust baseline は未記録。
- harness 自体は commit `461dc04` で Rust binary へ retarget 済み
  (fake-worker-stream.mjs が module seam → RunFixture JSON
  generator へ転換、README も Rust 版 seam 記述へ更新済み)。
- 未計測のまま残った dimension: `materializedPromptBytes`・
  `workerStreamConcurrency`(Rust fake-stream DSL に観測 hook が
  存在しない)。Route 系・失敗系の dimension は schema に無い。

## Rust 側の既存資産(harness が再利用可能な seam / fixture)

| 資産 | 場所 | benchmark での使い道 |
| --- | --- | --- |
| fake-stream seam | `crates/cheaper-cli/src/fake_stream.rs` | `CHEAPER_TEST_FAKE_STREAM` + `NODE_ENV=test` で RunFixture JSON を load(`dev-seam` cargo feature で gate — ADR-034DyGyITd0YaATg8fkld7)。spawn binary に `--features dev-seam` build が必須 |
| RunFixture DSL | `crates/cheaper-runtime/src/dev/fixture.rs`(969 行・serde) | `sessions[].steps` — emit/delay_ms/tool_calls/usage_delta/turn/**error**/**hang**/**hang_until_abort**/**abort_response**/**panic**/finish の 11 種 + worker/policy/provider/model/env/auth_logins/input/abort/abort_grace_ms/expect 節。**worker failure・timeout・postprocess failure の fixture は DSL 側にすでに存在**し、`.mjs` generator は 5/11 step しか使っていない(拡張すれば JS 側に DSL の parallel 実装が育つ = drift 面)。型は **Deserialize-only**(serde import :71・Serialize impl 無し — `RunFixture::load`/`parse` は parse 専用 :138-157)で、`FixtureStep` は bare string/keyed object の untagged 二形(:418-470)ゆえ正準出力形も未定義 — fixture JSON 生成は `serde_json::json!`/template 組立て + `RunFixture::parse` round-trip 検証で canonical schema へ fail-fast anchor する方式を ADR が固定(24 件の hand-authored fixture が同経路で検証済み) |
| self-re-exec fixture child | `crates/cheaper-tools/tests/proc_process.rs`(`CHEAPER_PROC_CHILD_MODE`)・`crates/cheaper-cli/tests/integration/route_run.rs`(`CHEAPER_ROUTE_CHILD_MODE` mode 群: io/ok/envdump/big/tail/sleep/e2e)・`crates/cheaper-cli/tests/workflow_dev_seam.rs`(独自 reshim) | `current_exe()` + mode env で test binary 自身を子 process として再実行する実証済み pattern。**`{node}` workload の代替として使える** — harness binary が自ら fixture child になれば emit/exit-code/large-output を runtime 非依存で供給できる |
| fixture→test emit channel | `CHEAPER_PROC_PID_DIR`(issue #180) | pid dir 経由の決定的待ち合わせ機構 |
| in-process `run_cli` | `crates/cheaper-cli/tests/common/mod.rs` `Fixture` | temp ws・cfg・env_map・stdin/stdout/stderr sink の組立一式。**process 起動 cost を含まない**ため CLI-overhead の wall-time 計測には使えない(spawn 計測継続が前提) |
| Route 実行面 | `route resolve --command/--argv`(dry-run)・`hook pre-tool-use --target pretooluse`(stdin `{"tool_name":"Bash","tool_input":{"command":…}}` → `updatedInput.command` = `"cheaper" route run --payload <tok>` の rewrite verdict)・`route run --payload`(routed-exec workflow)・`route logs <id>`(refetch) | Route 測定の全経路が CLI spawn で再現可能。`route run` 内部は `resources/workflows/routed-exec.workflow.json`(exec command job → `when` outputTokens>tokenThreshold で condense worker)= 「routed-exec→condense」の実体 |
| payload codec | `crates/cheaper-cli/src/route/payload.rs` `encode(EncodeInput)`(pub) | `route run --payload` の token を harness 側で mint 可能 — route_run.rs テストが実証 |
| routed-exec・condense builtin | `crates/cheaper-cli/resources/{routes.yaml,workflows/routed-exec.workflow.json,workers/condense.md}` | 実 route の builtin 定義。`condense` worker は read 限定・`outputPerInput:0.05`・log artifact source 経由の要約 |
| usage.jsonl | `<CHEAPER_CONFIG_DIR>/usage.jsonl` | `kind:"run"`/`"route"`/`"route-refetch"` record。会計の統一対象 |
| fixture 実例集 | `crates/cheaper-runtime/tests/fixtures/*.json`(24 件) | success・repair・timeout・max-turns/tool-calls・abort・error retry・panic・usage-delta の参照実装 — benchmark fixture の seed |

## #182 決定文脈(F2 が本 issue へ defer したもの)

- `docs/adr/keep-node-js-as-the-pinned-dev-tooling-runtime.md`:
  Option A 採択 — node pin は doc-suite(upstream 所有 42 `.js` + 1
  `.cjs`)がある限り除去不能。**Option B(benchmarks のみ Rust 化)は
  「required entrypoints 6→5 に減るだけで removal payoff 無し、
  稼働中 harness の回帰リスクに見合わない」として当時は却下**。
  ただし revisit trigger (2)「benchmark harness の大改修
  (scenario・計測面の変更)が必要になったとき — その機会に Rust 化
  を畳み込むか再評価」を明記。
- `docs/designs/node-dev-tooling-boundary-contract-issue-182.md`:
  - §1.2 row 8 — `run.mjs` は optional/manual entry、`{node}` →
    `process.execPath` 置換 scenario(scenarios.json:194・263)が
    「計測対象 workload としての node」を含み、結果 record の
    `environment.node` field を保持(検証未宣言)。
  - §2.1 — `benchmarks/` は repo tooling として node 結合可、ただし
    「harness 全体の構造変更は issue #183 に委ね」。
  - §4 T2 — issue #183 の着手が trigger。「`{node}` workload
    scenario と `environment.node` field の再設計が焦点」。
  - §5 F2 — 「`run.mjs`+`lib/*.mjs`+`test/`+fake-stream fixture を
    Rust harness へ移行し `{node}` workload scenario・
    `environment.node` field を再設計」。acceptance sketch:
    「workflow vs direct の計測意味論が維持され、結果 record
    schema の変更が version/比較可能性として明示される。
    `node --test benchmarks/test/` が cargo test 系へ置換される」。
    ordering は「**issue #183 への結合が前提**」— 本 issue が F2 の
    実施機会。
- issue 本文の対応: 「Rust native harness(Cargo test/bench、専用
  bin/xtask 等)か Node 維持を採択」「Rust へ移す場合、既存
  `CHEAPER_TEST_FAKE_STREAM` + `dev-seam` の宣言的 JSON fixture を
  再利用し、JS runtime 前提の fixture/process arm をなくす」
  「Node vs Rust の差を cheaper の改善と誤認しないよう baseline の
  同等性・変更理由を別記」。

## Alternatives and Comparison(harness architecture 決定空間)

| Option | 内容 | 利点 | 欠点 | 傾き |
| --- | --- | --- | --- | --- |
| O1: Node harness 維持+拡張 | `run.mjs`/`lib/` を拡張し Route・失敗系・claim 境界を `.mjs` で実装 | 稼働資産・test 済み schema/aggregate の再利用。JSON 操作の簡潔さ。cross-platform spawn/path 処理が実績済み | `{node}` workload・`environment.node` が残存(arm 再設計で消せるが harness 自体は node 前提のまま)。fake-stream generator を error/hang/panic 等 DSL 残 7 step へ拡張すると **RunFixture DSL の JS 平行実装が育ち drift 面が拡大**。Route executable fixture(grant 付き子 command)も JS 側に新規構築が必要。`node --test benchmarks/test/` が required node entrypoint として残る | no |
| O2: Rust native harness | 専用 bin crate(推奨形)・cargo test・xtask 等で harness を Rust 化 | RunFixture DSL を fixture 生成の canonical schema として anchor できる(型は Deserialize-only — `json!` 組立て + `RunFixture::parse` round-trip 検証で generator drift 解消)。self-re-exec fixture child で `{node}` workload・route child・nonzero exit を runtime 非依存化。route `payload::encode` が pub で mint 可能。`node --test benchmarks/test/` → `cargo test` 吸収で required node entry 6→5。issue の要件実装面(Route fixture・失敗 matrix)が Rust 側の実証済み test pattern と一致 | runner+lib+fixture gen ≈1.25k 行 + test 312 行の移植に加え、再利用実績の無い新規実装(self-re-exec child mode 群・fixture JSON 生成・schema v2・manifest v2・nonzero-exit capture — `runCli` は exit≠0 で throw)が乗り、Option A 比 marginal ≈1.5–3.5k 行 + parity 再検証。result schema の `environment.node` 等で schemaVersion bump が必須。spawn 計測を維持する設計が要る(in-process `run_cli` では起動 cost が落ちて v1 系列と意味論が変わる) | **yes** |
| O3: hybrid | 例: Rust 計測 bin + `.mjs` driver、または cargo micro-bench + `.mjs` e2e | 部分的移行で段階的に進められる | 会計・schema が 2 系統に割れ最悪の保守面。claim 境界を一貫させる責務が曖昧になる | no |

### O2 の形態評価(sub-decision)

| 形態 | 評価 |
| --- | --- |
| 専用 workspace member `crates/cheaper-bench`(bin + tests) | **採用候補**。`cargo run --release -p cheaper-bench -- --scenario …` で手動実行。unit test は `cargo test --workspace` に自然吸収(`node --test benchmarks/test/` の置換)。publish=false・配布なし・dev-only。`cheaper-cli` lib(route payload encode)と `cheaper-runtime::dev`(fixture 型)へ依存可能 |
| xtask crate | 同等の形態だが xtask pattern は未導入で、目的特異名の方が誠実(F4 の check-*.mjs 移行先として将来 xtask が並立し得る) |
| `[[bin]]` を cheaper-cli に追加 | `cargo install --git … cheaper-cli` が全 bin を install 対象にするため consumer へ dev tool が漏れる(required-features で抑止できるが crate 責務を濁す) — 不採用 |
| `[[test]]`/`tests/benchmark/` 専用 target | `cargo test --workspace` が測定を実行してしまい check pipeline が壊れる。`--ignored` 運用は CLI arg 面も弱く不自然 — 不採用 |
| criterion / `cargo bench` | workspace に bench 基盤・依存がなく、必要なのは e2e CLI 計測であり criterion の統計 model は不要(YAGNI) — 不採用 |

## Tentative Conclusions and Hypotheses

- **H1(confirmed)**: 現行 harness ≈1,973 行の責務分解は runner
  (74)・env/spawn/worksapce(165)・arm 実装(461)・計測(92)・
  schema(117)・集計(175)・manifest 検証(66)・fixture
  generator(102)・manifest(392)・unit test(312)に整理できる。
- **H2(confirmed)**: issue 記載「17 arm」は logging arms 追加前の
  数で、現行 declared arm は 20(実装 vocabulary は 8 種 +
  armBase alias)。
- **H3(confirmed)**: TS 時代結果 8 file(単一 scenario 7 +
  merge 1)+ Rust 部分 run 1 file が
  混在し、workflow-usage.md が引用するのは TS 時代値 — Rust
  baseline の全 scenario 再測定は言語選択と独立に欠けている。
- **H4(confirmed)**: Route 測定に必要な全要素(rewrite verdict・
  payload mint・routed-exec workflow・condense worker・route
  logs・kind:"route" usage record・grant 付き fixture child)は
  Rust 側に実証済みで、**全て CLI spawn で再現可能**。
- **H5(confirmed)**: `{node}` workload と `environment.node` は
  node 結合の残滓であり、self-re-exec fixture child で
  runtime 非依存に再設計できる。
- **H6(hypothesis → ADR で採択)**: harness は **Rust native —
  dev-only workspace member `crates/cheaper-bench`** が最適。
  根拠は removal payoff(ゼロのまま)ではなく、(a) RunFixture DSL の
  canonical schema へ fixture 生成を anchor して JS 平行実装
  drift を解消(Deserialize-only — `json!` 組立て +
  `RunFixture::parse` round-trip)、
  (b) issue 要求の測定拡張面(Route executable fixture・失敗
  matrix・hook verdict)が全て Rust 側資産と一致、(c) F2 が本
  issue への結合を前提としていた、(d) required node entrypoint
  6→5 で `node --test benchmarks/test/` が cargo test へ吸収。
- **H7(confirmed)**: wall-time 意味論の保存には product binary の
  subprocess spawn が必須 — in-process `run_cli` は起動 cost を
  含まず、「CLI 起動 overhead と workflow overhead の分離」という
  issue 要求そのものを壊す。

## Open Questions

- result schema v2 の field set — `environment.node` 後続
  (harness/rust toolchain・build profile・feature 表示)、claim 境界
  field(measured/estimated/unavailable の per-dimension 表現)、
  Route・失敗系 dimension の追加項目。design-doc で固定する。
- `direct-filesystem` 相当 baseline の Rust 再定義(harness 内
  `fs::read` を「delegate しない floor」として維持するか、canonical
  tool/CLI 呼出に置き換えるか — issue §2「direct canonical
  tool/CLI」文言の解釈)。`processScript`(`process.execPath` で
  spawn)と `node:fs` read の双方が置換対象のため、fixture
  child・`fs::read` への再定義は本問と一体で決める。
- `{node}` placeholder は manifest v2 で rename が要る(例
  `{bench}`/`{fixtureChild}` — spawn 先が fixture child に変わる
  ため名実を合わせる)。同様に `NODE_ENV=test` は seam activation
  var として Rust 移行後も残る(fake_stream.rs:31)— node 命名の
  vestige として文書で明示する。
- `process-*` scenario の v1 wall-time は node 起動 cost を含んで
  いた — fixture child 化で絶対値はずれる(arm 間 delta は相殺
  される)。baseline 同等性の別記対象として結果・文書で明示する。
- v1 byte 計測(`interfacePayloadBytes`/`retainedBytes`)は
  `JSON.stringify` 由来。Rust `serde_json` の数値 formatting は
  edge case で JS とずれるため、`cheaper-core` の JS 互換
  canonical number formatter(`crates/cheaper-core/src/json/number.rs`
  — root Cargo.toml:23-24 の `raw_value` 前提)を bench crate
  から使い byte parity を保つ(design-doc で固定)。
- `benchmarks/` dir の残存構成(scenarios・results・README は
  言語中立資産として残すか crate 配下へ移すか)。
- usage.jsonl `kind:"route"`/`"route-refetch"` と route envelope の
  数え方の統一仕様(二重計上防止の厳密規則 — spec で要件化)。
- Route arm で `route run --payload` の token mint に `payload::encode`
  を使うか、hook verdict 経路(spawn `hook pre-tool-use`)で採取
  するか — 実装時に両経路の対象計測面を分けて選択する余地。

## Promotion Candidates

- [x] spec-doc needed: redesign の done 定義 — `docs/specs/benchmark-redesign-scope-issue-183.md`
- [x] adr-doc needed: harness architecture(Node 維持 vs Rust vs hybrid + 形態)— `docs/adr/rust-native-benchmark-harness.md`
- [ ] design-doc: result schema v2・scenario manifest v2・arm/route fixture 設計 — 実装着手前に要検討(高確度で必要)

## Sources and Relations

- issue: <https://github.com/scarecrow173/cheaper/issues/183>
- 依存判断: <https://github.com/scarecrow173/cheaper/issues/182>
  (Option A)・follow-up <https://github.com/scarecrow173/cheaper/issues/188>
  (F1 ambient node repoint)・<https://github.com/scarecrow173/cheaper/issues/189>
  (F5 brainstorming node drop)
- 実測(2026-10-25 実施): `benchmarks/` 全 file 精読(run.mjs・
  lib 6・fixtures・scenarios.json・test 4)・`results/` 8 artifact
  の command/environment/scenarioIds 照合(TS 7・Rust 1)・
  `mise.toml`・`docs/guides/workflow-usage.md`・
  `crates/cheaper-cli/src/fake_stream.rs`・
  `crates/cheaper-runtime/src/dev/fixture.rs`・
  `crates/cheaper-cli/tests/common/mod.rs`・
  `crates/cheaper-cli/tests/{workflow_dev_seam.rs,run_dev_seam.rs}`・
  `crates/cheaper-cli/tests/integration/route_run.rs`・
  `crates/cheaper-tools/tests/proc_process.rs`・
  `crates/cheaper-cli/resources/{routes.yaml,workflows/routed-exec.workflow.json,workers/condense.md}`・
  `crates/cheaper-cli/src/route_cmd.rs`・
  `crates/cheaper-cli/src/route/{payload.rs,run.rs}`・
  `crates/cheaper-cli/Cargo.toml`・workspace `Cargo.toml`
- 先行 chain(別 issue・`references` で参照): #67 benchmark chain
  (`docs/{discovery,specs,designs}/workflow-orchestration-benchmark.md`・
  `docs/adr/benchmark-harness-placement-and-accounting.md`・
  `docs/plans/implement-workflow-orchestration-benchmarks.md`)・#182
  chain(inventory・scope・ADR・boundary design)。
