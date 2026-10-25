---
id: "TASK-7pwH1mAayBeUw0g24iMXvR"
type: "task"
status: "done"
title: "Scaffold cheaper-bench crate — lib+bin, fixture-child dispatch, RunFixture round-trip, workspace/policy materialization"
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
        path: "crates/cheaper-bench/tests/fixture.rs"
    modified:
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "crates/cheaper-cli/src/workflow_run.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-rust-native-benchmark-harness-issue-183.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-rust-native-benchmark-harness-issue-183.md"
  blocks:
    - "docs/tasks/implement-bench-manifest-v2-scenarios-and-observation.md"
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
    - "docs/adr/rust-native-benchmark-harness.md"
  defers: []
  deferred-by: []
---

# Scaffolding cheaper-bench — crate skeleton, fixture child, fixture generation, workspace/policy materialization

## Work

`crates/cheaper-bench` を design §1 の確定構成で新設し、harness の基盤層
(manifest 型・workspace/env/spawn・fixture 生成・fixture child)を実装
する。後続 task(T2/T3)がこの上に arm 実装・計測・record/aggregate を
載せるため、本 task は「crate が compile し unit test が green」という
self-contained な中間状態を作る。

1. `crates/cheaper-bench/Cargo.toml` — `publish = false`・`[lib]` +
   `[[bin]] name = "cheaper-bench"`。dep edge は design §1.2 どおり:

   ```toml
   [dependencies]
   cheaper-core    = { path = "../cheaper-core" }
   cheaper-cli     = { path = "../cheaper-cli" }
   cheaper-runtime = { path = "../cheaper-runtime", features = ["dev-seam"] }
   serde.workspace = true
   serde_json.workspace = true
   tempfile.workspace = true
   uuid.workspace = true
   ```

   - `cheaper-runtime` edge の `features = ["dev-seam"]` は
     `dev::fixture::RunFixture` 型が `#[cfg(feature = "dev-seam")]`
     配下のため必須 — `cargo run`/`cargo test -p cheaper-bench` が
     `--features` 無しで動く。
   - **`cheaper-cli` edge に `dev-seam` を宣言してはならない** —
     feature unification で同一 invocation に含まれる `cheaper` bin
     自体が seam 有効になり、計測対象 product binary への seam 付与が
     暗黙化する(ADR 禁止事項)。Cargo.toml にこの禁止を明記する
     comment を書く。計測用 binary の dev-seam 付与は必ず明示 build
     (`cargo build -p cheaper-cli --features dev-seam`)で行う。
   - 併せて comment に、`cheaper-runtime` edge の `dev-seam` 宣言
     により **flag 無しの `cargo test --workspace` でも
     cheaper-runtime の dev-seam 依存 target が新たに compile
     される**点(feature unification — `cheaper-cli` 側は default
     edge のため flag 必須のままで、compile 範囲は意図的に非対称)
     を記す — 発見されて驚かれないよう宣言時に文書化する。
   - crate 独自 feature は定義しない。async runtime(tokio)は導入
     しない — `std::process::Command` の blocking spawn で足りる。
   - workspace membership は root `Cargo.toml` `members = ["crates/*"]`
     glob で自動取得 — root manifest 変更は不要。
   - `Cargo.lock` が cheaper-bench package entry で更新される —
     `cargo fetch`/`cargo build` 後に commit する
     (`node scripts/check-lockfile.mjs` が drift を検査)。

2. `src/lib.rs` — module root。harness 全責務を lib 側に持つ
   (tests/ が lib API を直接検証する構成のため)。

3. `src/main.rs` — 薄い entry。`argv[1] == "--fixture-child"` を他の何
   よりも先に判定して `child::dispatch` へ委譲し、child 処理後
   `process::exit(code)` で抜ける — harness 本体・arg parse には進ま
   ない。通常経路は CLI parse → run。

4. `src/manifest.rs` — manifest v2 の **serde 型定義 + load** を実装する
   (fail-fast validation 本体は T2)。型は design §2.1 の manifest 形式
   を全て表現できること: `schemaVersion`・`fixtureRevision`・
   `scenarios[]`{id・title・files・workflowMaxJobs・policy overlay・
   workflow・savedWorkflowFile・processPolicy・route・fakeStream・arms・
   armBase・armEnv・armExpect・armFakeStream・manual・comparisons}。
   未知 key 方針・`schemaVersion === 2` の構造 check 程度は入れてよいが、
   REQUIRED_SCENARIO_IDS exact coverage・arm/route/comparisons の
   domain 規則は T2 の validation 層に置く。

