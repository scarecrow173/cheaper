---
id: "TSPEC-034MGCzUAhlbOn8zU2Mnal"
type: "test-spec"
status: "proposed"
title: "Workflow Job History lifecycle"
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
    - "docs/designs/execution-history-issue-192.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Workflow Job History lifecycle

## Purpose

Preserve Workflow Job lifecycle evidence at the state-machine boundary,
including Jobs that never reach an executor. Retire this spec only if the
Workflow observer contract is superseded without losing parent, skip, blocked,
or terminal evidence.

## Feature

Feature: Workflow Job transitions produce correlated History records without
changing engine outcomes or Usage cardinality.

## Rules

- The engine emits exactly one observer `Started` transition before executor
  dispatch and one terminal transition for completed, failed, cancelled,
  timeout, skipped, blocked, or admission-failure states.
- The CLI observer maps each logical `job_id` to a fresh UUID execution ID and
  uses the Workflow execution UUID as the actual parent; logical IDs are not
  reused as execution IDs.
- A terminal event with a prior start writes an immutable terminal record. A
  terminal-only skip/blocked/admission failure writes a decision-only Job
  record instead of fabricating a start.
- Worker identity and metrics observed within a Job belong to that Job record;
  the observer never emits a standalone Worker Usage `run` row for the same
  work.
- Observer callbacks are synchronous, bounded, and best-effort outside the
  engine state lock. Persistence failure cannot abort a state transition or
  change the Workflow result.

## Examples

- Given a Job is admitted and runs, when it enters running and then completes,
  then one child execution has the Workflow parent and a terminal record.
- Given dependency failure prevents a Job from running, when the engine marks
  it skipped or blocked, then a terminal-only decision preserves the reason and
  dependency context without claiming Worker start.
- Given a later Job fails or the process exits, when earlier Job records are
  queried, then their starts and terminals remain independently observable.
- Given History storage is unavailable, when a Job transition is committed,
  then the Workflow state and final result are unchanged.

## Guarantees

- Lifecycle fixtures pin exactly-once observer transitions, distinct UUIDs,
  actual Workflow parent edges, skip/blocked/admission states, and worker
  metric projection.
- Job evidence is emitted at transition ownership rather than reconstructed
  from a final Workflow summary, so parallel and unexecuted Jobs remain
  diagnosable.
- Job History cannot double-count Usage or introduce a dependency from the
  workflow engine to CLI History schema types.

## Non-goals

- This spec does not define Workflow scheduling/lease semantics, general
  storage/query budgets, privacy projection, or standalone Worker lifecycle.
- It does not require a History record for parser failures before a Workflow
  execution boundary is identified.

## Risk

- Reconstructing Jobs from the final summary can erase skipped/blocked work and
  the point of failure. Reusing logical IDs or emitting Worker Usage rows can
  break correlation and inflate accounting; observer failures must not make
  workflow execution fail.
