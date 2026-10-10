---
id: "TASK-034D9bpfd4n0vQJjgXtBzj"
type: "task"
status: "done"
title: "Fix volume-root workspace boundary predicate"
created: "2026-10-09"
updated: "2026-10-09"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - { type: file, path: crates/cheaper-core/src/workspace/paths.rs }
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/volume-root-workspace-boundary-predicate-fix.md"
  implemented-by:
    - "docs/impl/ir/volume-root-workspace-boundary-predicate-trailing-separator-guard.md"
  depends-on:
    - "docs/plans/volume-root-workspace-boundary-predicate-fix.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/volume-root-workspace-boundary-predicate-fix.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/volume-root-workspace-boundary-predicate-behavior.md"
  references: []
  defers: []
  deferred-by: []
---

# Fix volume-root workspace boundary predicate

## Work

`crates/cheaper-core/src/workspace/paths.rs` の
`is_inside_workspace_root`（:103-110）に trailing-separator guard を
追加し、volume-root workspace の regression test を同 file 内
test mod に追加する（issue #77）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/volume-root-workspace-boundary-predicate-fix.md`
- `docs/designs/trailing-separator-guard-for-workspace-boundary-predicate.md`
- `docs/specs/volume-root-workspace-boundary-acceptance.md`
- `docs/test-specs/volume-root-workspace-boundary-predicate-behavior.md`

## Dependencies

- なし（単一 task、他 task への依存なし）

## Affected Files

- `crates/cheaper-core/src/workspace/paths.rs`

## Done When

- [x] `is_inside_workspace_root` の prefix が normalized `r` から
  `ends_with(nodepath::SEP)` guard 付きで構築される
  （`resource_contains` と同一 pattern）
- [x] win32/posix の volume-root unit test が `cfg!` gate で追加
  され、修正前に fail・修正後に pass する
- [x] `resolve_workspace_path` の volume-root test（nonexistent
  一意名 tail、期待 `real`/`display` 付き）が追加される
- [x] `mise exec -- cargo test -p cheaper-core` と
  `mise run check` が pass する

## Verification

- [x] `is_inside_workspace_root("C:\\", "C:\\resource-probe", true)`
  が win32 build で `true`（修正前 `false`）
- [x] `is_inside_workspace_root("/", "/a", false)` が posix build で
  `true`（posix assertion は CI 側で検証。本 host は win32）
- [x] `is_inside_workspace_root("C:\\", "D:\\x", true)` が `false`
- [x] guard の形が `predicates.rs:36-40` `resource_contains` と同一
  であることを code review で確認
