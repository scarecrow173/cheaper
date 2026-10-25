---
id: "PLAN-pxStSqvAk1Qr5rSUgRMp5w"
type: "plan"
status: "approved"
title: "Implement Rust-native benchmark harness — cheaper-bench (issue #183)"
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
        path: "docs/plans/implement-rust-native-benchmark-harness-issue-183.md"
      - type: "file"
        path: "docs/test-specs/cheaper-bench-harness-contract-issue-183.md"
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
        path: "biome.jsonc"
      - type: "file"
        path: ".gitignore"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "docs/specs/benchmark-redesign-scope-issue-183.md"
      - type: "file"
        path: "docs/adr/rust-native-benchmark-harness.md"
      - type: "file"
        path: "docs/designs/benchmark-harness-redesign-issue-183.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
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
        path: "benchmarks/results/"
  implements:
    - "docs/specs/benchmark-redesign-scope-issue-183.md"
  implemented-by:
    - "docs/tasks/scaffold-cheaper-bench-crate-and-fixture-child.md"
    - "docs/tasks/implement-bench-manifest-v2-scenarios-and-observation.md"
    - "docs/tasks/implement-bench-record-schema-v2-and-aggregation.md"
    - "docs/tasks/wire-bench-check-pipeline-and-sync-docs.md"
    - "docs/tasks/verify-bench-smoke-run-on-real-binary.md"
    - "docs/tasks/remove-legacy-node-benchmark-harness.md"
    - "docs/tasks/run-full-benchmark-measurement-and-update-workflow-usage.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/implement-workflow-orchestration-benchmarks.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/benchmark-redesign-scope-issue-183.md"
    - "docs/designs/benchmark-harness-redesign-issue-183.md"
    - "docs/adr/rust-native-benchmark-harness.md"
  derived-by:
    - "docs/tasks/scaffold-cheaper-bench-crate-and-fixture-child.md"
    - "docs/tasks/implement-bench-manifest-v2-scenarios-and-observation.md"
    - "docs/tasks/implement-bench-record-schema-v2-and-aggregation.md"
    - "docs/tasks/wire-bench-check-pipeline-and-sync-docs.md"
    - "docs/tasks/verify-bench-smoke-run-on-real-binary.md"
    - "docs/tasks/remove-legacy-node-benchmark-harness.md"
    - "docs/tasks/run-full-benchmark-measurement-and-update-workflow-usage.md"
  verifies: []
  verified-by:
    - "docs/test-specs/cheaper-bench-harness-contract-issue-183.md"
  references:
    - "docs/discovery/benchmark-harness-audit-issue-183.md"
    - "docs/designs/node-dev-tooling-boundary-contract-issue-182.md"
    - "benchmarks/README.md"
    - "benchmarks/scenarios/scenarios.json"
    - "benchmarks/results/README.md"
    - "docs/guides/workflow-usage.md"
    - "mise.toml"
    - "biome.jsonc"
    - ".pre-commit-config.yaml"
    - "Cargo.toml"
  defers: []
  deferred-by: []
---

# Implement Rust-native benchmark harness — cheaper-bench (issue #183) Implementation Plan

## Goal

ADR-tBRMKuj5yPFMYM57an5HpS(Option B — dev-only workspace member
`crates/cheaper-bench`)と design(DESIGN-034LUqevp1IIwiPQRmWmE4)の規定どおり、
現行 Node benchmark harness(`benchmarks/` 配下 `.mjs` ≈1.56k 行)を Rust
native harness へ置き換える。具体的には (1) `crates/cheaper-bench`(lib +
bin + unit tests・publish=false)を新設し runner・arm 実装・計測・集計・
schema 検証を Rust へ移植し、(2) scenario manifest を schemaVersion 2
(Route・失敗 matrix・`{fixtureChild}`・claim 宣言)へ拡張し 12 required
scenario を実装し、(3) result record/aggregate を schemaVersion 2
(claim enum・validation invariant・`environment.node` 除去)へ移行し、
(4) `node --test benchmarks/test/` を `cargo test -p cheaper-bench` へ
吸収させる `mise run check` 統合と文書同期を行い、(5) 旧 `.mjs` harness
を削除し、(6) 現行 Rust binary で再測定して `workflow-usage.md` を
Rust-era 実測へ更新する。

## Scope

7 task の直列 + 終端 2 並列 — crate 内部の層依存(skeleton →
manifest/arm/observation → record/aggregate)が直列を強制し、旧 harness
削除(T6)と全 scenario 実測(T7)は harness 完成後に互いに独立する。

In scope(spec In Scope + design In Scope の転記):

- `crates/cheaper-bench` の crate 構成(design §1.1 の module 一覧)・dep
  edge(§1.2 — `cheaper-runtime` + `dev-seam`・`cheaper-cli` default
  feature・`cheaper-core`)・CLI surface(§1.3)・fixture child 自己再実行
  (§1.4 — `--fixture-child <mode>`、mode 語彙 emit/fail + 先行宣言
  sleep/envdump)。
