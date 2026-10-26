---
id: "IMPL-034MGQ2A2LscE3yZbiOkGB"
type: "impl"
status: "completed"
title: "Create bounded execution History core"
created: "2026-10-27"
updated: "2026-10-27"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified:
      - { type: file, path: crates/cheaper-cli/Cargo.toml }
      - { type: file, path: crates/cheaper-cli/src/lib.rs }
      - { type: file, path: crates/cheaper-cli/src/history/mod.rs }
      - { type: file, path: crates/cheaper-cli/src/history/identity.rs }
      - { type: file, path: crates/cheaper-cli/src/history/schema.rs }
      - { type: file, path: crates/cheaper-cli/src/history/store.rs }
      - { type: file, path: crates/cheaper-cli/src/history/query.rs }
      - { type: file, path: Cargo.lock }
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/execution-history-core.md"
  implemented-by: []
  depends-on: []
  blocks: []
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
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Create bounded execution History core

## Summary

Implemented approved task `TASK-034MGH8FHwescRINQne4sz`: shared History identity, schema/projection, bounded store lifecycle, and read-only query primitives. A dedicated worker owned only the planned core files.

## Context

Follow `docs/plans/implement-execution-history-for-issue-192.md`, accepted storage ADR `docs/adr/record-per-execution-history-with-bounded-best-effort-storage.md`, design `docs/designs/execution-history-issue-192.md`, and linked lifecycle/privacy test specs. The scope is shared schema/store/module registration and the existing UUID dependency edge.

## Implementation

### Changes

Added History module registration and a direct dependency on the existing pinned workspace UUID. Implemented `ExecutionIdentity`, `HistoryEvent`, `HistoryStore`, `HistoryQuery`, and `HistoryCursor`. Store uses lazy per-user-config creation, pending-to-published directory rename, create-new terminal publication, bounded best-effort sweep, and no side effects on reads. History projection enforces typed fields, size/numeric bounds, and allowlisted identities.

### Decisions During Implementation

Reuse the existing workspace UUID v4 pin. No package/version or APM/mise lock changes are in scope.

## Related Experiments

### Adopted

No experiment log; the approved plan provided the implementation direction.

### Rejected

None.

## Validation

TDD: the first focused run failed because `ExecutionIdentity` was absent; identity tests passed after implementation. Final checks: `mise exec -- cargo fmt --check -p cheaper-cli` passed; `mise exec -- cargo clippy -p cheaper-cli --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam -- -D warnings` passed; `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam history::` passed 22 named tests, 0 failures (31.37s); `git diff --check` passed.

## Risks

Only Windows execution was performed for this slice. Canonical workspace check and POSIX run remain for final integration verification. The 20,001-directory scan-budget fixture accounts for most focused test runtime.

## Follow-ups

Task complete; proceed with dependency-ready lifecycle owners. The implementation will receive the planned final independent branch review.
