---
id: "TASK-034EDQKcQM3ZUANOmMgg6p"
type: "task"
status: "done"
title: "T11: cheaper-tools process/grep/read splits"
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
    - "docs/tasks/p1-tools-characterization-tests.md"
    - "docs/tasks/p0-tools-dead-assets.md"
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

# T11: cheaper-tools process/grep/read splits

## Work

tools の巨大ファイルを責務単位に分割する（spec Phase 1。T07 の
characterization test 完了が前提）。

- `src/tools/process/mod.rs` → `process/{prepare,spawn,monitor,cleanup}.rs`
- `src/tools/grep.rs` orchestration 分割（execute_grep から scan loop/
  footer 分離。`GrepEvalWorker` seam は維持）
- `src/tools/read.rs` 3 mode 分離（artifact read/publication/workspace
  read_bounded）

不変条件: scheduler/resources/operation の 3 者分離・deferred callback・
lock 順序（resources > scheduler）・`glob/mod.rs`・`line_framer.rs` は
対象外。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/execute-post-feature-structural-refactoring.md`
- `docs/designs/post-feature-structural-refactoring.md`

## Dependencies

- `docs/tasks/p1-tools-characterization-tests.md`（必須先行）
- `docs/tasks/p0-tools-dead-assets.md`（同一 crate の P0 整理に先行依存）

## Affected Files

- `crates/cheaper-tools/src/tools/process/{mod,prepare,spawn,monitor,cleanup}.rs`
- `crates/cheaper-tools/src/tools/{grep.rs,read.rs}`

## Done When

- [ ] 3 file が責務単位に分割され characterization/differential test が緑
- [ ] process cleanup 順序・grep stop reason・UTF-16/code-unit parity 不変

## Verification

- [ ] `mise exec -- cargo test -p cheaper-tools` が pass（differential
      fixture 10 件含む）
