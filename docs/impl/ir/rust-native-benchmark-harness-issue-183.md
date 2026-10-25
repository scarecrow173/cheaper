---
id: "IMPL-xRnq2HWCVlE0nqjLTLYWWb"
type: "impl"
status: "completed"
title: "Rust-native benchmark harness — cheaper-bench (issue #183)"
created: "2026-10-25"
updated: "2026-10-25"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/183"
  changes:
    added:
      - type: "file"
        path: "crates/cheaper-bench/Cargo.toml"
      - type: "file"
        path: "crates/cheaper-bench/src/lib.rs"
      - type: "file"
        path: "crates/cheaper-bench/src/main.rs"
      - type: "file"
        path: "crates/cheaper-bench/src/cli.rs"
      - type: "file"
        path: "crates/cheaper-bench/src/manifest.rs"
      - type: "file"
        path: "crates/cheaper-bench/src/workspace.rs"
      - type: "file"
        path: "crates/cheaper-bench/src/env.rs"
      - type: "file"
        path: "crates/cheaper-bench/src/spawn.rs"
      - type: "file"
        path: "crates/cheaper-bench/src/fixture.rs"
      - type: "file"
        path: "crates/cheaper-bench/src/child.rs"
      - type: "file"
        path: "crates/cheaper-bench/src/arms/mod.rs"
      - type: "file"
        path: "crates/cheaper-bench/src/measure.rs"
      - type: "file"
        path: "crates/cheaper-bench/src/record.rs"
      - type: "file"
        path: "crates/cheaper-bench/src/aggregate.rs"
      - type: "file"
        path: "crates/cheaper-bench/src/report.rs"
      - type: "file"
        path: "crates/cheaper-bench/tests/manifest.rs"
      - type: "file"
        path: "crates/cheaper-bench/tests/fixture.rs"
      - type: "file"
        path: "crates/cheaper-bench/tests/schema.rs"
      - type: "file"
        path: "crates/cheaper-bench/tests/aggregate.rs"
      - type: "file"
        path: "docs/impl/ir/rust-native-benchmark-harness-issue-183.md"
    modified:
      - type: "file"
        path: "benchmarks/scenarios/scenarios.json"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "CONTRIBUTING.md"
      - type: "file"
        path: "benchmarks/README.md"
      - type: "file"
        path: "benchmarks/results/README.md"
      - type: "file"
        path: "docs/guides/workflow-usage.md"
      - type: "file"
        path: "docs/designs/node-dev-tooling-boundary-contract-issue-182.md"
      - type: "file"
        path: "docs/discovery/benchmark-harness-audit-issue-183.md"
      - type: "file"
        path: "docs/discovery/observability-foundation-logging-and-profiling.md"
      - type: "file"
        path: "biome.jsonc"
      - type: "file"
        path: ".gitignore"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "crates/cheaper-cli/src/workflow_run.rs"
      - type: "file"
        path: "docs/tasks/scaffold-cheaper-bench-crate-and-fixture-child.md"
      - type: "file"
        path: "docs/tasks/implement-bench-manifest-v2-scenarios-and-observation.md"
      - type: "file"
        path: "docs/tasks/implement-bench-record-schema-v2-and-aggregation.md"
      - type: "file"
        path: "docs/tasks/wire-bench-check-pipeline-and-sync-docs.md"
      - type: "file"
        path: "docs/tasks/verify-bench-smoke-run-on-real-binary.md"
      - type: "file"
        path: "docs/tasks/remove-legacy-node-benchmark-harness.md"
      - type: "file"
        path: "docs/tasks/run-full-benchmark-measurement-and-update-workflow-usage.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted:
      - type: "file"
        path: "benchmarks/run.mjs"
      - type: "file"
        path: "benchmarks/lib/cli.mjs"
      - type: "file"
        path: "benchmarks/lib/run-scenario.mjs"
      - type: "file"
        path: "benchmarks/lib/measurements.mjs"
      - type: "file"
        path: "benchmarks/lib/result-schema.mjs"
      - type: "file"
        path: "benchmarks/lib/scenarios.mjs"
      - type: "file"
        path: "benchmarks/lib/aggregate.mjs"
      - type: "file"
        path: "benchmarks/fixtures/fake-worker-stream.mjs"
      - type: "file"
        path: "benchmarks/test/aggregate.test.mjs"
      - type: "file"
        path: "benchmarks/test/measurements.test.mjs"
      - type: "file"
        path: "benchmarks/test/result-schema.test.mjs"
      - type: "file"
        path: "benchmarks/test/scenario-manifest.test.mjs"
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "benchmarks/results/251d949f-d6d6-4c02-ae61-e4b96a402901.json"
      - type: "file"
        path: "benchmarks/results/f4dfecf7-6b76-44d5-b830-f129334288ac.json"
  implements:
    - "docs/tasks/scaffold-cheaper-bench-crate-and-fixture-child.md"
    - "docs/tasks/implement-bench-manifest-v2-scenarios-and-observation.md"
    - "docs/tasks/implement-bench-record-schema-v2-and-aggregation.md"
    - "docs/tasks/wire-bench-check-pipeline-and-sync-docs.md"
    - "docs/tasks/verify-bench-smoke-run-on-real-binary.md"
    - "docs/tasks/remove-legacy-node-benchmark-harness.md"
    - "docs/tasks/run-full-benchmark-measurement-and-update-workflow-usage.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-rust-native-benchmark-harness-issue-183.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/benchmark-harness-audit-issue-183.md"
    - "docs/impl/ir/operationalize-node-dev-tooling-boundary-contract-issue-182.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-rust-native-benchmark-harness-issue-183.md"
    - "docs/designs/benchmark-harness-redesign-issue-183.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/cheaper-bench-harness-contract-issue-183.md"
  references:
    - "docs/specs/benchmark-redesign-scope-issue-183.md"
    - "docs/adr/rust-native-benchmark-harness.md"
    - "docs/designs/node-dev-tooling-boundary-contract-issue-182.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Rust-native benchmark harness — cheaper-bench (issue #183)