5. `src/workspace.rs` — temp workspace 生成(design §2.4・
   Non-Functional Constraints):
   - scenario `files` の materialize・`.cheaper/models.yaml`
     (`cheaper-fake/fake-model` override)・`<cfg>/policy.yaml` の
     **section merge**(`workflowMaxJobs` → `workflow.maxJobs`・
     scenario `policy` overlay の section 部分 object・route
     `command.grants` — 後勝ち上書きではなく section merge)。
   - `processPolicy` は project 層 `<ws>/.cheaper/policy.yaml` の
     `process` 節へ(v1 parity — route grant と layer/scope が異なる
     ため同 file に混ぜない)。command の `{fixtureChild}` placeholder
     は `current_exe()` の abs path へ展開する。
   - `route` block の materialize: `<cfg>/routes.yaml`(version:1・
     route 1 件・target routed-exec.workflow.json・grant・
     output.tokenThreshold・passthroughEnv)・`<cfg>/policy.yaml` の
     `command.grants.<grant>`(`executable` = bench bin abs path・
     `argv` = literal `["--fixture-child", childMode]`・`env` =
     CHEAPER_BENCH_CHILD_BYTES 等・`maxOutputBytes?`・`timeout?`)・
     `<cfg>/routed-exec.workflow.json` を
     `env!("CARGO_MANIFEST_DIR")/../cheaper-cli/resources/workflows/
     routed-exec.workflow.json` から copy。
   - ws・cfgDir は observation(計測 spawn)毎に新規生成する粒度 —
     scenario/arm/rep 間で使い回さない(route-logs arm の setup→計測
     2 段のみ同一 cfgDir を共有するのは T2 の measure 側責務)。
   - section merge は spawn を伴わない純粋関数 — `#[cfg(test)]` 内
     unit test で、`workflowMaxJobs`→`workflow.maxJobs`・scenario
     `policy` overlay の section・route `command.grants` が同一
     `<cfg>/policy.yaml` 上で共存 merge されること(後勝ち上書きで
     一方が消えない — design §2.4)を pin する。計測実行ではない
     ため `cargo test` 混入禁止に触れない。

6. `src/env.rs` — spawn env 組立(v1 parity): PATH passthrough・
   HOME/USERPROFILE=workspace・`NODE_ENV=test`・`CHEAPER_CONFIG_DIR`・
   `CHEAPER_TEST_FAKE_STREAM`・SystemRoot/TEMP/TMP・`CHEAPER_LOG*`
   ambient passthrough allowlist・per-arm `armEnv` override。
   `NODE_ENV=test` は seam activation var として残す(rename 範囲外
   の vestige — comment で明示)。

7. `src/spawn.rs` — product binary / fixture child の blocking spawn:
   `std::process::Command` で stdout/stderr を常に capture・wall-time
   (`Instant::now` 前後差)・exit code を保持。**exit≠0 で throw
   しない**(design §7 nonzero-exit capture path — 失敗系計測の
   前提)。stdout の envelope parse は best-effort とし、解釈不能は
   `envelopeOk` 判定材料として扱い harness error にしない(per-mode
   parser 列は T2)。fixture child は `current_exe()` abs path 直
   spawn・shell 非経由・`Command` API のみ(Windows/Unix parity)。

8. `src/fixture.rs` — RunFixture JSON 生成(ADR 確定方式):
   `serde_json::json!`/template 組立て → **書込み前に必ず
   `RunFixture::parse` で round-trip 検証** → file write →
   `CHEAPER_TEST_FAKE_STREAM` path 設定用の path を返す。
   - `fakeStream` 指定: `"auto"`(既定)/`{profiles:[…], delayMs,
     toolCallPath}` — v1 generator 相当の profile 駆動 session 生成
     (usage_delta scripted 固定値・bulk-reader output schema 適合
     text を Rust 側へ移植)。
   - `{fixture:{…}}` — RunFixture DSL の部分 object を passthrough
     merge して生成(`sessions[].steps` の error/hang/panic/abort 等
     全節を manifest が直接書ける)。`armFakeStream` は arm 単位の
     差し替え。
   - 生成 JSON は必ず `RunFixture::parse` へ通す — 無効 step・型違反
     は benchmark 実行前に fail-fast。
   - `emit` child 出力用に fixed-seed PRNG 由来の deterministic 高
     エントロピ印字可能列生成器を実装する — `'L'.repeat(n)` 型の低
     エントロピ反復は禁止(condense `outputTokens > tokenThreshold`
     評価の決定性を担保できない — design §1.4)。

9. `src/child.rs` — fixture child mode 実装(design §1.4):
   - dispatch: `argv[1] == "--fixture-child"`・`argv[2]` が mode。
   - `emit` — `CHEAPER_BENCH_CHILD_BYTES`(既定 4096)byte を stdout
     へ書き exit 0(上記 seeded generator を使う)。
   - `fail` — stdout/stderr marker を書き
     `CHEAPER_BENCH_CHILD_EXIT`(既定 7)で exit。
   - `sleep`・`envdump` — **先行宣言のみ**(12 required scenario は
     消費しない。予約語彙として mode enum に宣言し、未実装 stub で
     よい — 到達時は未対応を示す exit)。
   - route 経路では typed command の argv(`argv[1..]` = exec_args)が
     そのまま `argv[1]`/`argv[2]` になる — grant の `argv` は
     ArgPattern の match pattern であって注入器ではない(design §1.4
     参照)。

