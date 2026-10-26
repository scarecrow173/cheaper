---
id: "TSPEC-034MGCuNnSmgxjF4fq9Cea"
type: "test-spec"
status: "proposed"
title: "Execution History bounded read-only queries"
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

# Execution History bounded read-only queries

## Purpose

Preserve the read-only and bounded-work contract of `cheaper history list`,
`show`, and `trace`. Retire this spec only if the History query interface and
its explicit incomplete-result semantics are superseded.

## Feature

Feature: History queries inspect retained records without creating or mutating
the store.

## Rules

- List and trace order records by `recordedAt` descending, then
  `executionId` descending, and use that pair for an opaque keyset cursor.
- List and trace use a default page size of 50 and accept at most 500 records;
  `show` reads only the requested UUID directory and its known event files.
- Every query is bounded by 20,000 execution directories, 128 MiB of event
  data, or 500 ms. Reaching a limit returns the rows found so far with
  `scanComplete:false` and no continuation cursor.
- Malformed, truncated, oversized, or unrelated event files are ignored while
  valid records remain visible; a missing terminal is reported as incomplete
  or unknown rather than as a fabricated success.
- A missing store remains missing: query commands do not create directories,
  write Usage or History records, run retention, or execute work.

## Examples

- Given valid started records with equal timestamps, when `list` is paged,
  then UUID descending order and the returned cursor are stable.
- Given a retained dataset that exceeds the directory or time budget, when
  `trace` scans it, then the response is explicitly incomplete and has no
  cursor that implies all remaining rows are reachable.
- Given a valid started file and a truncated terminal file, when `show` reads
  the execution, then the started data is returned with incomplete state.
- Given an absent `<userConfigDir>/history/`, when `list` or `show` runs, then
  the filesystem remains unchanged and an empty/not-found result is returned.

## Guarantees

- Query output has deterministic ordering, bounded page and scan work, and an
  explicit distinction between complete and incomplete scans.
- Valid records survive neighboring corruption, and query output never
  exposes raw malformed contents or performs a write as a side effect.
- Text and JSON forms agree on records and incomplete/not-found state.

## Non-goals

- Writer publication, terminal first-writer-wins, and retention deletion are
  covered by the storage-lifecycle spec.
- This spec does not change `usage` aggregation, backfill missing History, or
  promise pagination beyond a dataset fully scanned within the stated budget.

## Risk

- Unbounded or mutating queries can hang a CLI invocation, recreate user
  configuration unexpectedly, or turn a partial scan into a false complete
  history. Dropping corruption isolation can also hide valid executions behind
  one malformed file.
