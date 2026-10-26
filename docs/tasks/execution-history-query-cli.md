---
id: "TASK-034MGHz9sAMkZkvZW0WFgZ"
type: "task"
status: "in-progress"
title: "Add read-only History CLI and bounded queries"
created: "2026-10-27"
updated: "2026-10-27"
owners: ["scarecrow173"]
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
    - "docs/plans/implement-execution-history-for-issue-192.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-execution-history-for-issue-192.md"
    - "TASK-034MGH8FHwescRINQne4sz"
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
    - "TSPEC-034MGCuNnSmgxjF4fq9Cea"
  references: []
  defers: []
  deferred-by: []
---

# Add read-only History CLI and bounded queries

## Work

Implement the History list/show/trace grammar, bounded read-only queries, pagination, stable text/JSON output, and integration-test registration for History CLI. Preserve `cheaper route resolve` read-only behavior.

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-execution-history-for-issue-192.md`

## Dependencies

- `docs/tasks/execution-history-core.md` (`TASK-034MGH8FHwescRINQne4sz`)

## Affected Files

- `crates/cheaper-cli/src/args.rs`, `cli.rs`, `text_format.rs`, `src/history/query.rs`
- `crates/cheaper-cli/tests/integration/main.rs` and History integration tests

## Done When

- [ ] List/show/trace enforce approved scan and per-file bounds; incomplete scans return no cursor.
- [ ] Reads create no History/Usage records and `route resolve` stays side-effect-free.
- [ ] Integration module is explicitly registered and filtered test listing is nonempty.

## Verification

- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration history:: -- --list` lists named History tests; then `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration history::` exits 0.
