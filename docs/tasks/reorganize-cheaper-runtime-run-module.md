---
id: "TASK-034DzYxGyUC8loD0ruTCQN"
type: "task"
status: "done"
title: "T6: cheaper-runtime 内部再配置 (F6)"
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
    - "docs/tasks/remove-js-json-shim.md"
  blocks:
    - "docs/tasks/gate-dev-seams-behind-dev-seam-feature.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# T6: cheaper-runtime 内部再配置 (F6)

## Work

plan `docs/plans/execute-post-migration-structural-cleanup.md`
(PLAN-034Dz3LX6Jz18cbOR0KtFd)の T6 stream(F6 / R-B)。
`cheaper-runtime` の run 経路に属する flat file を `src/run/` module
階層へ再配置する。module alias re-export で現行の公開 path
(`cheaper_runtime::{meta,run_error,panic_boundary}::*`)を維持し、
consumer 書き換えを最小化する。behavior-preserving。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/execute-post-migration-structural-cleanup.md` — 本 task が
  implement する plan。File Map「T6: `cheaper-runtime` 内部再配置
  (F6 / R-B)」が作業の正
- `docs/specs/post-migration-structural-cleanup-scope.md` — F6 stream の
  acceptance criteria
- `docs/designs/post-migration-structural-cleanup-reorganization.md` —
  Components and Boundaries の cheaper-runtime target layout

## Dependencies

- `docs/tasks/remove-js-json-shim.md`(T1)— runtime `meta.rs`/
  `repair.rs` の `js_json` import 変更との競合回避。T1 完了後に着手する
  (plan の Dependencies table: T6 depends on T1)
- `docs/tasks/gate-dev-seams-behind-dev-seam-feature.md`(T8)を block
  する — dev/ 化は T6 の run module 確定後に実施する

## Affected Files

- `src/run.rs` → `src/run/mod.rs`
- `src/run_error.rs` → `src/run/error.rs`
- `src/abort.rs` → `src/run/abort.rs`
- `src/grace.rs` → `src/run/grace.rs`
- `src/repair.rs` → `src/run/repair.rs`
- `src/output.rs` → `src/run/output.rs`
- `src/meta.rs` → `src/run/meta.rs`
- `src/panic_boundary.rs` → `src/run/panic_boundary.rs`
- `src/lib.rs` — module 宣言追従 + 公開 path を `pub use` で維持
- module path 互換方針(確定): module alias re-export で維持し、
  consumer 書き換えは最小化する。`src/run/mod.rs` で
  `pub mod error; pub mod meta; pub mod panic_boundary;` 等を宣言し、
  `src/lib.rs` で `pub use run::error as run_error;`・`pub use run::meta;`・
  `pub use run::panic_boundary;` 等により
  `cheaper_runtime::{meta,run_error,panic_boundary}::*` の現行 path を
  維持する。known consumer: cli `tests/envelope.rs`:14,38、runtime
  `tests/run.rs`:38,560、`tests/run_fixture_test.rs`:33
- `auth/`・`catalog/`・`plugin.rs`・`runtime.rs`・`fake_provider.rs`・
  `fixture.rs` は T6 では現位置維持(dev/ 化は T8)

(全 path の prefix は `crates/cheaper-runtime/`)

## Done When

- [ ] run 経路 8 file が `src/run/` 配下へ再配置され、`lib.rs` の
      module 宣言が追従している
- [ ] `cheaper_runtime::{meta,run_error,panic_boundary}::*` の現行
      module path が `pub use` alias で維持され、known consumer が
      無変更でビルドされる
- [ ] `cargo build --workspace` と `cargo test -p cheaper-runtime`・
      `cargo test -p cheaper-cli` が green

## Out of Scope

- `fake_provider.rs`・`fixture.rs` の `dev/` 化と feature gate — T8 の
  範囲。`auth/`・`catalog/`・`plugin.rs`・`runtime.rs` は現位置維持

## Verification

- [ ] `mise exec -- cargo build --workspace` exits 0
- [ ] `mise exec -- cargo test -p cheaper-runtime` exits 0
- [ ] `mise exec -- cargo test -p cheaper-cli` exits 0(known consumer
      `tests/envelope.rs` を含む)
- [ ] `test -f crates/cheaper-runtime/src/run/mod.rs && test -f
      crates/cheaper-runtime/src/run/error.rs && test -f
      crates/cheaper-runtime/src/run/meta.rs` exits 0
      (target layout が存在)
- [ ] `test ! -f crates/cheaper-runtime/src/run_error.rs && test ! -f
      crates/cheaper-runtime/src/meta.rs && test ! -f
      crates/cheaper-runtime/src/panic_boundary.rs` exits 0
      (旧 flat file 不在)
- [ ] `git grep -n "pub use run::error as run_error\|pub use run::meta\|
      pub use run::panic_boundary" -- crates/cheaper-runtime/src/lib.rs`
      が alias re-export を返す(公開 path 互換維持)
