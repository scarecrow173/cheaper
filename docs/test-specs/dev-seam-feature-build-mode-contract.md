---
id: "TSPEC-034DzTH1ljq4pSfIHyGwVs"
type: "test-spec"
status: "draft"
title: "dev-seam feature の build-mode contract"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/execute-post-migration-structural-cleanup.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/post-migration-structural-cleanup-reorganization.md"
  derived-by: []
  verifies:
    - "docs/adr/gate-dev-test-seams-behind-a-dev-seam-cargo-feature.md"
    - "docs/specs/post-migration-structural-cleanup-scope.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# dev-seam feature の build-mode contract

## Purpose

`dev-seam` cargo feature 導入による build-mode 契約 — dev/test seam
(`cheaper-runtime` の `fake_provider`/`fixture`、`cheaper-cli` の
`fake_stream` と run 経路の fixture plumbing)が default build では
コンパイル対象外となり、feature 有効時のみ現行通りに動作すること — を
保証する意図を記録する。

consumer install(`cargo install --git --locked cheaper-cli`)が seam を
含まない binary を得ること(攻撃面・公開 API の純化)と、dev/test/
benchmark/e2e の全実行経路が feature 有効で維持されることの両立が本
契約の核心であり、「seam は開発用」という意図が feature 宣言として
コード構造から読める状態を守る。seam 自体が廃止されるか、
ADR-034DyGyITd0YaATg8fkld7 が棄却・supersede された場合にのみ退役する。

## Feature

`dev-seam` cargo feature による dev/test seam の compile-time gate。
default off とし、feature 有効時のみ seam がコンパイル・動作する。

## Rules

- Rule: feature 名は `dev-seam` に一元化する。runtime と cli で同名とし、
  cli 側は `dev-seam = ["cheaper-runtime/dev-seam"]` で forward する。
  `fake`/`fixture`/`test-utils` 等の複数 feature 化はせず、default
  feature への暗黙追加もしない(consumer install が seam を拾うと目的を
  破壊する)。
- Rule: feature OFF(default build)では seam コード自体がコンパイル
  対象外である。「seam を呼ばない」runtime fallback ではなく
  `#[cfg(feature = "dev-seam")]` で除外する。対象は
  `cheaper_runtime::dev::{fixture,fake_provider}`(`pub use dev::{
  fixture, fake_provider}` による `cheaper_runtime::{fixture,fake_provider}`
  互換 path 含む)、`cheaper-cli` の `fake_stream` module 全体、および
  `run.rs`/`workflow_run.rs` の fixture plumbing(`load_test_stream`
  呼出・env overlay・`cheaper-fake` plugin 登録・`ScriptedAuthInteraction`
  /`auth_logins`・fixture abort task・`abort_grace_ms` override・
  `explicit_model`/`policy` fallback)。
- Rule: feature OFF の binary では `CHEAPER_TEST_FAKE_STREAM=<fixture>`
  と `NODE_ENV=test` を設定しても fake 経路は起動せず、run は通常の
  production 経路に従う(fixture worker は named resolution で
  `WORKER_NOT_FOUND`)。precedent は cli `tests/run.rs` の
  `run_fake_stream_disabled_without_node_env_test` — 同じ不成立条件を
  compile-time gate へ引き上げたもの。
- Rule: feature ON(`--features cheaper-runtime/dev-seam,cheaper-cli/dev-seam`)
  では seam 依存の全挙動が現行と同一である。
  `CHEAPER_TEST_FAKE_STREAM` fixture loading、`NODE_ENV=test` activation
  規則、`cheaper-fake` provider plugin 登録、`auth_logins` scripted
  auth、env overlay、fixture abort task、`abort_grace_ms` override、
  `explicit_model`/`policy` fallback を含む。
- Rule: feature 有効化は workspace 経由で対象 crate を明示した
  `cargo test --workspace --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam`
  の形とし、この canonical 経路で全 seam 依存 test target が実行される。
