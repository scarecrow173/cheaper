---
id: "TSPEC-tcLTueFfBvt2PbpNF7NRof"
type: "test-spec"
status: "draft"
title: "cheaper-bench harness contract — manifest v2, schema v2 claim boundary, aggregation invariants (issue #183)"
created: "2026-10-25"
updated: "2026-10-25"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/183"
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
  derives-from:
    - "docs/specs/benchmark-redesign-scope-issue-183.md"
    - "docs/designs/benchmark-harness-redesign-issue-183.md"
    - "docs/adr/rust-native-benchmark-harness.md"
  derived-by: []
  verifies:
    - "docs/specs/benchmark-redesign-scope-issue-183.md"
    - "docs/designs/benchmark-harness-redesign-issue-183.md"
  verified-by:
    - "docs/tasks/scaffold-cheaper-bench-crate-and-fixture-child.md"
    - "docs/tasks/implement-bench-manifest-v2-scenarios-and-observation.md"
    - "docs/tasks/implement-bench-record-schema-v2-and-aggregation.md"
    - "docs/tasks/wire-bench-check-pipeline-and-sync-docs.md"
    - "docs/tasks/verify-bench-smoke-run-on-real-binary.md"
    - "docs/tasks/remove-legacy-node-benchmark-harness.md"
    - "docs/tasks/run-full-benchmark-measurement-and-update-workflow-usage.md"
  references:
    - "docs/discovery/benchmark-harness-audit-issue-183.md"
    - "docs/plans/implement-rust-native-benchmark-harness-issue-183.md"
  defers: []
  deferred-by: []
---

# cheaper-bench harness contract — manifest v2, schema v2 claim boundary, aggregation invariants (issue #183)

## Purpose

issue #183 の Rust-native benchmark harness(`crates/cheaper-bench`)が、
「宣言(manifest v2)→ 観測(observation)→ 集計(aggregate)→ 主張
(claim)」の各境界で design(DESIGN-034LUqevp1IIwiPQRmWmE4)の確定
invariant を維持していることを pin する検証契約。harness が deterministic
seeded fixture を誤って解釈したり、claim 境界(measured / estimated /
unavailable)を緩めて主張不能な数値を emit したり、spawn 測定を
`cargo test` に混入させて check pipeline を壊したりしないための判定基準。

本 test spec は将来 `crates/cheaper-bench`・`benchmarks/scenarios/*.json`・
`benchmarks/results/`・`mise.toml` check pipeline・関連文書を書き換える
変更が、design が固定した invariant を維持しているかを判定するための
契約である。廃止・supersede の検討タイミングは、harness が再設計されて
manifest/result schemaVersion が 3 以降へ上がったとき、または claim 境界
の強制面が record validation 以外の層へ移ったとき。

## Feature

benchmark harness は deterministic な seeded fixture だけで動作し、
manifest validation・fixture round-trip・record/aggregate validation・
集計関数の全てが「fail-fast で主張境界を壊さない」形を取る —
measured は測定 fact のみ、estimated は明示 assumptions 付きのみ、
unavailable の上に数値主張は置かない。spawn 単位の測定実行は
`cargo test` に含まれず、`cargo test -p cheaper-bench` は純粋関数・
fixture・manifest・schema の unit 検証のみを担う。

## Rules

### Manifest v2(tests/manifest.rs)

- `schemaVersion === 2`・`fixtureRevision` 非空・scenario id 一意・
  `REQUIRED_SCENARIO_IDS`(design §2.2 の 12 件)の exact coverage —
  missing・unexpected の双方を reject する。
- `armBase`/`armEnv`/`armExpect`/`armFakeStream` の key は全て宣言 arm
  内、`armBase` の value は宣言 arm ∪ impl 語彙(alias 連鎖不可)。
  `armBase` 解決後の名前は impl 語彙(§2.3 の 13 種)に含まれる。
- `route` block: `match.command` は literal token 列のみ(`*` 含有は
  reject)・`match.command[0]=="cheaper"`(case-insensitive)は reject・
  `route.command` の tokenize 後 argv について `match.command` が
  literal prefix であり `argv[1..] == ["--fixture-child", childMode]`
  完全一致・`route.timeout` は DURATION_PATTERN 適合のみ・route 宣言
  scenario は `route-*` impl arm を 1 つ以上含む。
- scenario `policy` overlay は既知 top-level section のみ受理(未知
  section は reject)。`comparisons` の left/right は宣言 arm、
  `reduction` の field 名は既知 metric/dimension 名に限定。
- `armExpect` の key 語彙は §2.6 の確定集合(top-level
  {envelopeOk, errorCode, exitCode, success}・`route` block・`resolve`
  block・`metrics`)に限定。

### Fixture 生成(tests/fixture.rs)

