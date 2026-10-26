---
id: "TSPEC-034MGCxlMYuOemMCKIx9RZ"
type: "test-spec"
status: "proposed"
title: "Execution identities and Usage ownership"
created: "2026-10-26"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/execution-history-issue-192.md"
    - "docs/designs/execution-history-issue-192.md"
  derived-by: []
  verifies:
    - "docs/specs/execution-history-issue-192.md"
    - "docs/specs/usage-analysis-scope-issue-184.md"
    - "docs/adr/record-per-execution-history-with-bounded-best-effort-storage.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Execution identities and Usage ownership

## Purpose

Preserve one-to-one execution identity and the ownership boundary between
History evidence and `usage.jsonl`. Retire this spec only if the execution
cardinality or Usage ownership decision is superseded together with its
compatibility contract.

## Feature

Feature: History identities map observable work to the correct Usage owner
without double counting.

## Rules

- A standalone Worker invocation has one execution ID; an applicable `run`
  Usage record carries that same ID, including usage-bearing failures.
- A Workflow envelope has one execution ID and each Job has a child execution
  ID. Worker fields owned by a Job stay on the Job record and do not create a
  second standalone Worker `run` row.
- A Route run has its own execution ID and preserves existing `runId`/`logRef`;
  route-owned execution/postprocess Usage remains Route-owned. A refetch has a
  distinct execution ID and joins its source only by existing non-empty
  `logRef` semantics.
- Hook/Route decisions have History evidence but no Usage row. `count-tokens`
  remains outside Issue 192 History and keeps its existing `kind:"count"`
  record unchanged.
- Same `traceId` never deduplicates Usage rows. History and Usage writes are
  independent best-effort operations; absence on either side never fabricates
  or backfills the other.

## Examples

- Given a Worker fails before usage is available, when its execution closes,
  then History may show the failure but no Usage row is invented.
- Given a Workflow Job runs a Worker, when Usage is recorded, then the Job
  History identity carries the Worker metrics and no duplicate `run` row is
  emitted for that same work.
- Given a Route and a refetch share a `logRef`, when Usage is aggregated, then
  the Route and independent refetch identities remain distinct and #184 joins
  only the in-window route owner.
- Given two standalone runs share a trace, when records are read, then both
  executions and both applicable Usage rows remain independently countable.

## Guarantees

- Fixtures covering Worker, Workflow/Job, Route, refetch, decisions, and
  `count-tokens` prove the ownership table and absence of duplicate rows.
- Existing Usage aggregation and refetch ownership remain unchanged for legacy
  records without IDs; History absence never alters aggregate totals.
- A failed History append or failed Usage append is observable only as missing
  linkage, never as guessed ownership or a changed product result.

## Non-goals

- This spec does not test the internal JSON schema, privacy allowlist, query
  budgets, or storage retention.
- It does not backfill historical Usage rows, change `usage --since`, or use
  parent/trace traversal as a replacement for the established `logRef` join.

## Risk

- A wrong owner silently inflates or suppresses cost/token aggregates, and a
  Job-level duplicate can make one piece of work appear twice. Inferring
  linkage from History availability would also make telemetry failure alter
  accounting semantics.
