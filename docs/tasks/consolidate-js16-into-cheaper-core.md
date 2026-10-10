---
id: "TASK-034DzYnno038qspspOYHv2"
type: "task"
status: "done"
title: "T2: js16 core 集約 (F4)"
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
  blocks:
    - "docs/tasks/reorganize-cheaper-tools-modules.md"
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

# T2: js16 core 集約 (F4)

## Work

plan `docs/plans/execute-post-migration-structural-cleanup.md`
(PLAN-034Dz3LX6Jz18cbOR0KtFd)の T2 stream(F4)。`cheaper-tools` に
残置されている `js16` 実装を `cheaper-core` へ集約し、
`cheaper_tools::js16` を thin re-export 化して crate 境界を整理する。
caller の `crate::js16` 参照は互換維持のため無変更。behavior-preserving。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/execute-post-migration-structural-cleanup.md` — 本 task が
  implement する plan。File Map「T2: `js16` の core 集約(F4)」が作業の正
- `docs/specs/post-migration-structural-cleanup-scope.md` — F4 stream の
  acceptance criteria
- `docs/designs/post-migration-structural-cleanup-reorganization.md` —
  target layout

## Dependencies

- task 依存なし(plan の Dependencies table で T2 は独立)
- `docs/tasks/reorganize-cheaper-tools-modules.md`(T5)を block する —
  同一 crate を触る順序付けとして T2 完了後に T5 へ進む

## Affected Files

- `crates/cheaper-core/src/js16.rs` — `cheaper-tools/src/js16.rs` の
  実装を移設
- `crates/cheaper-core/src/lib.rs` — `pub mod js16;` 追加
- `crates/cheaper-tools/src/js16.rs` — `pub use cheaper_core::js16::*;`
  の thin re-export に置換(`cheaper_tools::js16` path 互換のため)
- caller(`crates/cheaper-tools/src/tools/{edit,find,grep}.rs`)は
  `crate::js16` 参照のまま変更不要の想定
- caller 調査済み: `crate::js16` 参照は tools/{edit,find,grep}.rs の
  3 file(edit.rs:365・find.rs:17・grep.rs:35)のみで外部参照ゼロ
  (repo-wide grep で検証済み)。spec の caller 調査 precondition を
  満たす

## Done When

- [ ] `js16` の実装が `crates/cheaper-core/src/js16.rs` へ移設され、
      `cheaper-core` が `pub mod js16;` で公開している
- [ ] `crates/cheaper-tools/src/js16.rs` が
      `pub use cheaper_core::js16::*;` の thin re-export になっている
- [ ] `crate::js16` caller(tools/{edit,find,grep}.rs)が無変更で
      ビルド・test が green

## Out of Scope

- `cheaper-tools` 内部の module 再配置(glob/process の階層化) —
  T5 の範囲

## Verification

- [ ] `mise exec -- cargo build --workspace` exits 0
- [ ] `mise exec -- cargo test -p cheaper-core` exits 0
- [ ] `mise exec -- cargo test -p cheaper-tools` exits 0
- [ ] `git grep -n "pub mod js16" -- crates/` が
      `crates/cheaper-core/src/lib.rs` のみを返す
- [ ] `git grep -n "crate::js16" -- crates/cheaper-tools/src/` の hit が
      tools/{edit,find,grep}.rs の既知 3 file(edit.rs:365・find.rs:17・
      grep.rs:35)のみであることを確認する
