---
id: "TASK-034MGH8FHwescRINQne4sz"
type: "task"
status: "done"
title: "Create bounded execution History core"
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
    - "TSPEC-034MGCscrFncKerl6vu8Iw"
    - "TSPEC-034MGCFe4bqwpXhKCEYgRA"
  references: []
  defers: []
  deferred-by: []
---

# Create bounded execution History core

## Work

Implement the shared History identity/schema/store API, per-execution pending-to-published lifecycle, bounded retention, first-terminal-wins behavior, and typed field projections. Add the CLI crate module registration and direct edge to the existing workspace UUID dependency. Keep all writes lazy and best-effort.

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-execution-history-for-issue-192.md`
- `docs/specs/execution-history-issue-192.md`
- `docs/adr/record-per-execution-history-with-bounded-best-effort-storage.md`

## Dependencies

- None; this is the prerequisite task for all instrumentation and read surfaces.

## Affected Files

- `crates/cheaper-cli/src/history/{mod,identity,schema,store,query}.rs`
- `crates/cheaper-cli/src/lib.rs`, `crates/cheaper-cli/Cargo.toml`, `Cargo.lock`

## Done When

- [ ] Valid started records publish atomically; terminal writes are first-writer-wins; failures remain best-effort.
- [ ] Bounded cleanup, lazy store creation, schema allowlists, size/numeric caps, and identity projection are covered by focused tests.
- [ ] Existing workspace UUID pin is reused without a new package/version or unrelated lock/tooling changes.

## Verification

- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam history::` exits 0 with named history tests.