## Summary

issue #183 の Option B 決定(ADR-tBRMKuj5yPFMYM57an5HpS — dev-only
workspace member `crates/cheaper-bench`)と design
(DESIGN-034LUqevp1IIwiPQRmWmE4)の規定どおり、Node `.mjs` benchmark
harness(`benchmarks/` 配下 ≈1.56k 行・12 file)を Rust native harness へ
置き換えた PLAN-pxStSqvAk1Qr5rSUgRMp5w の実装記録。T1–T7 の全 task が
`done`。

- **T1 — crate skeleton + fixture child**(`dc005da`):
  `crates/cheaper-bench` を lib + bin・`publish = false` で新設し、dep
  edge(design §1.2 — `cheaper-runtime` + `dev-seam`・`cheaper-cli`
  default feature・`cheaper-core` + serde/serde_json/tempfile/uuid)・
  `main.rs` 先頭の `--fixture-child` dispatch(mode 語彙 `emit`/`fail` +
  予約語彙 `sleep`/`envdump`)・`RunFixture::parse` round-trip 必須の
  fixture 生成・workspace materialization(files・models.yaml・
  `<cfg>/policy.yaml` section merge・`{fixtureChild}` 展開・route
  block → routes.yaml/grant/routed-exec copy・observation 毎 fresh
  生成)・spawn capture(stdout/stderr・wall-time・exit code、非零で
  throw しない)を実装。
- **T2 — manifest v2 + observation layer**(`3ae916e`):
  `benchmarks/scenarios/scenarios.json` を schemaVersion 2・
  `fixtureRevision:"seeded-v2"`・12 required scenario exact
  coverage・計 35 arm へ flip。design §2.6 の fail-fast validation・
  13 impl 語彙の arm dispatch・3 種 stdout parser(envelope・raw
  text+trailer・hook JSON)・route-rewrite 前の hook-eval wipe・
  route-logs の setup/measure 2 段 spawn・preflight probe(dev-seam
  無し binary を fail-fast 診断)を実装。`mise.toml` の
  `node --test benchmarks/test/` 行削除 + justification comment
  書換を manifest flip と同一変更で着地(red window 回避 — plan
  pin どおり)。byte 計測は `cheaper_core::json::stringify` 経由に
  統一。