10. `src/cli.rs` — arg parse skeleton(`--scenario` 複数・`--warmups`・
    `--repetitions`・`--output`・`--binary`・`--pricing`)・binary
    解決順(`--binary` → `target/release/cheaper` → `target/debug`
    fallback・repo root は `env!("CARGO_MANIFEST_DIR")/../..`)・
    scenario×arm×(warmups+reps) の run ループ骨格。**arm dispatch・
    preflight probe・record/aggregate は T2/T3** — 本 task では構造
    のみ(未接続 arm は「未実装」の明示 error で fail-fast)。

11. `tests/fixture.rs` — 生成 fixture JSON の `RunFixture::parse`
    round-trip(profile 生成・`{fixture:{…}}` passthrough・error/
    hang/panic 等の失敗系 step 語彙・無効 step/型違反の fail-fast)・
    emit generator の determinism/byte 長 pin・session 生成の規則
    (worker job N 件 → session N 件)を検証。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-rust-native-benchmark-harness-issue-183.md (T1 slice)
- docs/designs/benchmark-harness-redesign-issue-183.md (§1 crate layout・§1.2 dep edge・§1.4 fixture child・§2.4 materialize・§2.5 fixture 生成・Non-Functional Constraints)
- docs/adr/rust-native-benchmark-harness.md (dep edge 制約・fixture 供給方式の確定)

## Dependencies

- docs/plans/implement-rust-native-benchmark-harness-issue-183.md

## Affected Files

- `crates/cheaper-bench/Cargo.toml` + `src/{lib,main,cli,manifest,workspace,env,spawn,fixture,child}.rs` + `tests/fixture.rs`(全て新設)
- `Cargo.lock`(新規 workspace member の package entry — commit 必須)

対象外: product crate の実装(`cheaper-cli`/`cheaper-runtime` 等の
`src/`)、`benchmarks/` 配下の既存 file、`mise.toml`。

## Done When

- [x] `crates/cheaper-bench` が lib + bin + `publish = false` で新設
      され、dep edge が design §1.2 どおり(`cheaper-runtime` +
      `dev-seam`・`cheaper-cli` default feature + 禁止 comment・
      `cheaper-core`・serde/serde_json/tempfile/uuid、async runtime・
      独自 feature なし)
- [x] `main.rs` が `argv[1] == "--fixture-child"` を最初に判定し
      `child::dispatch` → `process::exit(code)` で抜ける
- [x] child mode `emit`(seeded 高エントロピ bytes・
      `CHEAPER_BENCH_CHILD_BYTES`)/`fail`(marker +
      `CHEAPER_BENCH_CHILD_EXIT`)が動作し、`sleep`/`envdump` が
      予約語彙として宣言されている
- [x] fixture 生成が profile 駆動・`{fixture:{…}}` passthrough 両経路で
      `RunFixture::parse` round-trip を必ず通り、無効入力が生成時
      fail-fast する
- [x] workspace.rs が files・models.yaml・`<cfg>/policy.yaml` section
      merge・`{fixtureChild}` placeholder 展開・route block の
      routes.yaml/grant/routed-exec copy を生成できる(observation 毎
      fresh 生成の粒度)
- [x] policy.yaml section merge の spawn-free unit test があり、
      `workflowMaxJobs`・`policy` overlay section・`command.grants`
      が共存 merge されることを pin している
- [x] spawn.rs が stdout/stderr capture・wall-time・exit code を返し、
      exit≠0 で throw しない
- [x] `cargo test -p cheaper-bench` が pass(tests/fixture.rs を含む)し
      `cargo build --workspace` が compile する
- [x] `Cargo.lock` が cheaper-bench entry を含み commit 済み

## Verification

- [x] `mise exec -- cargo test -p cheaper-bench` exits 0
- [x] `mise exec -- cargo build -p cheaper-bench` exits 0(fixture child
      として `cheaper-bench --fixture-child emit` が byte 出力を返す
      ことを手動で 1 回確認)
- [x] `mise exec -- cargo fmt --all -- --check` exits 0
- [x] `mise exec -- cargo clippy -p cheaper-bench --all-targets -- -D warnings` exits 0(未接続 arm/skeleton の dead code 残存なし)
- [x] `node scripts/check-lockfile.mjs` exits 0(Cargo.lock が commit 済みで drift なし)
- [x] Manual: `grep -n "dev-seam" crates/cheaper-bench/Cargo.toml` が
      `cheaper-runtime` edge のみを返す(`cheaper-cli` edge に
      dev-seam が無いこと — supplement)