- `benchmarks/scenarios/scenarios.json` の manifest v2(schemaVersion 2・
  `fixtureRevision:"seeded-v2"`・12 required scenario・arm 語彙・route
  block・`armExpect`/`armFakeStream`/`comparisons`・policy overlay)と
  fail-fast validation(design §2)。
- result record/aggregate schemaVersion 2 — claim enum
  {measured, estimated, unavailable}・6 件の validation invariant・
  `environment.node` 拒否・median/p95 v1 parity・`create_new` 非上書き
  保存(design §3・§4)。
- `benchmarks/test/*.mjs` 4 suite から `cargo test -p cheaper-bench` の
  4 suite(manifest/fixture/schema/aggregate)への回帰 test 移行と
  `mise.toml` check pipeline の整合(design §5)。
- design §6 doc-sync list 全件(mise.toml・AGENTS.md・CONTRIBUTING.md・
  benchmarks/README.md・results/README.md・workflow-usage.md・#182
  design addendum・biome.jsonc・`.pre-commit-config.yaml` 確認)+
  `.gitignore` の Node tooling comment 整合(review 追加分)。
  ※ mise.toml の pipeline 行削除自体は T2 の manifest v2 flip と
  同一変更で行い、T4 は残りの doc-sync を担う。
- 旧 `.mjs` harness 削除(run.mjs・lib 6・fixtures・test 4)。
- dev-seam 有効 product binary を用いた smoke run と全 12 scenario 実測、
  `workflow-usage.md` の Rust-era 実測への更新(TS 時代値は legacy 区別)。

Out of scope(spec/design Out of Scope の転記):

- 実 provider・実 Cursor/Copilot・外部 CLI・Ollama を前提にした計測、
  host approval bypass の保証。
- product runtime・envelope・worker schema・`usage.jsonl` record 形の変更
  (harness は観測側のみを実装する)。
- `fixture.rs` への `Serialize` 付与(ADR が見送った将来 refinement)。
- `cargo test` ambient `node` 結合の fixture binary 化 — issue #188(F1)
  の管轄。actual parent usage の取得経路(現在常に unavailable)。