- **T3 — record/aggregate schema v2**(`ce413b7`): claim enum
  {measured, estimated, unavailable}・design §3.4 の 6 validation
  invariant(v1 reject・parentUsage status/理由・
  `usage.source:"scripted"`・`environment.node`/ambientRustc/profile・
  comparisons claim 別形状・数値 parent-usage field 不存在)・
  armExpect 等値照合(mismatch は `success:false` 記録 + run 終端で
  集約して非 0 exit)・median/p95 v1 parity(nearest-rank)・
  comparisons の delta/reduction emit・`--pricing` 有無で変わる
  estimated layer・`create_new` 非上書き store + v1 file reject を
  実装(`tests/schema.rs` 18・`tests/aggregate.rs` 12 で pin)。
- **T4 — doc/config sync**(`dbdd15f`): `AGENTS.md`・
  `CONTRIBUTING.md`(crate list 追加 + dev-seam 非対称注記)・
  `benchmarks/README.md`(2 段手順・fixture child/seam・schema v2・
  `{fixtureChild}`・Route/失敗 scenario・claim 境界・baseline 同等性
  migration note)・`benchmarks/results/README.md`(v1 不変・混在不可)・
  `biome.jsonc`(`benchmarks/**/*.mjs` dead glob 除去)・`.gitignore`
  (Node tooling comment から `benchmarks/` 除去)・#182 design への
  日付付き addendum(§1.1 row 4 migrated・§5 F2 absorb 済み)・
  `docs/guides/workflow-usage.md` への legacy 区別枠 + 実測
  placeholder。`.pre-commit-config.yaml` は確認のうえ変更不要と判断。
- **T5 — smoke run**(`e8aa3e4`・`98aff54`): dev-seam 有効 release
  binary へ 6 代表 scenario を `--warmups 1 --repetitions 3` で実行 —
  **84 observation 全て `success:true`**・artifact
  `benchmarks/results/251d949f-d6d6-4c02-ae61-e4b96a402901.json`
  (`schemaVersion:2`・`devSeamProbe:"passed"`)。失敗系 arm が
  `armExpect` どおり観測され、preflight probe が dev-seam 無し
  binary を `MODEL_TIER_UNAVAILABLE` + 診断付き exit 2 で fail-fast
  することを実証。armExpect mismatch 系も一時改変で確認後 revert。
- **T6 — 旧 harness 削除**(`1fbdf1c`): `benchmarks/run.mjs`・
  `lib/*.mjs`×6・`fixtures/fake-worker-stream.mjs`・
  `test/*.test.mjs`×4 の計 **12 file を `git rm`**。`benchmarks/` の
  残存は言語中立資産(`README.md`・`scenarios/`・`results/`)のみ。
  v1 result JSON 9 件は無改変。
- **T7 — 全実測 + workflow-usage.md**(`c78b5a2`): 全 12 scenario ×
  35 arm を `--warmups 2 --repetitions 15` で実測 — **595
  observation・失敗 0・全 armExpect 一致・11 comparison 全て
  `claim:"measured"`**。artifact
  `benchmarks/results/f4dfecf7-6b76-44d5-b830-f129334288ac.json`
  (`repositoryRevision` = 実測時 HEAD `1fbdf1c`)を非上書き保存・
  commit。`docs/guides/workflow-usage.md` に「Rust-era 測定(manifest
  v2 — 実測)」節を新設し runId/revision/fixtureRevision/環境を
  明記、TS 時代値は legacy 区別で保持、claim 境界(parentUsage
  unavailable 時の費用・親 token 削減主張の禁止)を宣言。併せて
  削除済み `.mjs` を `source` 列挙から外す relation repair を 2
  discovery doc(`benchmark-harness-audit-issue-183`・
  `observability-foundation-logging-and-profiling`)へ適用 —
  `provenance-unresolved-local-source` finding の解消。

## Context

承認済み plan(PLAN-pxStSqvAk1Qr5rSUgRMp5w, approved)の 7 task —

- T1: TASK-7pwH1mAayBeUw0g24iMXvR(crate skeleton + fixture child +
  materialization)
- T2: TASK-Df79tWn026fFvY79W1xIku(manifest v2 + 12 scenario +
  observation + pipeline 行削除)
- T3: TASK-IoIMnYeXx4TFTREaOLTquP(record/aggregate schema v2 +
  invariants + store)
- T4: TASK-oxOS7kvPXiqJ0LEOT1edeB(doc sync — pipeline 行削除は T2
  で着地済み)