- Rule: 丸ごと seam 依存の test target は `required-features` で gate する
  (runtime `tests/run_fixture_test.rs`、cli `tests/regression/` 一式)。
  seam/non-seam 混在 target は file-split で seam case を専用 target へ
  分離する(runtime `tests/run.rs`、cli `tests/run.rs`・
  `tests/workflow.rs`)ため、non-seam coverage は feature の有無で
  gate されない。in-src `#[cfg(test)]` で seam 型を使うものは
  `#[cfg(all(test, feature = "dev-seam"))]` とする。
- Rule: behavior-preserving — envelope `schemaVersion:1` parity、
  exit code、usage.jsonl、hook 挙動は feature ON/OFF いずれでも不変。

## Examples

- Example: `cargo build -p cheaper-cli`(default feature)で
  `fake_provider`・`fixture`・`fake_stream` がコンパイル対象に含まれず、
  `cargo check` でも `CHEAPER_TEST_FAKE_STREAM` 設定でも seam は
  発火しない。
- Example: default-feature binary に `CHEAPER_TEST_FAKE_STREAM=<fixture>`
  + `NODE_ENV=test` を設定して `run fixture-worker` を実行しても、
  fixture は読まれず production 経路の named resolution が
  `WORKER_NOT_FOUND` error envelope を返す。
- Example: `cargo test --workspace --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam`
  で seam 依存 target が全て実行され pass する(runtime `tests/run.rs`
  seam case + `tests/run_fixture_test.rs`、cli `tests/run.rs` seam
  case + `tests/workflow.rs` seam case + `tests/regression/` 55 case)。
  feature 無しの `cargo test --workspace` では required-features により
  seam target が skip され、non-seam test はそのまま pass する。
- Example: `cargo build --release -p cheaper-cli --features dev-seam`
  で build した binary は `benchmarks/run.mjs` が
  `CHEAPER_TEST_FAKE_STREAM` 経由の fixture で現行通り動作し、Ollama
  e2e(`CHEAPER_E2E_OLLAMA=1`)も feature 有効で動く。
- Example: `cargo install --git --locked cheaper-cli` は default
  feature のまま feature 指定を consumer に要求せず、install された
  binary に seam コードは同梱されない。

## Guarantees

- consumer install および default build artifact が dev/test seam コードを
  一切含まず、`cheaper_runtime` の公開 API に fixture/fake 型が露出しない。
- mise task の feature 配線(`tasks.check` の `cargo test`・`cargo clippy
  --all-targets -- -D warnings`、`tasks.setup` の `cargo build
  --workspace --all-targets`)により、gated code が lint・build・test の
  検証から抜け落ちない。
- non-seam test coverage(`tests/envelope.rs` 等、および混在 file から
  分離された non-seam case)は feature ON/OFF に関わらず常に実行される。
- feature ON では `cheaper_runtime::{fixture,fake_provider}` の現行
  module path が `pub use dev::{fixture, fake_provider}` で維持され、
  seam consumer(test・benchmark harness)の書き換えは最小化される。

## Non-goals

- seam 機能自体の正しさ(fixture DSL・scripted provider の挙動・
  activation 規則の個別 semantics)の検証 — 既存 test が担う領域であり、
  本 spec は build-mode の gate 契約のみを対象とする。
- gate 実装の個別詳細(seam-off 時の dead-code 解消・`session_factory`
  signature 調整等)と runnable な検証コマンド — plan T8 file map と
  Verification matrix、task-doc の `## Verification` の領域。
- `grep_eval`(`__grep-eval` dispatch)の gate 化 — production code で
  あり gate 対象外と確定済み。本 spec の seam 定義には含めない。
- 実行結果の記録 — pass/fail は impl-doc experiment log・CI evidence の
  領域。

## Risk

- この保証が落ちると、default build に seam が混入したままになり、
  consumer binary の攻撃面縮小と公開 API 純化という ADR の目的そのものが
  silent に破壊される。
- feature 配線の漏れがあると benchmark・regression・e2e が default
  binary で fail するか required-features で silent skip され、
  seam 前提の検証基盤が気づかないまま縮退する。
- activation 規則(`NODE_ENV=test` + `CHEAPER_TEST_FAKE_STREAM`)や
  fixture 契約が変わると、seam に依存する全 test・benchmark が崩れ、
  behavior-preserving の前提(envelope parity・exit code 不変)を
  検証する術自体が失われる。