- 生成する全 fixture JSON は書込み前に必ず `RunFixture::parse` を通る —
  profile 駆動生成(`"auto"`/`{profiles:[…]}`)・`{fixture:{…}}`
  passthrough・`armFakeStream` の 3 経路すべてが同一 anchor を持つ。
  無効 step・型違反は生成時 fail-fast。
- DSL 全 step 種(error/hang/hang_until_abort/abort_response/panic 等の
  失敗系を含む)が平行 vocabulary なしで使える — `{fixture:{…}}`
  passthrough が sessions[].steps の全節を manifest から直接受け取る。
- `emit` child mode の出力は fixed-seed PRNG 由来の deterministic 高
  エントロピ印字可能列 — `'L'.repeat(n)` 型の低エントロピ反復は
  禁止(o200k BPE で高圧縮され condense `when: outputTokens >
  tokenThreshold` 評価の決定性が失われる)。
- condense 発火 scenario は condense output schema(`{"summary":…}` の
  み・additionalProperties:false)適合 finish を持つ session を明示
  bind する。bulk-reader 形 text を condense job へ流す fixture は
  manifest 上の誤りとして検出する。
- session 消費規則: `run` 系 arm は 1 spawn = 1 fixture = 1 session、
  workflow 系 arm は worker job N 件に session N 件、`repairAttempts` = r
  の全失敗 arm は r+1 連続 schema 違反 session を宣言順で要求
  (direct `run` 経路にのみ fixture `policy`/`abort`/`expect` が効く —
  workflow 経路の制御は scenario `policy` overlay)。

### Record / aggregate schema v2(tests/schema.rs)

- `schemaVersion === 2` 完全一致 — v1 record は reject(v1 file は
  改変せず残るが v2 validator は受理しない)。
- `parentUsage.status ∈ {observed, unavailable}`・unavailable ⇒ `reason`
  必須・observed ⇒ `source` 必須。
- `metrics.usage` を持つ observation は `usage.source === "scripted"`
  必須 — 未標識 scripted usage の記録を拒否する。
- `environment.node` 存在は reject。`environment.ambientRustc`・
  `measuredBinary.profile` は必須。
- `aggregate.comparisons[]` entry: `claim ∈ {measured, estimated,
  unavailable}`。`unavailable` ⇒ `deltas` 全値 null + `reason` 必須。
  `estimated` ⇒ `assumptions` record 必須(`{pricing:{model,
  catalogRevision?, source, inputUsdPerMTok, outputUsdPerMTok,
  cachedInputUsdPerMTok?}, tokenizer:"o200k_base",
  boundary:"scripted-usage"}`)。`measured` ⇒ `assumptions`/`reason`
  は key 自体を emit し両方 null(null 以外の値を持つ measured entry
  は mislabel として fail-fast)。
- record は `parentUsage` block 以外に parent usage の数値 field を
  持たない — 親 token/費用削減の数値主張を schema 形で排除する。

### Aggregation 純粋関数(tests/aggregate.rs)

- `median`: 昇順 sort → 奇数 `sorted[floor(n/2)]`・偶数は
  `sorted[n/2-1]` と `sorted[n/2]` の平均。単一要素・偶数/奇数の
  boundary を pin。
- `p95`(`percentile(xs, 0.95)`): 昇順 sort → nearest-rank
  `sorted[max(0, ceil(n*f)-1)]`(補間しない — v1 parity)。
- `stableValue`: 全 obs で finite number かつ全同一ならその値、変動・
  欠測・非数を含むなら null。exitCode/errorCode/route dims・
  postprocessFailures・logRefetches の stableValue(runId は rep 毎に
  変わるため実質 null — presence/形式の pin は armExpect 側)。
- `successRate` = `success===true` の obs / 全 obs — armExpect 全項目
  一致が `success` の定義。`armExpect` 未宣言の補助 arm のみ fallback
  で `exitCode==0`(envelope arm は加えて `envelopeOk===true`)。
- warmup observation は集計除外。comparisons の delta は median と
  p95 の双方を出し、対象 dim は design §4.2 の確定集合。
- `reduction` 宣言は `reductionBytes = leftStable − rightStable`・
  `reductionRatio = (left−right)/left`(どちらか null・left==0 なら
  null)— route byte/token 軸と definition-vs-interface を同じ機構で
  表現する。
- estimated layer: `--pricing` file は load 時 fail-fast(malformed
  JSON・必須 key 欠落・非数値・未知 key は即 error)。`--pricing` 無し
  では estimated entry を一切 emit しない。estimated entry は
  `deltas.estimatedCostUsd` + assumptions echo(model・catalogRevision
  含む)を持つ。

### armExpect 照合

- `armExpect` の照合は宣言 key のみを対象とし、値の多様体(数値・
  bool・文字列・null)は等値比較。不一致項目を持つ observation は
  `success:false` で記録される(即時 abort ではない)— run 終了時に
  mismatch obs を抱える scenario を harness が失敗報告(exit≠0)。