- T5: TASK-32FiZMrj2twoTPHLYB0QgC(実 binary smoke — T6 の削除 gate)
- T6: TASK-6DBO0a5MUlapTYav2YJtS0(旧 Node harness 削除)
- T7: TASK-b6CX0aXZBAJ6ElAqC0uXxe(全実測 + workflow-usage.md 実値)

に従う。critical path は T1 → T2 → T3 → T4 → T5 → {T6 ∥ T7}。検証契約は
TSPEC-tcLTueFfBvt2PbpNF7NRof(manifest v2・schema v2 claim 境界・
aggregation invariant)。upstream chain は
DISC-qgvOWFM6MtYgrq3viqbssW(resolved)→
SPEC-hLFwwZHikKoZ5cMu7GM8oG(approved)→
ADR-tBRMKuj5yPFMYM57an5HpS(accepted)→
DESIGN-034LUqevp1IIwiPQRmWmE4(approved)で確定済み。#182 chain の
deferred follow-up F2(benchmark harness Rust 化)を本 chain が消費し、
#182 design へ addendum を write-back した(IMPL-034LTxBKOIF2rELRSlh5SJ
の Follow-ups と整合)。本 IR は issue #183 chain 内の doc 集合で閉じ、
#182 design への参照は addendum 記録と F2 来歴の位置付けのみ。

## Implementation

### Changes

- `crates/cheaper-bench/`(新設・19 file): `Cargo.toml`(publish=false・
  [lib]+[[bin]]・design §1.2 の dep edge — `cheaper-cli` edge に
  dev-seam を宣言しない feature unification 罠の回避 comment 付き)、
  `main.rs`(`--fixture-child` dispatch 先頭判定)、`cli.rs`(arg parse・
  run ループ・preflight probe・run 終端の mismatch 集約 exit)、
  `manifest.rs`(v2 serde 型 + §2.6 fail-fast validation)、
  `workspace.rs`(observation 毎の fresh materialization)、
  `env.rs`・`spawn.rs`・`fixture.rs`・`child.rs`(emit/fail +
  予約 sleep/envdump)・`arms/mod.rs`(13 impl 語彙 dispatch)・
  `measure.rs`(byte 計測は `cheaper_core::json::stringify` 経由)・
  `record.rs`(schema v2 + `validate_result` の 6 invariant)・
  `aggregate.rs`(median/p95 nearest-rank + comparisons)・
  `report.rs`(`create_new` 非上書き save + v1 reject)、
  `tests/{manifest,fixture,schema,aggregate}.rs`。
- `crates/cheaper-cli/src/workflow_run.rs`: `CompiledRun.io` field へ
  `#[cfg_attr(not(feature = "dev-seam"), allow(dead_code))]` + 理由
  comment を追加 — cheaper-bench の `cheaper-cli` default-feature dep
  edge が非 seam 側 compile を初めて通し、`io` が dev-seam 配下の env
  採取でのみ読まれる未使用 field として warning 化するため。
- `benchmarks/scenarios/scenarios.json`: schemaVersion 2 へ flip —
  `fixtureRevision:"seeded-v2"`・12 required scenario・計 35 arm・
  Route block・`armExpect`/`armFakeStream`/`comparisons`・policy
  overlay(`worker-failure-modes` は `execution.timeout:"500ms"` +
  `output.repairAttempts:1` の確定値)。
- `mise.toml`: `node --test benchmarks/test/` pipeline 行を削除し、
  justification comment を `cargo test -p cheaper-bench` canonical
  参照へ書換(T2 の manifest flip と同一変更 — red window 回避)。
- `AGENTS.md`・`CONTRIBUTING.md`: benchmark 記述を「harness は
  `crates/cheaper-bench`・`benchmarks/` は言語中立資産」へ整合、
  CONTRIBUTING crate list への cheaper-bench 行追加と flag 無し
  `cargo test --workspace` の dev-seam 非対称注記。
- `benchmarks/README.md`: 2 段手順(cargo test 回帰 + 手動 spawn
  測定)・fixture child/dev-seam 前提・schema v2・`{fixtureChild}`・
  Route/失敗 scenario・claim 境界・baseline 同等性 migration note・
  `NODE_ENV=test` vestige を記述。
- `benchmarks/results/README.md`: schemaVersion 2・v1 file 不変・
  混在不可の方針。
