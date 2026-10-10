---
id: "TASK-034DzYI5SJnPTX6KZeAvYU"
type: "task"
status: "done"
title: "T8: dev-seam feature gate (F2)"
created: "2026-10-10"
updated: "2026-10-10"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/execute-post-migration-structural-cleanup.md"
  implemented-by: []
  depends-on:
    - "docs/plans/execute-post-migration-structural-cleanup.md"
    - "docs/tasks/reorganize-cheaper-runtime-run-module.md"
    - "docs/tasks/rename-provider-crate-and-modularize-devin.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/dev-seam-feature-build-mode-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# T8: dev-seam feature gate (F2)

## Work

plan `docs/plans/execute-post-migration-structural-cleanup.md`
(PLAN-034Dz3LX6Jz18cbOR0KtFd)の T8 stream(F2 / S-B)。dev/test seam
(`cheaper-runtime` の `fake_provider`/`fixture`、`cheaper-cli` の
`fake_stream` と run 経路の fixture plumbing)を `dev-seam` cargo
feature で compile-time gate する。default build では seam コードが
コンパイル対象外となり、feature 有効時のみ現行通りに動作する。
build-mode contract の正は
`docs/test-specs/dev-seam-feature-build-mode-contract.md`。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/execute-post-migration-structural-cleanup.md` — 本 task が
  implement する plan。File Map「T8: `dev-seam` feature gate
  (F2 / S-B)」が作業の正
- `docs/specs/post-migration-structural-cleanup-scope.md` — F2 stream の
  acceptance criteria
- `docs/adr/gate-dev-test-seams-behind-a-dev-seam-cargo-feature.md`
  (ADR-034DyGyITd0YaATg8fkld7)— feature 名一元化・default off・
  `required-features` test gating・mise task feature 配線の判断根拠
- `docs/test-specs/dev-seam-feature-build-mode-contract.md` — 本 task が
  満たすべき build-mode verification contract(`verified-by`)
- `docs/designs/post-migration-structural-cleanup-reorganization.md` —
  dev/ module target layout

## Dependencies

- `docs/tasks/reorganize-cheaper-runtime-run-module.md`(T6)— run
  module 構成確定後に cfg 対象を確定させる
- `docs/tasks/rename-provider-crate-and-modularize-devin.md`(T7)— cli
  `Cargo.toml`・import を共有するため rename 確定後に着手
- plan の Dependencies table: T8 depends on T6, T7(全ての reorg 後に
  実施して cfg 対象を確定させる)

## Affected Files

- `crates/cheaper-runtime/Cargo.toml` — `[features] dev-seam = []`
- `crates/cheaper-cli/Cargo.toml` —
  `[features] dev-seam = ["cheaper-runtime/dev-seam"]`
- `crates/cheaper-runtime/src/dev/{mod,fake_provider,fixture}.rs` —
  fake_provider.rs・fixture.rs を `dev/` 配下へ移動し
  `#[cfg(feature = "dev-seam")] pub mod dev;` で公開
- `crates/cheaper-runtime/src/lib.rs` — fake/fixture の mod 宣言を
  cfg gate
- module path 互換方針(確定): `#[cfg(feature = "dev-seam")]
  pub use dev::{fixture, fake_provider};` で
  `cheaper_runtime::{fixture,fake_provider}::*` の現行 module path を
  維持し、consumer 書き換えを最小化する。known consumer: runtime
  `tests/run.rs`:33,622,668,719・`tests/run_fixture_test.rs`:27-30、cli
  `tests/regression/harness.rs`:19・`src/fake_stream.rs`:19・
  `src/run.rs`:45。`fixture_default_policy`(lib.rs:69)は consumer ゼロ
  (repo-wide grep で検証済み)のため再 export せず drop する
- `crates/cheaper-cli/src/fake_stream.rs` — `#[cfg(feature = "dev-seam")]`
  で module 全体を gate(`src/lib.rs` の `pub mod fake_stream;` も)
- `crates/cheaper-cli/src/run.rs` — fixture plumbing を cfg gate(全
  site): `load_test_stream` 呼出(:359)・`resolve_run_worker` への
  fixture param/`resolve_named_worker` fallback(:361)・
  `f.policy.apply_to(&mut policy)`(:376-381)・`create_runtime` への
  fixture 引数(:406)・`session_factory` 呼出(:414)・`explicit_model`
  fallback(:419-422)・fixture abort task(:432-443)・
  `abort_grace_ms` override(:457)。`create_runtime`(:105-135)内の
  env overlay・`cheaper-fake` plugin push・`auth_logins` scripted
  login も同様
- `crates/cheaper-cli/src/workflow_run.rs` — 同上:
  `load_test_stream` 呼出(:166)・session_factory 分岐(:170-173)・
  `explicit_model`(:178)・`abort_grace_ms`(:179)
- seam-off 時の dead-code 対象(各々 `#[cfg]` gate または
  restructure): `SignalRelay::controller()`(run.rs:297、sole caller
  :433)、`Shared.env` field(workflow_run.rs:123、sole reader :166、
  構築 :735-738)、`session_factory` signature の `RunFixture` 参照
  (run.rs:184)、`create_runtime` の fixture param(run.rs:105-107)。
  clippy `-D warnings` での dead-code fallout 解消を T8 の想定工数に
  含める
