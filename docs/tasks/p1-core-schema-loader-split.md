---
id: "TASK-034EDQLedhfXM1TMwrV4jG"
type: "task"
status: "done"
title: "T13: cheaper-core worker schema and loader splits"
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
    - "docs/tasks/p0-core-shared-helpers.md"
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

# T13: cheaper-core worker schema and loader splits

## Work

core の巨大 file 分割と loader 共通化（spec Phase 1）。

- `src/worker/schema.rs`（~1,007 行）→ `worker/schema/{mod,strict}.rs`
  （AJV strict 再実装 ~500 行を strict.rs へ機械的移動。公開 API 不変）
- `read_policy_file`/`read_models_config` の共通骨格抽出（error reason
  文字列の parity を維持 — 差分が出ないことを差分確認）

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/execute-post-feature-structural-refactoring.md`
- `docs/designs/post-feature-structural-refactoring.md`

## Dependencies

- `docs/tasks/p0-core-shared-helpers.md`（同一 crate の helper 集約に先行依存）

## Affected Files

- `crates/cheaper-core/src/worker/schema/{mod.rs,strict.rs}`
- `crates/cheaper-core/src/{policy/load.rs,model/config.rs}`

## Done When

- [ ] schema.rs が strict 分離され公開 API 不変
- [ ] loader 共通骨格で error reason 文字列の parity を維持

## Verification

- [ ] `mise exec -- cargo test -p cheaper-core -p cheaper-cli` が pass