- `docs/guides/workflow-usage.md`: T4 で legacy 区別枠 +
  placeholder、T7 で「Rust-era 測定(manifest v2 — 実測)」節へ
  `f4dfecf7` artifact の `aggregate` 由来の実値を転記(runId・
  repositoryRevision・fixtureRevision・os/arch/cpu/rustc/
  harnessProfile 明記・掲載名は v2 語彙・claim 境界・baseline
  同等性段落)。TS 時代節は legacy 表記で保持。
- `docs/designs/node-dev-tooling-boundary-contract-issue-182.md`:
  日付付き addendum — §1.1 row 4 の benchmark 行を migrated 扱い・
  §5 F2 を本 plan で absorb 済みと記録。
- `biome.jsonc`: dead glob `"benchmarks/**/*.mjs"` を除去
  (`"benchmarks/**/*.json"`・`!benchmarks/results` は残存)。
- `.gitignore`: Node tooling comment から `benchmarks/` を除去。
- `Cargo.lock`: cheaper-bench entry 追加。
- `benchmarks/` 旧 `.mjs` harness 12 file を削除(上記 deleted 一覧)。
- `docs/discovery/benchmark-harness-audit-issue-183.md`・
  `docs/discovery/observability-foundation-logging-and-profiling.md`:
  削除済み `.mjs` を `relations.source` 列挙から除去する relation
  repair(監査対象だった file の実在性と整合)。
- `docs/tasks/` ×7: status → `done`・Done-When/Verification チェック
  完了・各 Notes へ実測観測を追記・`implemented-by` で本 IR を指す
  reciprocal edge を付与。`docs/tasks/README.md` の status cell も
  `done` へ更新。

### Decisions During Implementation

- **実測で判明した product 意味論 2 点を armExpect pin 側へ反映**
  (T2) — `route run` は child の非零 exit code を自身の process
  exit として mirror するため `route-child-nonzero-exit` の pin は
  `exitCode:7`(route record 側の child exitCode は unpinned data
  `metrics.routeChildExitCode` として記録)。workflow の provider
  call timeout は `TOOL_EXECUTION_FAILURE` に wrap されず `TIMEOUT`
  が伝播するため `worker-failure-modes/workflow-worker-timeout` の
  pin は `errorCode:"TIMEOUT"`。observation 側は不変で、manifest 側の
  期待値を product の実挙動へ合わせた修正。
- **`worker-failure-modes` の overlay 値を T2 で確定** —
  `execution.timeout:"500ms"` + `output.repairAttempts:1`。T7 の全
  実測は確定値で走り計測側では調整しない(wall-time 爆発防止の
  pre-agreed 前提 — T2 Notes)。
- **`mise.toml` の pipeline 行削除を T2 の manifest flip と同一変更で
  着地** — `benchmarks/test/` が schemaVersion 1・exact-7 coverage
  を仮定するため v2 manifest 上に残すと `mise run check` が壊れる
  red window を回避(plan の ordering pin どおり。T4 は doc-sync
  専任に縮小)。
- **`CompiledRun.io` へ cfg 付き `allow(dead_code)`** —
  cheaper-bench が `cheaper-cli` を default feature で dep するため
  非 dev-seam compile 経路が初めて有効になり、dev-seam 配下でのみ
  読まれる `io` field が dead code warning を出した。seam 側の挙動は
  不変で、理由 comment 付きの最小豁免とした。
- **`.pre-commit-config.yaml` は変更不要** — biome mirror の files
  pattern(`^(scripts|benchmarks|packages)/.*\.(mjs|json)$`)は残存
  `benchmarks/scenarios/*.json` に合致し続け、`:36` exclude が
  `benchmarks/results/` を除外するため現状維持(T4/T6 Notes に
  確認記録)。
- **crate 内 doc comment の旧 `.mjs` path 言及は意図的保持** —
  `benchmarks/lib/aggregate.mjs` parity 等の provenance 記述は
  design §5 の port mapping 表記と一致するため dangling
  reference ではなく保持(T6 Notes)。
- **T5 の artifact commit を orchestrator 側へ委譲** — smoke
  artifact は生成・検証済みだが実装 delegate の constraint で
  commit は orchestrator が実施(`e8aa3e4` で着地済み。T5 Notes)。
