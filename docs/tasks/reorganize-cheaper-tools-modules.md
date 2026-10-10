---
id: "TASK-034DzYs0gJRVZ5JJfKAIAn"
type: "task"
status: "done"
title: "T5: cheaper-tools 内部再配置 (F1)"
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
    - "docs/tasks/consolidate-js16-into-cheaper-core.md"
  blocks: []
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

# T5: cheaper-tools 内部再配置 (F1)

## Work

plan `docs/plans/execute-post-migration-structural-cleanup.md`
(PLAN-034Dz3LX6Jz18cbOR0KtFd)の T5 stream(F1 / T-B)。
`cheaper-tools` 内部の flat な file 配置を design の target layout に
従って module 階層へ再配置する(glob 系・process 系)。behavior-
preserving であり、公開 API・tool 挙動は不変。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/execute-post-migration-structural-cleanup.md` — 本 task が
  implement する plan。File Map「T5: `cheaper-tools` 内部再配置
  (F1 / T-B)」が作業の正
- `docs/specs/post-migration-structural-cleanup-scope.md` — F1 stream の
  acceptance criteria
- `docs/designs/post-migration-structural-cleanup-reorganization.md` —
  Components and Boundaries の cheaper-tools target layout

## Dependencies

- `docs/tasks/consolidate-js16-into-cheaper-core.md`(T2)— 同一 crate
  を触る順序付け。T2 の `js16` 集約完了後に着手する(plan の
  Dependencies table: T5 depends on T2)

## Affected Files

- `crates/cheaper-tools/src/glob.rs` → `src/glob/mod.rs`
- `crates/cheaper-tools/src/glob_tables.rs` → `src/glob/tables.rs`
- `crates/cheaper-tools/src/tools/process.rs` → `src/tools/process/mod.rs`
- `crates/cheaper-tools/src/tools/process_executable.rs` →
  `src/tools/process/executable.rs`
- `crates/cheaper-tools/src/tools/process_tree.rs` →
  `src/tools/process/tree.rs`
- `crates/cheaper-tools/src/lib.rs`・`src/tools/mod.rs` — module 宣言追従
- glob/process 内部の相対参照(`crate::glob_tables` 等)を module path
  に追従

## Done When

- [ ] glob 系が `src/glob/{mod,tables}.rs`、process 系が
      `src/tools/process/{mod,executable,tree}.rs` の target layout に
      再配置されている
- [ ] `lib.rs`・`src/tools/mod.rs` の module 宣言と内部相対参照が
      新 module path に追従している
- [ ] `cargo build --workspace` と `cargo test -p cheaper-tools` が
      green

## Out of Scope

- `js16` の core 集約自体 — T2 で実施済み前提。本 task は file 配置の
  module 階層化のみ
- `cheaper-runtime` の再配置 — T6 の範囲

## Verification

- [ ] `mise exec -- cargo build --workspace` exits 0
- [ ] `mise exec -- cargo test -p cheaper-tools` exits 0
- [ ] `test -f crates/cheaper-tools/src/glob/mod.rs && test -f
      crates/cheaper-tools/src/glob/tables.rs && test -f
      crates/cheaper-tools/src/tools/process/mod.rs && test -f
      crates/cheaper-tools/src/tools/process/executable.rs && test -f
      crates/cheaper-tools/src/tools/process/tree.rs` exits 0
      (target layout が存在)
- [ ] `test ! -f crates/cheaper-tools/src/glob_tables.rs && test ! -f
      crates/cheaper-tools/src/tools/process_executable.rs && test ! -f
      crates/cheaper-tools/src/tools/process_tree.rs` exits 0
      (旧 flat file 不在)
- [ ] `git grep -n "glob_tables\|process_executable\|process_tree" --
      crates/` が 0 件を返す(exit 1。旧 module 名参照の残存なし)