- `envelopeOk`/`errorCode` は envelope を emit する arm のみ意味を持ち、
  raw-text/hook-JSON/harness 内 arm では null(null 期待値も宣言可)。
- 失敗は計測される観測値であって別実装ではない — 失敗 matrix は
  fixture + route/child 宣言 + expect で表現される。

### Boundary / pipeline invariants

- 新 harness と `benchmarks/` 配下(`benchmarks/results/` の
  immutable v1 artifact を除く)に `{node}`・`process.execPath`・
  `node:fs`・`environment.node` が存在しない — `git grep` で確認
  (`benchmarks/results/*.json` は旧計測 command を `node:fs` 等の
  literal で含む不変の記録であり境界対象外。placeholder は
  `{fixtureChild}`、record の environment に `node` key を持た
  ない)。
- `crates/cheaper-bench/Cargo.toml` の `cheaper-cli` dep edge は
  default feature — `dev-seam` をこの edge で宣言しない(feature
  unification で計測対象 product bin に seam が混入するため)。
  `cheaper-runtime` edge には `features = ["dev-seam"]` を宣言する。
- `cargo test -p cheaper-bench` は spawn 単位の測定実行を一切含まない
  — `[[test]]` target への測定封入・`--ignored` 運用・criterion/
  `cargo bench` 基盤を持ち込まない。測定実行は手動
  `cargo run -p cheaper-bench` のみ。
- spawn 計測は product binary の subprocess で行う — in-process
  `run_cli` による wall-time 計測を持ち込まない。
- `benchmarks/results/` の既存 v1 JSON は一切改変しない — result 書込み
  は `create_new` 非上書きで、v2 load は v1 file を reject する。
- `mise run check` は `node --test benchmarks/test/` を含まない pipeline
  で全 pass — `mise.toml` の当該行と justification comment が
  cheaper-bench 参照へ整合済み。manifest v2 flip と pipeline 行削除は
  **同一変更で着地する**(schemaVersion 1・exact-7 coverage を仮定
  する v1 suite が v2 manifest 上で走る red window を残さない)。
- 全 byte 計測は `cheaper_core::json::stringify::stringify` 経由 —
  `serde_json::to_string`/`to_string_pretty` を byte 計測に使わない
  (v1 `JSON.stringify` parity)。
- 会計分離: workflow の Worker usage は envelope `jobs[].meta` から
  一度だけ集計・direct arm は envelope + usage.jsonl 行数を別系統で
  記録・route arm は `kind:"route"`/`"route-refetch"` record 件数を
  `directUsageRecordCount`/`logRefetches` として記録 — token 集計への
  二重計上をしない。

### Doc-sync completeness

- `mise.toml` に `node --test benchmarks/test/` が残らず、comment が
  cheaper-bench の unit test を canonical 検証として指す — 削除は
  manifest v2 flip と同一変更で着地し、v1 suite が v2 manifest 上で
  走る窓を残さない。
- `AGENTS.md` の check pipeline 記述が `node --test`(scripts のみ)へ
  整合し、node tooling 列挙から `benchmarks/` が除かれる。
- `CONTRIBUTING.md` の pipeline・test コマンド一覧・node tooling 記述・
  構造一覧が `benchmarks/` = 言語中立資産 + `crates/cheaper-bench` 参照
  へ整合する — crate list に cheaper-bench 行が追加され、flag 無し
  `cargo test --workspace` が cheaper-bench の dev-seam edge 経由で
  runtime の dev-seam 依存 target を compile する非対称が注記される。
- `.gitignore` の Node tooling comment から `benchmarks/` が除かれる
  (scripts・doc-suite skills が残る Node tooling)。
- `benchmarks/README.md` が 2 段手順(`cargo build -p cheaper-cli
  --features dev-seam` + `cargo run -p cheaper-bench`)・fixture
  child/seam 説明・schema v2・`{fixtureChild}`・失敗/Route scenario・
  claim 境界・非上書き保存・`NODE_ENV=test` vestige・baseline 同等性
  migration note を含む。
- `docs/designs/node-dev-tooling-boundary-contract-issue-182.md` に
  日付付き addendum(§1.1 row 4 migrated・§5 F2 absorb 済み)がある。
- `docs/guides/workflow-usage.md` が旧 TS 測定(legacy)と新 Rust 測定を
  区別し、実測値は必須 12 scenario 全 arm の run artifact 由来のみ —
  `parentUsage.status=unavailable` 前提で親 token/費用削減の数値主張
  を文書に載せない。掲載表・prose の arm/scenario 名は v2 語彙へ整合
  し(「Node filesystem read」等の v1 表現を残さない)、spec AC を
  満たす本更新は T7 として本 change 内で着地する。
