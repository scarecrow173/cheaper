---
id: "TSPEC-034MGCscrFncKerl6vu8Iw"
type: "test-spec"
status: "proposed"
title: "Execution History bounded storage lifecycle"
created: "2026-10-27"
updated: "2026-10-27"
owners: ["scarecrow173"]
relations:
  verifies:
    - "docs/specs/execution-history-issue-192.md"
    - "docs/designs/execution-history-issue-192.md"
    - "docs/adr/record-per-execution-history-with-bounded-best-effort-storage.md"
  derives-from:
    - "docs/specs/execution-history-issue-192.md"
    - "docs/designs/execution-history-issue-192.md"
  verified-by: []
---

# Execution History bounded storage lifecycle

## Purpose

Preserve the append-attempt, publication, terminal, and retention guarantees of the per-execution store. Retire only if the approved storage decision is superseded.

## Feature

Feature: History writes publish bounded per-execution lifecycle records best-effort.

## Rules

- A started record is published atomically from a private pending file; readers never treat partial JSON as a valid record.
- A terminal update is first-writer-wins and cannot replace a valid terminal record.
- Store initialization and cleanup are lazy and bounded; unrelated commands and read-only query utilities do not create the store.
- Retention distinguishes valid started records from invalid/missing lifecycle data and never promises hard caps beyond the documented scan budget.
- A write or cleanup failure does not alter the command result.

## Examples

- Given concurrent executions with distinct IDs, when they publish records, each valid file remains independently parseable.
- Given a valid started record and malformed terminal attempt, when read, the valid start remains observable as incomplete.
- Given a directory with no valid started or decision record, when the orphan age threshold is exceeded, bounded cleanup may remove it.
- Given a query-only command on a fresh config directory, when it lists history, it does not create storage.

## Guarantees

Tests pin atomic visibility, first-terminal-wins, best-effort failures, retention age/category semantics, scan/time budgets, and lazy directory creation.

## Non-goals

This spec does not guarantee successful persistence under disk failure, global ordering across concurrent executions, or hard storage caps when cleanup cannot complete.

## Risk

Without these guarantees, readers may observe corrupt or overwritten execution state, cleanup may erase valid starts, and utility commands may unexpectedly mutate user configuration.
