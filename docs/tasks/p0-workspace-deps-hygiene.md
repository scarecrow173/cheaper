---
id: "TASK-034EDQo3a5Fsfj8WJj7D0Q"
type: "task"
status: "todo"
title: "T06: workspace deps and repo hygiene"
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

# T06: workspace deps and repo hygiene

## Work

横断 hygiene 項目（spec Phase 0 横断）。

- `workspace.dependencies` へ `uuid`/`base64`/`tempfile` を集約。
  `uuid` は workflow `=1.27.0` exact pin と provider `1` の差異あり —
  厳格側 `=1.27.0` を workspace に置くか差異を明文化するか実装時に確定
  し、task doc / commit に記録する
- `.gitignore` の Node boilerplate 整理（Rust product で不要な項目を削減）
- `benchmarks/results/` の保持方針を明文化（benchmarks 配下 README 等）

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/execute-post-feature-structural-refactoring.md`
- `docs/specs/post-feature-structural-refactoring-scope.md`

## Dependencies

- なし

## Affected Files

- `Cargo.toml`・各 `crates/*/Cargo.toml`
- `.gitignore`・`benchmarks/`

## Done When

- [ ] 3 依存が workspace deps に集約され lockfile check が pass
- [ ] `.gitignore` と benchmarks 保持方針が現状に即している

## Verification

- [ ] `mise exec -- cargo build --workspace`・`node scripts/check-lockfile.mjs` が pass
