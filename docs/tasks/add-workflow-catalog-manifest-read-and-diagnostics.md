---
id: "TASK-034ZiFaW8ZtvJ5XKVksixu"
type: "task"
status: "done"
title: "Add workflow catalog manifest read and diagnostics"
created: "2026-11-17"
updated: "2026-11-17"
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
    - "docs/plans/implement-workflow-catalog-and-list.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-workflow-catalog-and-list.md"
  blocks:
    - "docs/tasks/add-workflow-list-cli-command.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/workflow-catalog-manifest-and-list-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# Add workflow catalog manifest read and diagnostics

## Work

`cheaper-workflow` crate に `catalog.rs` を追加し、
`<workspace>/.cheaper/workflows/catalog.json` の schemaVersion 1
manifest を bounded に read/validate/diagnose する。design data model
(64 KiB・256 entries・id<=64・path<=512・summary<=500・tags<=16×64)
どおりの bound、manifest-level fail-closed(JSON parse・schemaVersion・
`workflows` 非 array・bound 超過→error)、per-entry status
(`ok`/`missing`/`invalid`/`duplicate`)と diagnostics
(`malformed-entry`/`duplicate-id`/`missing-path`/
`path-out-of-workspace`/`denied-path`/`not-regular-file`)を実装する。
entry path の検証は `resolve_workspace_path` + `create_path_denylist`
+ `policy.filesystem` の既存入力のみを使い、独自の escape/symlink
判定を持たない。compiler・resource inference・file 内容 read は
一切行わない。

## Classification

- `normal-plan-task`

## Upstream Documents

- plan: `../plans/implement-workflow-catalog-and-list.md`
- design: `../designs/workflow-catalog-manifest-and-read-only-list.md`
- test-spec: `../test-specs/workflow-catalog-manifest-and-list-contract.md`

## Dependencies

- なし(最初の task)。

## Affected Files

- `crates/cheaper-workflow/src/catalog.rs` (new)
- `crates/cheaper-workflow/src/lib.rs`
- `crates/cheaper-workflow/tests/filesystem_contracts/catalog.rs` (new)
- `crates/cheaper-workflow/tests/filesystem_contracts/main.rs`

## Done When

- [ ] `catalog::read_catalog` が absent manifest を
  `present:false` の空 read として返す。
- [ ] malformed JSON・非対応 schemaVersion・bound 超過・`workflows`
  非 array が CheaperError(INVALID_INPUT 系)で失敗する。
- [ ] valid entry が `status:"ok"`、missing file が `"missing"`、
  path escape/deny/not-file が `"invalid"`、重複 id が両 entry
  `"duplicate"` + `duplicate-id` diagnostic になる。
- [ ] manifest 内の workflow file 本文を読まない(stat/resolve のみ)。
- [ ] `lib.rs` から必要な型・関数が export される。

## Verification

- [ ] `mise exec -- cargo test -p cheaper-workflow --test filesystem_contracts catalog::`
- [ ] `mise exec -- cargo fmt --all -- --check` と clippy が通る。
