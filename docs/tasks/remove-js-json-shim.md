---
id: "TASK-034DzYbfe9r0G8Nd58FQjy"
type: "task"
status: "done"
title: "T1: js_json shim 削除 (F3)"
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
    - "docs/tasks/reorganize-cheaper-runtime-run-module.md"
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

# T1: js_json shim 削除 (F3)

## Work

plan `docs/plans/execute-post-migration-structural-cleanup.md`
(PLAN-034Dz3LX6Jz18cbOR0KtFd)の T1 stream(F3)。`cheaper-rig-adapter`
の `js_json` re-export shim を削除し、呼出側を
`cheaper_core::json::stringify` へ直接付け替える。behavior-preserving
な残滓除去であり、envelope・exit code・公開 API の挙動は不変。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/execute-post-migration-structural-cleanup.md` — 本 task が
  implement する plan。File Map「T1: `js_json` shim 削除(F3)」が作業の正
- `docs/specs/post-migration-structural-cleanup-scope.md` — F3 stream の
  acceptance criteria
- `docs/designs/post-migration-structural-cleanup-reorganization.md` —
  target layout

## Dependencies

- task 依存なし(plan の Dependencies table で T1 は他 stream と独立・
  並列化可能)
- `docs/tasks/reorganize-cheaper-runtime-run-module.md`(T6)を block
  する — T6 着手前に runtime `meta.rs`/`repair.rs` の import 変更を
  確定させる必要がある

## Affected Files

- `crates/cheaper-rig-adapter/src/js_json.rs` — 削除(re-export shim のみ)
- `crates/cheaper-rig-adapter/src/lib.rs` — `pub mod js_json;` 除去
- `crates/cheaper-rig-adapter/src/agent.rs` — `crate::js_json::stringify*` →
  `cheaper_core::json::stringify::*` へ import 変更(2 箇所)
- `crates/cheaper-rig-adapter/tests/adapter.rs` — 参照更新(関数名含め)
- `crates/cheaper-runtime/src/meta.rs` — `cheaper_rig_adapter::js_json::stringify`
  → `cheaper_core::json::stringify` へ変更
- `crates/cheaper-runtime/src/repair.rs` — 同上(stringify_pretty)

## Done When

- [ ] `crates/cheaper-rig-adapter/src/js_json.rs` が削除され、
      `lib.rs` の `pub mod js_json;` 宣言が除去されている
- [ ] 全呼出側(`agent.rs`・`tests/adapter.rs`・runtime の
      `meta.rs`/`repair.rs`)が `cheaper_core::json::stringify` を直接
      参照している
- [ ] `cargo build --workspace` と対象 crate の test が green

## Out of Scope

- `js16` の core 集約(T2)、runtime 内部再配置(T6)など他 stream の
  変更 — 本 task は `js_json` shim の除去のみ

## Verification

- [ ] `mise exec -- cargo build --workspace` exits 0
- [ ] `mise exec -- cargo test -p cheaper-rig-adapter` exits 0
- [ ] `mise exec -- cargo test -p cheaper-runtime` exits 0
- [ ] `git grep -n "js_json" -- crates/` が 0 件を返す(exit 1)