- `biome.jsonc` から `"benchmarks/**/*.mjs"` dead glob が除かれ、
  `"benchmarks/**/*.json"`・`!benchmarks/results` は残る。
- `.pre-commit-config.yaml` の biome mirror files pattern は変更不要
  (残存 `benchmarks/**/*.json` に合致し続ける — 確認のみ)。

## Examples

- `scenarios.json` が 12 required scenario を exact coverage で宣言し、
  `route-routed-exec` の `route.command` が `"benchrun --fixture-child
  emit"`(argv[1..] == `["--fixture-child","emit"]`)で
  `armExpect` に `condenseFired:false` を pin — manifest validation が
  `*` 含有 token・`cheaper` 自己 match・argv 不一致を reject。
- `worker-failure-modes` の `workflow-postprocess-failure` arm が
  `output.repairAttempts` overlay + r+1 連続 schema 違反 finish session
  を `armFakeStream` で宣言 — 全試行 schema 違反が deterministic に
  観測される。
- observation で `envelopeOk:false`・exit 7 の route-run arm は
  `armExpect` 全項目一致で `success:true`(期待どおり失敗)として
  記録され、`successRate` の母数に含まれる — harness は throw しない。
- v1 file を load しようとすると schemaVersion 不一致で reject —
  `benchmarks/results/` の旧 JSON は bytes 不変のまま残る。
- `--pricing` file に未知 key があると load 時即 error — assumptions を
  持たない estimated entry・`--pricing` 無しでの estimated entry は
  一切 emit されない。
- route-rewrite arm は各 spawn 前に `<cfg>/hook-eval/` を削除 —
  rep≥2 が dedup cache hit して route-resolve と非対称 bias にならない。
  observation 毎 fresh cfgDir の設計では rep 間で cache が共有されず
  効果は観測不能のため、これは spec'd behavior を維持する
  defense-in-depth 措置である。
- `cargo test --workspace` に cheaper-bench の manifest/fixture/schema/
  aggregate suite が含まれ、`node --test benchmarks/test/` は
  pipeline から消えている(scripts の `node --test` は残る)。

## Guarantees

- `cargo test -p cheaper-bench` は manifest・fixture・schema・aggregate
  の純粋関数検証のみを行い、`cargo test --workspace`・`mise run check`
  が再現可能に green である(spawn 測定は test 対象外)。
- result record/aggregate は claim enum の 3 層分類を fail-fast で強制し、
  unavailable 上の数値 delta・未標識 scripted usage・`environment.node`
  の再混入を schema 形で排除する。
- fixture 生成は `RunFixture::parse` の canonical schema に anchor され、
  無効 step 語彙を benchmark 実行前に検出する(JS 平行実装の drift
  解消)。
- 計測は deterministic seeded fixture を使い、token/byte 面は反復間で
  一致する(stableValue が回帰 anchor)。
- 旧 `.mjs` harness は完全に削除され、required node entrypoint は
  6→5(node pin は doc-suite 等のため不変)。
- `benchmarks/results/` の v1 file は不変で、v2 file とは混在しない
  (schemaVersion で reject)。

## Non-goals

- spawn 単位の測定実行自体の正しさ(実際の wall-time 値・scenario の
  測定結果) — `cargo test` 対象外の手動実行であり、T5/T7 の実測
  task が担保する。
- product runtime・envelope・worker schema・`usage.jsonl` record 形の
  正しさ — harness は観測側のみであり、実行契約(exit code mirror・
  log 保持・再実行防止)の回帰は既存 `route_run.rs` が担う。
- actual parent usage の取得経路(現在常に `unavailable`)と
  `observed` 配線の contract 拡張 — 将来の拡張時に本 spec を更新する。
- `cheaper usage` 側の route record 分析(issue #184 管轄)・
  `count-tokens` 部分カウント経路・`cargo test` ambient `node` 結合
  (issue #188 管轄)。
- 表示・文面側の規則(estimated を実料金として表示しない等)の機械的
  強制 — CI 強制不能面は review 対象の文書規則であり、本 spec は
  schema 形で強制可能な面のみを pin する。

## Risk

- manifest/record validation が緩むと、deterministic fixture の回帰
  anchor が失われ、「計測された」のに claim 境界を超えた主張(親
  token 削減・費用削減)が result・集計・文書に混入する。
- fixture 生成が canonical schema と drift すると(JS 平行実装と同じ
  罠)、失敗 matrix の fixture が seam 側で静かに無視され、計測対象
  scenario が意味を持たない observation を生む。
- `{node}`・`environment.node` の再混入や `cargo test` への測定封入が
  あると、runtime 非依存化と pipeline 分離という ADR の核心が崩れ、
  `mise run check` の verification 信号が毀損される。
- v1 result file の改変・混在があると append-only の記録系が壊れ、
  TS/Rust 時代の baseline 比較が不能になる。
