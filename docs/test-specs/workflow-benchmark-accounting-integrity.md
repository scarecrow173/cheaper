---
id: "TSPEC-034AR5JjpeSFMmwDJ2ohy3"
type: "test-spec"
status: "proposed"
title: "Workflow benchmark accounting integrity"
created: "2026-10-05"
updated: "2026-10-05"
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
    - "docs/designs/workflow-orchestration-benchmark.md"
  derived-by: []
  verifies:
    - "docs/specs/workflow-orchestration-benchmark.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Workflow benchmark accounting integrity

## Purpose

Preserve honest provenance for benchmark totals across workflow and direct CLI arms. Retire only if workflow execution receives a replacement aggregate contract for child metadata.

## Feature

Feature: Workflow and parent-orchestration accounting boundaries

## Rules

- Rule: A workflow aggregate is produced exactly once from its JSON envelope and `jobs[].meta`; workflow child jobs are not separately recorded in usage.jsonl.
- Rule: Direct CLI usage.jsonl records remain observations for the direct arm and are not added again as independent workflow aggregates.
- Rule: Parent usage fields are populated only from directly observed parent telemetry; otherwise they are explicitly unavailable.
- Rule: Scenario 1's seeded Node filesystem operation is a direct-work comparator, not a parent-agent usage record; a fake `cheaper run` comparator, if present, is separately identified.
- Rule: Scripted invocation count may proxy parent turns for manual orchestration, but it never proxies parent tokens.

## Examples

- Example: Given a workflow result with multiple Worker jobs, when aggregating it, read each job's metadata once and do not consult usage.jsonl for workflow children.
- Example: Given the equivalent direct invocation sequence, when usage.jsonl contains run entries, keep them in the direct arm instead of adding them to a workflow aggregate.
- Example: Given no parent runtime telemetry, when serializing the manual arm, mark parent tokens unavailable and invocation count only as a turns proxy.
- Example: Given scenario 1, when reporting the Node direct-tool comparator and workflow tool job, compare elapsed time and bytes only; do not label their difference as parent tokens saved.

## Guarantees

- Every reported metric has a source arm and source record identifier.
- The schema rejects missing or ambiguous parent usage and duplicate aggregate inputs.
- The documented manual arm states its CLI invocation sequence and limitations.

## Non-goals

- Whether scripted invocation count equals actual parent-agent turns.
- Cost estimates based on model pricing or unobserved context.

## Risk

- Accounting drift could fabricate savings or double-count usage, invalidating workflow selection guidance.