- `mise.toml` の `node` pin 除去(#182 Option A の gate 未達)。
- `cheaper usage` 側の route record 分析 — issue #184 の管轄。
- `count-tokens` の部分カウント経路(`CHEAPER_COUNT_MAX_BYTES`)。
- `[[test]]` target への測定封入・`--ignored` 運用・criterion/`cargo
  bench` 基盤(ADR が不採用とした形態)— spawn 単位の測定実行は
  `cargo test` 対象外(手動実行)のまま。

## Upstream Documents

- `docs/specs/benchmark-redesign-scope-issue-183.md`
  (SPEC-hLFwwZHikKoZ5cMu7GM8oG, approved) — 要件・Requirements・
  Acceptance Criteria。拘束力のある contract。
- `docs/adr/rust-native-benchmark-harness.md`
  (ADR-tBRMKuj5yPFMYM57an5HpS, accepted) — Option B 決定・dep edge
  制約・fixture 供給方式の確定・scenario 意味論移行 note。
- `docs/discovery/benchmark-harness-audit-issue-183.md`
  (DISC-qgvOWFM6MtYgrq3viqbssW, resolved) — 現行 harness ≈1,973 行の
  監査・7 scenario/20 arm・schema v1・Rust 側資産。
- 依存判断(別 chain・references のみ):
  `docs/designs/node-dev-tooling-boundary-contract-issue-182.md`
  (F2 が本 plan で実施される — §6 addendum で migrated 扱いにする)。

## Design Inputs

- `docs/designs/benchmark-harness-redesign-issue-183.md`
  (DESIGN-034LUqevp1IIwiPQRmWmE4, approved) — crate layout(§1)・
  manifest v2(§2)・result schema v2(§3)・aggregation(§4)・check
  統合(§5)・doc-sync(§6)・Non-Functional Constraints が本 plan の
  authoritative HOW。本 plan は design を implementation order へ転記
  したもので独自判断を足さない。

実装順序上の pin(design に反さない範囲の裁定 — review 用に明記):

- **manifest 型定義は T1・validation は T2** — `workspace.rs` の
  materialization(policy.yaml merge・route block → routes.yaml/grant/
  routed-exec copy)が scenario manifest 型を消費するため、serde 型定義・
  load・構造的な最低限の check は T1 に置き、`REQUIRED_SCENARIO_IDS`
  exact coverage・arm/route/comparisons の domain 規則(§2.6)は T2 で
  実装する。中間状態でも compile/test が成立する境界。
- **smoke run と全実測を分離(T5/T7)** — T5 は代表 subset(design §2.2
  の 12 scenario 中、workflow/direct・route・失敗系・hook parser/exit
  mirror を跨ぐ `single-simple-tool-call`・`route-resolve-and-rewrite`・
  `route-routed-exec`・`route-condense-and-refetch`・
  `route-child-nonzero-exit`・`worker-failure-modes` の 6 scenario、
  `--warmups 1 --repetitions 3`)で「harness が実 binary を spawn し
  valid な schema v2 artifact を出す」ことを検証し T6 の削除 gate と
  する。全 12 scenario × 全 arm の実測(`--warmups 2 --repetitions 15`)
  と `workflow-usage.md` への実値転記は T7 とする — 長い wall-time を
  持つ run を削除 gate から切り離す。
- **`workflow-usage.md` は 2 段更新** — T4 で「実測 pending」の placeholder
  + 旧 TS 値の legacy 区別枠のみ書き、T7 で実値を転記する
  (design §6「本 doc 時点では値を書かない」の運用)。
- **`mise.toml` の pipeline 行削除は T2 の manifest v2 flip と同一
  変更** — `benchmarks/test/` が schemaVersion 1・exact-7 v1
  coverage を仮定するため、v2 manifest 上に `node --test
  benchmarks/test/` が残ると `mise run check` が壊れる(red
  window)。行削除(:73)+ justification comment 書換(:61-62)を
  T2 scope へ置き、T4 は doc-sync 専任とする。
- **T6 は T5 後置** — `node --test benchmarks/test/` の pipeline 行は T2
  で除去済み、旧 `.mjs` file 削除は新 harness の smoke が green になって
  から行う(delegate 指定の ordering constraint)。
- **T7 は本 PR で着地必須** — spec AC「測定結果を workflow-usage.md
  へ反映し、旧 TS 測定と新 Rust 測定を区別する」は T7 の実測
  artifact でのみ満たされるため、T7 を follow-up へ回さず本
  change に含める(T6 と並列可だが PR scope から外さない —
  T7 未完では change は incomplete)。

## File Map

| Path | Change | Responsibility | Notes |
| --- | --- | --- | --- |
| `crates/cheaper-bench/Cargo.toml` | add | publish=false・[lib]+[[bin]]・dep edge(cheaper-core・cheaper-cli default・cheaper-runtime+dev-seam・serde/serde_json/tempfile/uuid) | design §1.2。`cheaper-cli` edge に dev-seam を宣言しない(feature unification 罠) |
| `crates/cheaper-bench/src/lib.rs` | add | module root — harness 全責務を lib 側へ | design §1.1 |
| `crates/cheaper-bench/src/main.rs` | add | 薄い entry — `--fixture-child` dispatch 先頭判定 → CLI parse → run | design §1.1/§1.4 |
| `crates/cheaper-bench/src/cli.rs` | add | arg parse(`--scenario` 複数・`--warmups`/`--repetitions`/`--output`/`--binary`/`--pricing`)・run ループ・preflight probe | design §1.3 |
| `crates/cheaper-bench/src/manifest.rs` | add | manifest v2 serde 型(T1)+ fail-fast validation(T2) | design §2.1/§2.6 |
| `crates/cheaper-bench/src/workspace.rs` | add | temp workspace 生成(files・`.cheaper/models.yaml`・`<cfg>/policy.yaml` section merge・routes.yaml・routed-exec copy・route materialize) | design §2.4・observation 毎新規生成(Non-Functional) |
| `crates/cheaper-bench/src/env.rs` | add | spawn env 組立(PATH・HOME/USERPROFILE・NODE_ENV=test・CHEAPER_CONFIG_DIR・CHEAPER_TEST_FAKE_STREAM・SystemRoot/TEMP/TMP・CHEAPER_LOG*・armEnv) | v1 parity allowlist |
| `crates/cheaper-bench/src/spawn.rs` | add | product binary / fixture child の blocking spawn・stdout/stderr capture・wall-time・exit code 保持・per-mode stdout parser(envelope JSON / route-run raw text+trailer / hook response JSON) | nonzero exit は throw しない(design §7) |
| `crates/cheaper-bench/src/fixture.rs` | add | RunFixture JSON 生成(`serde_json::json!`/template + `RunFixture::parse` round-trip・profile 駆動 session・`{fixture:{…}}` passthrough・`armFakeStream`) | ADR fixture 供給方式の確定 |
| `crates/cheaper-bench/src/child.rs` | add | fixture child mode(emit=seeded 高エントロピ出力・fail=marker+exit code・sleep/envdump は先行宣言のみ) | design §1.4/§2.3 |
| `crates/cheaper-bench/src/arms/mod.rs` | add | arm impl 語彙 13 種(workflow×3・direct-filesystem・direct-worker-tool-call・direct-worker-sequence・full-definition・compact-interface・route-resolve・route-rewrite・route-run・route-logs・direct-child-exec) | design §2.3 |
| `crates/cheaper-bench/src/measure.rs` | add | envelope/record 計測(retained bytes・critical path・usage 集計・usage.jsonl record 走査・route 系 field・hook-eval wipe・payload mint・route-logs setup) | design §2.3-§2.5・§4.4 |
| `crates/cheaper-bench/src/record.rs` | add | result schema v2 型 + validate(6 invariant)+ armExpect 照合 | design §3.3-§3.4 |
| `crates/cheaper-bench/src/aggregate.rs` | add | median/p95(v1 parity)・stableValue・successRate・comparisons・reduction・estimated layer | design §4 |
| `crates/cheaper-bench/src/report.rs` | add | result JSON 組立て + `create_new` 書込み(`benchmarks/results/<runId>.json`) | design §3.5 |
| `crates/cheaper-bench/tests/{manifest,fixture,schema,aggregate}.rs` | add | 旧 `benchmarks/test/*.mjs` 4 suite の Rust 移植(manifest 検証・fixture round-trip・schema invariant・aggregate 純粋関数) | design §5 — spawn 測定は test 対象外 |
| `benchmarks/scenarios/scenarios.json` | modify | manifest v2(`schemaVersion:2`・`fixtureRevision:"seeded-v2"`・12 required scenario・`{fixtureChild}`・route/armExpect/armFakeStream/comparisons 宣言) | design §2.2 の 12 件 exact coverage |
| `benchmarks/scenarios/saved-workflow.json` | 維持 | saved arm の source(変更なし) | v1 parity |
| `Cargo.lock` | modify | cheaper-bench package entry(新規 workspace member) | `cargo fetch`/`build` で再生成・commit 必須(check-lockfile) |
| `mise.toml` | modify | :73 `node --test benchmarks/test/` 削除・:61-62 justification comment を cheaper-bench 参照へ書換 | design §5 — **T2 scope**(manifest v2 flip と同一変更 — v1 suite が v2 manifest 上で走る red window を防ぐ) |
| `AGENTS.md` | modify | check pipeline 記述(node --test scripts のみ)・node tooling 列挙から `benchmarks/` 除去 | design §6 |
| `CONTRIBUTING.md` | modify | pipeline・test コマンド一覧・node tooling 記述・構造一覧(crate list への cheaper-bench 行追加 + benchmarks/ 行の言語中立資産化)・flag 無し cargo test --workspace の dev-seam 非対称注記 | design §6 |
| `benchmarks/README.md` | modify | Rust harness 手順(2 段 build)→fixture child/seam・schema v2・`{fixtureChild}`・失敗/Route scenario・claim 境界・非上書き保存・baseline migration note へ全面書換 | design §6 |
| `benchmarks/results/README.md` | modify | schemaVersion 2・v1 file 不変・混在不可の方針追記 | design §6 |
| `docs/guides/workflow-usage.md` | modify | T4: legacy 区別 + placeholder。T7: 実測値転記 | design §6 |
| `docs/designs/node-dev-tooling-boundary-contract-issue-182.md` | modify | 日付付き addendum(§1.1 row 4 migrated・§5 F2 が #183 に absorb 済み) | design §6 |
| `biome.jsonc` | modify | `"benchmarks/**/*.mjs"` dead glob 行除去(`benchmarks/**/*.json`・`!benchmarks/results` は維持) | design §6 |
| `.gitignore` | modify | :5 Node tooling comment から `benchmarks/` 除去 | review 追加 — benchmark harness Rust 化で stale 化 |
| `.pre-commit-config.yaml` | 確認のみ | biome mirror files pattern は残存 `benchmarks/**/*.json` に合致 — 変更不要 | design §6 |
| `benchmarks/{run.mjs,lib/*.mjs(6),fixtures/fake-worker-stream.mjs,test/*.mjs(4)}` | delete | 旧 Node harness 全撤去(T6) | T5 green 後置の ordering constraint |
| `benchmarks/results/<runId>.json` | generated | T5 smoke artifact + T7 全実測 artifact(create_new・commit 対象・v1 file 不変) | design §3.5/§6 |
| upstream chain doc front-matter ×3 | modify (relations のみ) | spec/ADR/design への derived-by・verified-by・implemented-by reciprocal link | doc-workflow 慣行(#180/#182 chain と同型)— plan 作成時に適用済み、実装 task 対象外 |

## Implementation Sequence

### T1 — cheaper-bench crate skeleton + fixture child + materialization

- [ ] `crates/cheaper-bench/` 新設: Cargo.toml(dep edge 確定 +
      `cheaper-cli` edge の dev-seam 禁止 comment)・lib.rs・main.rs
      (child dispatch 先頭判定)
- [ ] manifest.rs の v2 serde 型 + load(validation 本体は T2)・
      workspace.rs(files・models.yaml・policy.yaml section merge・
      routes.yaml・routed-exec copy・route block materialize)・env.rs
      (v1 parity allowlist + armEnv)・spawn.rs(blocking Command・
      stdout/stderr capture・wall-time・exit code — throw しない)
- [ ] fixture.rs(`json!` 組立て + `RunFixture::parse` round-trip・
      profile 駆動 session 生成・`{fixture:{…}}` passthrough merge)
- [ ] child.rs(`--fixture-child <mode>` dispatch・emit=seeded
      deterministic 高エントロピ byte 列・fail=marker+exit・
      sleep/envdump は宣言のみ)
- [ ] cli.rs skeleton(arg parse・binary 解決・scenario×arm×rep ループ
      骨格 — arm 実装接続は T2)
- [ ] `tests/fixture.rs` — 生成 fixture の RunFixture::parse
      round-trip・step 語彙・profile 生成 + workspace.rs の
      `#[cfg(test)]` unit test(policy.yaml section merge:
      workflowMaxJobs・policy overlay・command.grants の共存 merge
      — spawn-free 純粋関数)
- [ ] `cargo test -p cheaper-bench` + workspace compile/fmt/clippy green・
      Cargo.lock commit

### T2 — manifest v2 + 12 scenario + spawn/observation/parsers + pipeline 行削除

- [ ] manifest.rs validation 完成(§2.6 全 invariant —
      REQUIRED_SCENARIO_IDS exact coverage・armBase/armEnv/armExpect/
      armFakeStream domain・route block 規則・policy overlay 既知
      section・comparisons arm/field 照合)
- [ ] `benchmarks/scenarios/scenarios.json` を schemaVersion 2・12
      required scenario(design §2.2 の表)へ移行
- [ ] `mise.toml` — `node --test benchmarks/test/` 行(:73)削除 +
      justification comment(:61-62)を cheaper-bench unit test 参照へ
      書換(**manifest flip と同一変更** — v1 suite が v2 manifest
      上で走る red window を残さない)
- [ ] arms/mod.rs の 13 impl 語彙 + measure.rs(envelope/worker/utility・
      hook response JSON・route-run raw text+trailer regex の 3 parser・
      usage.jsonl record 走査・route 系 field・hook-eval wipe・
      `payload::encode` mint 導出規則・route-logs setup spawn)
- [ ] cli.rs 完成 — preflight probe(§1.3 bulk-reader schema 適合
      fixture)・arm dispatch・observation 毎 fresh ws/cfgDir
- [ ] `tests/manifest.rs` — required coverage・armBase/armEnv/armExpect
      整合・route/fakeStream 検証の fail-fast 確認
- [ ] `cargo test -p cheaper-bench` green + `mise run check` green
      (node --test は scripts のみ)

### T3 — record/aggregate schema v2 + claim invariants + store

- [ ] record.rs — schema v2 型・validate(§3.4 の 6 invariant:
      schemaVersion==2・parentUsage status/理由・usage.source scripted・
      environment.node 拒否 + ambientRustc/profile 必須・comparisons
      claim enum + claim 別形状・数値 parent-usage field 不存在)・
      armExpect 照合(宣言 key のみ等値比較・mismatch → success:false・
      run 終了時 mismatch 集計で exit≠0)
- [ ] aggregate.rs — median/p95 v1 parity algorithm・stableValue・
      successRate・arm 集計・manifest comparisons + reduction・
      estimated layer(`--pricing` file fail-fast load・assumptions
      echo・`deltas.estimatedCostUsd`・無指定時 emit 無し)
- [ ] report.rs — result JSON 組立て + `create_new(true)` 非上書き
      書込み・v1 file reject
- [ ] `tests/schema.rs`(record/aggregate v2 validation・claim
      invariant)+ `tests/aggregate.rs`(median/p95/stableValue/
      comparisons の boundary case)
- [ ] `cargo test -p cheaper-bench` green

### T4 — doc sync(pipeline 行削除は T2 で着地済み)

- [ ] `AGENTS.md`・`CONTRIBUTING.md`(crate list への cheaper-bench
      行追加 + dev-seam 非対称注記)・`benchmarks/README.md` 全面書換・
      `benchmarks/results/README.md`・`biome.jsonc` dead glob・
      `.gitignore` Node tooling comment・#182 design addendum・
      `docs/guides/workflow-usage.md` placeholder(legacy 区別 +
      実測 pending 枠)
- [ ] `.pre-commit-config.yaml` の確認(変更不要 — 記録のみ)
- [ ] `mise run check` green(node --test は scripts のみ — mise.toml
      行削除は T2 で着地済み)

### T5 — smoke run(実 binary・subset)

- [ ] `cargo build --release -p cheaper-cli --features dev-seam` +
      `cargo run --release -p cheaper-bench -- --scenario
      single-simple-tool-call --scenario route-resolve-and-rewrite
      --scenario route-routed-exec --scenario
      route-condense-and-refetch --scenario route-child-nonzero-exit
      --scenario worker-failure-modes --warmups 1 --repetitions 3`
- [ ] schemaVersion 2 artifact が `benchmarks/results/` へ非上書き保存・
      artifact が v2 validator を pass・失敗系 arm が armExpect どおり
      観測される
- [ ] preflight probe が seam 無し binary を fail-fast で検出することを
      別途確認(dev-seam 無し debug build を `--binary` で指す)

### T6 — 旧 Node harness 削除(T1-T5 green 後置)

- [ ] `benchmarks/run.mjs`・`lib/*.mjs`×6・`fixtures/fake-worker-stream.
      mjs`・`test/*.mjs`×4 を削除
- [ ] boundary grep: `benchmarks/`(`results/` 除外 — immutable v1
      artifact)配下に `{node}`・`process.execPath`・`node:fs` が
      残らず、tracked `.mjs` file が `git ls-files` で 0 件
- [ ] `mise run check` green(pipeline に node --test benchmarks/test/
      が無い状態で全 pass)

### T7 — 全 12 scenario 実測 + workflow-usage.md 実値(本 PR で着地必須)

- [ ] 全 12 scenario × 全 arm(~35 arm・600-900 spawn・~10-40 分 +
      初回 release build)を `--warmups 2 --repetitions 15` で実測し
      artifact を `benchmarks/results/` へ保存・commit — `--scenario`
      batch 分割 run 可(各 run が固有 runId artifact・文書は複数
      runId を cite 可)
- [ ] `docs/guides/workflow-usage.md` の placeholder を実測値へ置換・
      TS 時代値は legacy として区別(削除しない)・`fixtureRevision`
      /binary/revision を明記・arm 名/prose は v2 語彙へ更新
- [ ] parentUsage unavailable 前提で親 token/費用削減の数値主張を
      文書に載せないことを確認
- [ ] chain 収束処理: 全 task status→done・docs/tasks/README.md cell
      更新・audit_docs findings 0(orchestrator 巻取り)

## Dependency Graph

| Slice | Depends on | Blocks |
| --- | --- | --- |
| T1 crate skeleton | — | T2 |
| T2 manifest/scenarios/observation + pipeline 行削除 | T1 | T3 |
| T3 record/aggregate | T2 | T4 |
| T4 doc sync(pipeline 整合は T2 で着地済み) | T3 | T5・T7 |
| T5 smoke run(subset) | T4 | T6・T7 |
| T6 旧 harness 削除 | T5 | — |
| T7 全実測 + workflow-usage.md | T5(T4 経由で placeholder 着地済み) | — |

critical path は T1 → T2 → T3 → T4 → T5 → {T6 ∥ T7}。T1-T3 は crate 内部
の層依存(型 → 検証/arm → record)により直列必須。T6 と T7 は file 面が
交差しない(削除は `benchmarks/*.mjs`、実測は `benchmarks/results/` +
`workflow-usage.md`)ため T5 後に並列可。

## Dependencies and Risks

### Risk Register

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | `cheaper-cli/dev-seam` を cheaper-bench の dep edge で宣言し feature unification で計測対象 `cheaper` bin に seam が混入 | med | high | edge は default feature 固定 + Cargo.toml comment + benchmarks/README で固定。TSPEC の boundary grep(Cargo.toml に `cheaper-cli.*dev-seam` が無い)で機械検査 |
| 2 | byte 計測が serde_json の number formatting で v1 `JSON.stringify` parity とずれる | med | med | 全 byte 計測を `cheaper_core::json::stringify::stringify` 経由に限定(design §7)。schema test に parity case を含める |
| 3 | `benchmarks/test/` が schemaVersion 1・exact-7 v1 coverage を仮定するため、T2 が manifest を v2 へ flip した時点で pipeline に `node --test benchmarks/test/` が残ると `mise run check` が壊れる(red window)。逆順で dead file が lint/参照に残る対称 risk もある | med | med | mise.toml の行削除(:73)+ justification comment 書換(:61-62)を **T2 の manifest flip と同一変更**で行い T4 は doc-sync 専任に縮小 — pipeline-removal-lands-with-manifest-flip ordering。`.mjs` file 削除自体は従来どおり T6(T5 smoke green 後置)。T2/T6 Done-When に `mise run check` green を要求 |
| 4 | preflight probe が seam 無し binary を検出できず実 provider 経路へ進み auth error/遅延で失敗 | med | med | design §1.3 の probe 構成(bulk-reader output schema 適合 finish の session・`--cwd <probe ws>`・envelope ok:true 確認)を忠実実装。probe fixture は RunFixture::parse round-trip + bulk-reader schema 適合を test で pin |
| 5 | route-rewrite arm が `<cfg>/hook-eval/` dedup cache を消さず rep≥2 が cache hit して route-resolve と非対称 bias | low | med | design §2.3 どおり各 spawn 前に hook-eval dir を削除。observation 毎 fresh cfgDir では rep 間 cache 共有が起きず効果は観測不能だが、spec'd behavior として維持する defense-in-depth(test-spec に pin) |
| 6 | armExpect mismatch を即時 abort すると failure matrix の「期待どおり失敗」が run を止める | med | med | design §3.3 どおり mismatch は `success:false` 記録 + run 終了時に mismatch obs を抱える scenario を失敗報告(exit≠0)。schema test で pin |
| 7 | fixture 生成が canonical schema と drift(Deserialize-only 型に対し JS 平行実装と同じ罠) | med | high | 生成 JSON を必ず `RunFixture::parse` に通す fail-fast anchor(ADR 確定)。`{fixture:{…}}` passthrough + profile 生成の双方が同じ経路を通ることを tests/fixture.rs で検証 |
| 8 | workflow arm の fixture `policy`/`abort`/`expect` 節誤用 — `workflow run` には効かず hang session が default 300s timeout で wall-time 爆発 | med | med | design §2.1 明記どおり workflow 系の制御は scenario `policy` overlay(`execution.timeout`・`output.repairAttempts`)で供給。manifest validation で workflow arm を含む失敗 scenario の overlay 宣言を確認 |
| 9 | v1 result file の改変・混在 | low | high | v1 JSON は一切触れず(`create_new` 非上書き・v2 load が schemaVersion で reject)。T6/T7 の Done-When に v1 file 不変の git diff 確認を要求 |
| 10 | `cargo test` が spawn 測定を実行して check pipeline が壊れる | low | high | 測定実行は `cargo run -p cheaper-bench` のみ。`[[test]]` target・`--ignored` 測定は作らない(ADR 不採用形態)。unit test は純粋関数/fixture/manifest/schema に限定 |
| 11 | Windows/Unix parity で fixture child spawn が壊れる | low | med | `current_exe()` abs path 直 spawn・shell 非経由・`Command` API のみ(route_run.rs が両 platform で実証済みの同一手法) |
| 12 | `workflow-usage.md` に未計測の値・親 usage 由来の節約主張が混入 | low | med | T4 は placeholder のみ・T7 は実測 artifact 由来の値のみ・claim 規則(measured/estimated/unavailable + parentUsage unavailable 時の費用削減禁止)を test-spec で pin |
| 13 | usage.jsonl の route/usage-bearing error record を token 集計へ二重計上 | med | med | design §4.4 の分離規則(record 件数は別系統 field・envelope usage と再加算しない)を aggregate/record 層で実装・test で pin |
| 14 | 外部依存 availability | low | low | 新規 external dep なし(workspace 既存の serde/serde_json/tempfile/uuid + path dep のみ)。git(`rev-parse`)・rustc(`--version`)の ambient 呼出は best-effort(失敗 null) |

### Rollback

- **Trigger**: harness が deterministic fixture で pass しない・schema v2
  invariant が成立しない・`mise run check` が回復不能に fail する。
- **Procedure**: revert unit は本 change の commit 群 — T6 で削除した
  `.mjs` file は git 履歴から完全復元可能(削除自体が 1 commit)。
  `benchmarks/results/` の新 v2 artifact は append-only のため残して
  よい(v1 file は不変)。crate 追加・manifest v2・doc sync は段階
  commit で戻せる。
- **Verification**: revert 後に `mise run check` が pass すること
  (旧 pipeline — `node --test benchmarks/test/` 復帰を含む)。

## Task Breakdown

task doc へ分割した slice(7 task — T1-T5 直列、T6・T7 は T5 後に並列):

- T1: crate skeleton + fixture child + materialization —
  `docs/tasks/scaffold-cheaper-bench-crate-and-fixture-child.md`
- T2: manifest v2 + 12 scenario + spawn/observation/parsers —
  `docs/tasks/implement-bench-manifest-v2-scenarios-and-observation.md`
- T3: record/aggregate schema v2 + claim invariants + store —
  `docs/tasks/implement-bench-record-schema-v2-and-aggregation.md`
- T4: doc sync(pipeline 行削除は T2 で着地) —
  `docs/tasks/wire-bench-check-pipeline-and-sync-docs.md`
- T5: smoke run(実 binary・subset) —
  `docs/tasks/verify-bench-smoke-run-on-real-binary.md`
- T6: 旧 Node harness 削除 —
  `docs/tasks/remove-legacy-node-benchmark-harness.md`
- T7: 全 12 scenario 実測 + workflow-usage.md 実値 —
  `docs/tasks/run-full-benchmark-measurement-and-update-workflow-usage.md`

## Verification

| Spec AC / Requirement | Test / Check | Command |
| --- | --- | --- |
| manifest v2 validation が fail-fast(required coverage・arm/route/comparisons 規則) | `cargo test -p cheaper-bench --test manifest` | `mise exec -- cargo test -p cheaper-bench --test manifest` |
| fixture 生成が RunFixture::parse round-trip で canonical schema に anchor | `cargo test -p cheaper-bench --test fixture` | `mise exec -- cargo test -p cheaper-bench --test fixture` |
| schema v2 の 6 invariant(claim 境界・scripted marker・environment.node 拒否) | `cargo test -p cheaper-bench --test schema` | `mise exec -- cargo test -p cheaper-bench --test schema` |
| median/p95/stableValue/comparisons v1 parity + boundary case | `cargo test -p cheaper-bench --test aggregate` | `mise exec -- cargo test -p cheaper-bench --test aggregate` |
| `cargo test --workspace` が cheaper-bench unit test を含む | workspace test | `mise exec -- cargo test --workspace --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` |
| `mise run check` が `node --test benchmarks/test/` を除いた pipeline で全 pass | canonical verification contract | `mise run check` |
| `{node}`・`process.execPath`・`node:fs`・`environment.node` が新 harness/benchmarks に存在しない | boundary grep | `git grep -n '{node}\|process\.execPath\|node:fs\|environment\.node' -- benchmarks/ ':(exclude)benchmarks/results/**' crates/cheaper-bench/` が 0 件(`benchmarks/results/` の immutable v1 artifact は `node:fs` を含む旧計測記録のため除外) |
| smoke run が schema v2 artifact を非上書き保存・失敗系 arm が armExpect どおり観測 | T5 実測 | `mise exec -- cargo run --release -p cheaper-bench -- --scenario … --warmups 1 --repetitions 3` |
| preflight probe が dev-seam 無し binary を fail-fast で検出 | T5 手動確認 | dev-seam 無し build を `--binary` で指して probe 失敗 + 診断を確認 |
| 実行契約(exit code mirror・log 保持・再実行防止)の回帰 test | 既存 `route_run.rs` + cheaper-bench fixture unit test | `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration route_run` |
| v1 result file が不変 | immutability check | `git diff -- benchmarks/results/` で v1 JSON に差分無し |
| 全 12 scenario 実測 + workflow-usage.md 実値・TS 区別 | T7 実測 + doc 確認 | run artifact + `docs/guides/workflow-usage.md` の値が artifact 由来・legacy 表記あり |
| parentUsage unavailable 時に親 token/費用削減の表示が無い | schema invariant #6 + doc review | schema test + `workflow-usage.md`・README の表示規則確認 |
| doc-id・index・reciprocal 整合 | lint | `node scripts/check-doc-ids.mjs` exits 0・`audit_docs.js --type plan/test-spec/task` で本 chain findings 0・task graph に cycle/missing ref 無し |

## Open Questions / Gaps

- T5 smoke の rep 数(`--warmups 1 --repetitions 3`)は gate 用の最小値
  — 正式な統計値は T7 の `--warmups 2 --repetitions 15` run が出す。
- T7 の実測は長い wall-time を持つ可能性がある(hang/timeout fixture を
  含む worker-failure-modes は `execution.timeout` overlay で縮小済みの
  はず — manifest 値を T2 で確定)。実行環境の負荷で wall-time の絶対値は
  変わるが、token/byte 面は deterministic seeded fixture で不変のはず
  (回帰 anchor)。
- `workflow-usage.md` に掲載する値の粒度(median/p95・retainedBytes・
  route 削減率・condense overhead)は T7 実測時に artifact から選定 —
  design §6 は「実測 artifact 由来の値のみ採用」のみを拘束する。
- actual parent usage の取得経路は依然 `unavailable` — `observed` 配線は
  将来の contract 拡張(design Deferred Design Concerns)。

## Self-Review

- [x] Every spec requirement maps to a task.(要件全量 → T1-T7: crate/
      fixture/child=T1、manifest/scenario/observation+pipeline 行削除
      =T2、record/aggregate/claim=T3、doc sync=T4、smoke=T5、旧
      harness 削除=T6、実測+workflow-usage=T7)
- [x] No placeholders remain.(dep edge・12 scenario・6 invariant・
      verification command は design の確定値を転記。T7 掲載値の粒度
      のみ実測時確定の記録欄)
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution — T1-T3 は crate 内部の層依存により
  直列、T4-T7 は検証・doc・実測の段階。task 毎に green state を確認し
  ながら review checkpoint を置く。
- Independent work streams: T6・T7(T5 着地後に並列可)。
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 前提: upstream 4 doc(discovery resolved・spec approved・ADR
  accepted・design approved)が確定済み — design §1-§6 が field 級の
  authoritative HOW であり、本 plan は implementation order への転記。
  Design Inputs 節に記した 6 件の pin(manifest 型/validation の T1/T2
  分割・T5/T7 の smoke/実測分離・workflow-usage.md 2 段更新・pipeline
  行削除を T2 manifest flip と同一変更・T6 後置・T7 は本 PR 着地)
  以外の独自判断を足していない。
- 重点確認: (a) `cheaper-bench` の `cheaper-cli` dep edge が default
  feature のまま(dev-seam 宣言禁止 — feature unification で product
  bin に seam 混入)、(b) 全 byte 計測が
  `cheaper_core::json::stringify` 経由(v1 parity)、(c) armExpect
  mismatch が `success:false` 記録で即時 abort でないこと、(d)
  `node --test benchmarks/test/` 行削除が T2 の manifest flip と同一
  変更で着地し T6 の file 削除より先行すること、(e) preflight probe
  が bulk-reader output schema 適合 fixture
  を持つこと(空 stdin/非適合 finish では probe が意味を持たない)、
  (f) `environment.node`・`{node}`・`process.execPath`・`node:fs` が新
  harness に残っていないこと、(g) v1 result file が不変であること、
  (h) `cargo test` が spawn 測定を含まないこと。
- 既知の受容事項: `process-*` scenario の v1 wall-time は node 起動
  cost を含んでおり fixture child 化で絶対値がずれる — baseline 同等性
  の別記として `fixtureRevision:"seeded-v2"`・README migration note・
  結果 record で区別する(design §7)。`NODE_ENV=test` は seam
  activation var として残る vestige(rename は本決定の範囲外)。