- test gating strategy(確定):
  - 丸ごと seam 依存の target → `[[test]] required-features =
    ["dev-seam"]`: `crates/cheaper-cli/tests/regression/`(55 case)、
    `crates/cheaper-runtime/tests/run_fixture_test.rs`
  - seam/non-seam 混在 target → file-split: cli `tests/run.rs`
    (seam 依存 case + non-seam case 混在)・cli `tests/workflow.rs`・
    runtime `tests/run.rs` は seam 依存 test を専用 target file
    (`tests/fake_stream.rs` 等)へ分離して required-features、
    non-seam test は現 file に残す
  - in-src `#[cfg(test)]` module で seam 型を使うもの →
    `#[cfg(all(test, feature = "dev-seam"))]`
  - non-seam test(`tests/envelope.rs` 等)は ungated のまま
- `grep_eval` の seam 分類(確定): gate 対象外。`__grep-eval`
  dispatch は production code(`crates/cheaper-cli/src/main.rs`:73-77。
  `EvalSpawn::current_exe` が production default — grep_eval.rs:106)。
  `test_support`(grep_eval.rs:599-)のみ既に `#[cfg(test)]` 済み
- `mise.toml` — benchmark/regression/e2e 関連 task は存在せず、実
  touch point は `[tasks.setup]`・`[tasks.check]` の cargo コマンド:
  - `tasks.check` の `cargo test --workspace`(line ~46)→
    `cargo test --workspace --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam`
    へ更新(ADR-034DyGyITd0YaATg8fkld7 の prescribed form)
  - `tasks.check` の `cargo clippy --workspace --all-targets -- -D
    warnings`(line ~43)にも同 feature を付与 — 付けなければ gated
    code が `-D warnings` を逃れる
  - `tasks.setup` の `cargo build --workspace --all-targets`(line ~31)
    も同 feature で build する(推奨: 付与 — fresh checkout で seam
    test target も build 検証される)
  - 注記: regression(55 case)・Ollama e2e は `cargo test --workspace`
    内で動くため、上記 feature 付与で網羅される
- `benchmarks/lib/cli.mjs` — binary 解決 comment(:14-18)の build
  手順を `cargo build --release -p cheaper-cli --features dev-seam`
  前提に更新。`benchmarks/README.md` の前提 section(:9-15)の build
  手順も同様に更新(`benchmarks/run.mjs` 自体に build step は存在
  しない)
- `CONTRIBUTING.md` — `mise run check`・`cargo test` 手順(:21-44)を
  feature 有効 form へ追従

## Done When

- [ ] `dev-seam` feature が runtime/cli 両 `Cargo.toml` に宣言され、cli
      側が `cheaper-runtime/dev-seam` を forward している
- [ ] default build で seam コード(`dev::{fixture,fake_provider}`・
      `fake_stream`・run 経路 fixture plumbing)がコンパイル対象外で、
      `pub use dev::{fixture, fake_provider}` による現行 module path
      互換が feature ON 時に維持される
- [ ] test gating(required-features・file-split・
      `#[cfg(all(test, feature = "dev-seam"))]`)が確定方針通り適用
      され、non-seam test は ungated のまま実行される
- [ ] `mise.toml`・`benchmarks/lib/cli.mjs`・`benchmarks/README.md`・
      `CONTRIBUTING.md` の feature 配線・build 手順が更新されている
- [ ] Verification の default/feature-on 両 matrix が green

## Out of Scope

- seam 機能自体の正しさ(fixture DSL・scripted provider 挙動)の検証
  — 既存 test の領域(test-spec の Non-goals より)
- `grep_eval`(`__grep-eval` dispatch)の gate 化 — production code で
  あり gate 対象外と確定済み

## Verification

- [ ] `mise exec -- cargo build --workspace` exits 0(default feature
      で seam-off ビルドが通る)
- [ ] `mise exec -- cargo clippy --workspace --all-targets --features
      cheaper-runtime/dev-seam,cheaper-cli/dev-seam -- -D warnings`
      exits 0(gated code を含めて lint clean)
- [ ] `mise exec -- cargo test --workspace --features
      cheaper-runtime/dev-seam,cheaper-cli/dev-seam` exits 0
      (seam 依存 target 全件 + non-seam が pass)
- [ ] `mise exec -- cargo test --workspace` exits 0(feature 無しで
      seam target が required-features skip され、non-seam test が
      pass)
- [ ] `mise exec -- cargo build -p cheaper-cli` 後、default-feature
      binary で seam が起動しないこと: `echo '{}' |
      CHEAPER_TEST_FAKE_STREAM=/tmp/fixture.json NODE_ENV=test
      ./target/debug/cheaper-cli run fixture-worker | grep -q
      "WORKER_NOT_FOUND"` exits 0(fixture は読まれず named
      resolution が WORKER_NOT_FOUND を返す。precedent: cli
      `tests/run.rs`:323 `run_fake_stream_disabled_without_node_env_test`
      を default-feature binary 対象として実施)
- [ ] `git grep -n 'cfg(feature = "dev-seam")' --
      crates/cheaper-runtime/src/ crates/cheaper-cli/src/` が gate
      箇所を返す(seam module・fixture plumbing が cfg 済み)
- [ ] `mise run check` exits 0(feature 付与後の canonical pipeline が
      green)
- [ ] `cargo build --release -p cheaper-cli --features dev-seam` の
      binary で `mise exec -- node benchmarks/run.mjs` が現行通り
      動作する
- [ ] test-spec
      `docs/test-specs/dev-seam-feature-build-mode-contract.md` の
      Rules/Guarantees(default off・feature ON parity・
      required-features gating・behavior-preserving)を全て満たすことを
      上記 matrix で確認する
