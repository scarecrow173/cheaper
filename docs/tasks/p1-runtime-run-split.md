---
id: "TASK-034EDQJaU8cr8MkalDStBT"
type: "task"
status: "todo"
title: "T09: cheaper-runtime run module split and auth/provider extraction"
created: "2026-10-12"
updated: "2026-10-12"
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
    - "docs/plans/execute-post-feature-structural-refactoring.md"
  implemented-by: []
  depends-on:
    - "docs/plans/execute-post-feature-structural-refactoring.md"
    - "docs/tasks/p0-runtime-provider-adapter-helpers.md"
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
    - "docs/test-specs/post-feature-structural-refactoring-parity.md"
  references: []
  defers: []
  deferred-by: []
---

# T09: cheaper-runtime run module split and auth/provider extraction

## Work

runtime run module の分割と auth/provider 周辺の整理（spec Phase 1）。

- `run/mod.rs` → `run/{mod,session,boundary}.rs`、`RunCtx::run` を
  private stage method に分割（stage 順序・分岐は不変）
- `src/auth/oauth/openai_codex.rs` の in-file test ~850 行を crate 内
  sibling file へ分離（`openai_codex/` dir 化 + `tests.rs`。
  private item 依存のため `tests/` 外部 target には移さない）
- `ProviderEntry` の refresh state sub-struct 化 + `compose_provider`
  の auth 層抽出
- atomic write helper 共通化（`auth/store.rs` vs `catalog/persisted.rs`
  の Windows 事前削除差分は option 化で両立）

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/execute-post-feature-structural-refactoring.md`
- `docs/designs/post-feature-structural-refactoring.md`

## Dependencies

- `docs/tasks/p0-runtime-provider-adapter-helpers.md`

## Affected Files

- `crates/cheaper-runtime/src/run/{mod.rs,session.rs,boundary.rs}`
- `crates/cheaper-runtime/src/auth/oauth/openai_codex*`
- `crates/cheaper-runtime/src/auth/store.rs`・`catalog/persisted.rs`
- `ProviderEntry`/`compose_provider` 関連 file

## Done When

- [ ] run module が session/boundary に分割され挙動不変
- [ ] in-file test が分離され、公開 API・可視性が不変

## Verification

- [ ] `mise exec -- cargo test -p cheaper-runtime`（dev-seam 付き）が pass
