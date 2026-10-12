---
id: "TASK-034EDQKID9bMzTe59QT2BD"
type: "task"
status: "todo"
title: "T12: cheaper-workflow RunState and schema compile sharing"
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

# T12: cheaper-workflow RunState and schema compile sharing

## Work

workflow engine の workaround 解消（spec Phase 1）。

- `engine.rs` の `Pump` 再構築 ×3 + `stop!` macro → `struct RunState`
  化（children/remaining/status/outer_error/timers を所有し pump/admit/
  execute/on_admission/on_executed/stop を method 化）
- 二重 validate/compile 解消: worker job の冗長 schema compile を
  `Arc<CompiledSchema>` 共有・`compile_validated` variant で解消。
  `JobValidation.input` は `JobKind::Tool` で使用中のため保持。
  tool job の input validation 意味論は不変

不変条件: event-loop 意味論・deterministic topological order・admission/
deadline/abort/result shape。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/execute-post-feature-structural-refactoring.md`
- `docs/designs/post-feature-structural-refactoring.md`

## Dependencies

- なし

## Affected Files

- `crates/cheaper-workflow/src/{engine.rs,compiler.rs,types.rs,schema.rs}`

## Done When

- [ ] Pump 再構築が除去され RunState が event loop state を所有
- [ ] `cheaper-cli/tests/envelope.rs`・regression case が緑

## Verification

- [ ] `mise exec -- cargo test -p cheaper-workflow -p cheaper-cli`（dev-seam 付き）が pass