- **T6 は T5 smoke green 後置・T7 は本 PR で着地** — plan の
  sequencing どおり、旧 harness 削除は新 harness の smoke 成功を
  gate とし、spec AC「測定結果を workflow-usage.md へ反映し旧 TS
  測定と区別する」を満たす全実測は follow-up へ回さず本 change に
  含めた(T6/T7 は T5 後に並列)。
- **discovery `source` repair は別 commit 待ちの working-tree 変更**
  — 削除した `.mjs` を `relations.source` から除く 2 file の修正は
  実施済みだが、本 IR と同じく orchestrator の commit 対象として
  未 commit で残る。

## Related Experiments

### Adopted

なし(known-plan execution のため Experiment Log は未作成。
`metadata.experiments.adopted` は空)。

### Rejected

なし。

## Validation

- `mise exec -- cargo test -p cheaper-bench`: **exits 0**(unit 40 +
  tests/{manifest 18, fixture 11, schema 18, aggregate 12} = 99 test
  全 pass — T3 時点の内訳)。
- `mise run check`: **exits 0**(複数時点で確認 — 最終 pipeline は
  `node --test benchmarks/test/` を含まない 9 stage: biome(21 files)・
  fmt・clippy・check-version・check-doc-ids・`cargo test --workspace
  --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam`(57
  suite 0 fail)・`node --test scripts/test/`(43 pass)・
  check-lockfile・check-apm-payload)。
- T5 smoke: `cargo run --release -p cheaper-bench -- --scenario
  single-simple-tool-call --scenario route-resolve-and-rewrite
  --scenario route-routed-exec --scenario route-condense-and-refetch
  --scenario route-child-nonzero-exit --scenario
  worker-failure-modes --warmups 1 --repetitions 3` exits 0 —
  **84/84 observation `success:true`**・artifact
  `benchmarks/results/251d949f-d6d6-4c02-ae61-e4b96a402901.json` は
  `schemaVersion:2`・`fixtureRevision:"seeded-v2"`・
  `devSeamProbe:"passed"`・`environment` に `node` field 無し
  (事後 grep: schemaVersion×85・fixtureRevision×85・`"node"`×0・
  `source:"scripted"`×44)。
- preflight probe 実証: dev-seam 無し `target/debug/cheaper` を
  `--binary` で指した run が `MODEL_TIER_UNAVAILABLE` 観測 +
  診断・build 手順付きで **exit 2 fail-fast**(result file 非生成)。
- armExpect mismatch 系実証: `route-routed-exec` の
  `direct-child-exec` pin を一時的に 99 へ改変 → obs が
  `success:false`・`exit=0 observed vs expected 99` detail 付き・
  run 終了 exit 2。確認後 manifest revert(diff 無し)。
- T7 全実測: `cargo run --release -p cheaper-bench -- --warmups 2
  --repetitions 15`(binary 直起動 `--binary target/release/cheaper.exe`)
  exits 0 — **595 observation・失敗 0・全 armExpect 一致・11
  comparison 全て `claim:"measured"`**・artifact
  `benchmarks/results/f4dfecf7-6b76-44d5-b830-f129334288ac.json` は
  `create_new` 非上書き・`repositoryRevision:
  1fbdf1c48a5df5f2f2eb082d7475b3cc97ef55e8`(実測時 HEAD)・
  `conditions.selectedScenarios` が required 12 件を列挙・
  `environment` = windows/x86_64/16cpu/`rustc 1.99.0`/
  `harnessProfile:"release"`。
- 旧 harness 削除確認: `git ls-files -- 'benchmarks/**/*.mjs'` = 0 件。
  `git grep '{node}\|process\.execPath\|node:fs\|environment\.node'` は
  `benchmarks/`(results 除外)・`crates/cheaper-bench/` で live 参照
  0 件(残存 8 hit は v1→v2 境界を記述する prose/doc comment の
  意図的言及)。
- v1 artifact 不変: `git status --porcelain benchmarks/results/` が
  新規 runId JSON のみ・既存 v1 JSON 9 件に diff 無し(T5・T6・T7
  各時点で確認)。
- `workflow-usage.md` 掲載値は `f4dfecf7` の
  `aggregate.arms[].medianWallTimeMs/p95WallTimeMs/stableMetrics` と
  照合済み — v2 語彙・legacy 区別・claim 境界(費用・親 token
  削減の主張なし・estimated 非掲載)を遵守。
- `node scripts/check-doc-ids.mjs`: exits 0(1,225+ markdown files —
  本 IR 追加後も再実行で 0)。
- doc audit: `audit_docs.js --type task` で本 chain 7 file の
  findings 0・`docs/tasks/README.md` status cell 整合(T7 の収束
  処理 — 全 findings 144 件は非 blocking warning・baseline 149 →
  144 に減少)。`--type plan` で本 plan file の findings 0(index
  title mismatch warning 1 件は pre-existing・blocking:false)。
  discovery audit で 2 件の `provenance-unresolved-local-source` を
  relation repair で解消。impl type は audit script の `--type`
  列挙外のため IR 自体は lint 対象外(front matter 形は impl
  contract — `IMPL-<22>`・status `completed`・`metadata`
  必須 — に準拠)。
- `git diff main...HEAD --name-status`: 追加 19(crate)+2 artifact +
  chain doc・削除 12(.mjs)・product runtime の実行 code 差分は
  `workflow_run.rs` の `#[cfg_attr]` + comment のみで、product
  挙動・envelope・worker schema・`usage.jsonl` record 形は一切
  不変(harness は観測側のみ — spec Out of Scope どおり)。

証跡 commit: `dc005da`(T1 crate skeleton)→ `3ae916e`(T2 manifest
v2 + observation + pipeline 行削除)→ `ce413b7`(T3 record/aggregate
v2)→ `dbdd15f`(T4 doc sync)→ `e8aa3e4`・`98aff54`(T5 smoke +
task done)→ `1fbdf1c`(T6 旧 harness 削除)→ `c78b5a2`(T7 全実測
+ workflow-usage.md)。2 件の discovery source repair と本 IR は
orchestrator commit 対象。

## Risks

- wall-time 絶対値は実行環境依存 — deterministic seeded fixture により
  token/byte 面は反復間一致するが、記録値は環境(Windows・16cpu・
  `rustc 1.99.0`)付きの観測として扱う。`workflow-usage.md` の
  baseline 同等性段落と `benchmarks/README.md` migration note が
  v1↔v2 直接比較不可の guard。
- `parentUsage.status` は常に `unavailable` — actual parent usage の
  取得経路は未実装(out of scope)のため、費用・親 token 削減の主張は
  schema v2 の claim 規則と workflow-usage.md の明記段落で抑制済み。
- 旧 `.mjs` harness の復帰は不可能ではないが、v1 manifest/test は
  schema v2 と非互換 — 復旧する場合は本 chain の revert unit(plan
  Risks §10 の commit 群単位)で行う。
- `benchmarks/results/` に v1(9 件)と v2(2 件)が混在 — v2 loader
  が v1 を schemaVersion で reject し `create_new` が非上書きを保証
  するが、v1 file を手編集すると provenance が失われる(immutable
  前提は results/README.md に明記済み)。
- `.pre-commit-config.yaml` の biome mirror に deprecated `.mjs` を
  stage すると "No files were processed" exit 1 になり得る(T4
  Notes)— 対象 file は削除済みで ACMR filter 下の削除 commit には
  影響しないが、将来 `benchmarks/` へ `.mjs` を再導入した場合に
  顕在化する。

## Follow-ups

- **issue #184** — `cheaper usage` 側の route record
  (`kind:"route"`/`route-refetch`)詳細分析。workflow-usage.md には本
  harness の route scenario 観測値のみを掲載済み。
- **issue #188**(#182 F1)— `cargo test` ambient `node` 結合の
  fixture binary repoint。`benchmarks/` の Node 依存は本 chain で
  解消済みで、残存する ambient 結合は test 経路側。
- **`--pricing` estimated run** — opt-in layer は実装済み(T3)だが
  本 chain では run しない判断(T7 Notes)。必要時に別 run で実施。
- **F3/F4**(#182 chain の残 deferred follow-up)—
  `gen-glob-tables.cjs` oracle 再設計と `check-*.mjs`・`node --test`
  系の Rust/native 化は各 trigger 発火時の別 chain が担う(#182 plan
  追跡表どおり gate 未達のまま)。
- PR open・merge・issue #183 の close 判断は
  orchestrator/maintainer 所有。`benchmarks/logs/` は ignore 対象で
  commit しない。
