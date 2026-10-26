---
id: "IMPL-034MpAHqkHvEuevjW9HP58"
type: "impl"
status: "completed"
title: "Measure usage JSONL peak memory issue 184 T5"
created: "2026-10-26"
updated: "2026-10-26"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "file"
        path: "docs/impl/exp/README.md"
      - type: "file"
        path: "docs/tasks/measure-usage-jsonl-peak-memory.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/impl/exp/usage-jsonl-peak-memory-measurement.jsonl"
  implements:
    - "docs/tasks/measure-usage-jsonl-peak-memory.md"
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
  references:
    - "docs/plans/implement-usage-route-error-analysis-issue-184.md"
    - "docs/designs/usage-grouped-route-error-analysis-issue-184.md"
    - "docs/specs/usage-analysis-scope-issue-184.md"
    - "docs/test-specs/usage-route-error-analysis-contract-issue-184.md"
    - "docs/impl/exp/usage-jsonl-peak-memory-measurement.jsonl"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/usage-jsonl-peak-memory-measurement.jsonl"
    rejected: []
---

# Measure usage JSONL peak memory issue 184 T5

## Summary

Measured CLI peak working set against a reproducible 82,300-record synthetic
fallback anchored at 100x the observed local usage log line count. Streaming
remains deferred; no pass/fail memory threshold was specified.

## Context

Implements the measurement-only T5 task for Issue #184. Do not change the
report implementation or add a committed fixture generator. Record actual
dimensions and avoid inventing a threshold.

## Implementation

### Changes

- Observed `C:\Users\akiyoshi\.config\cheaper\usage.jsonl` metadata only:
  823 lines/257,336 bytes (197 run, 626 count, no route/refetch). Contents were
  not copied. The documented fallback is 82,300 records/29,892,700 bytes:
  61,725 run, 16,460 route, 4,115 route-refetch (assumed 75/20/5 mix).
- Measured `target/debug/cheaper.exe usage --group-by route` in a child process
  while sampling `PeakWorkingSet64` every 5ms. Three runs on the same SHA256
  `F3B09EB8702A6153449C5A9F951CC26E67DA690F4CB6AF65C2ACB6100E897B02` returned
  363,565,056 / 363,585,536 / 363,737,088 bytes (346.72–346.89 MiB), all exit 0.
- Recorded observation and validation events under the task-required
  `docs/impl/exp/usage-jsonl-peak-memory-measurement.jsonl`.

### Decisions During Implementation

- Actual local data contains no route records and is only 823 lines. The route
  distribution and 100x line-count scale are explicit fallback assumptions,
  not observed production proportions.
- Keep `Vec<Value>` for this issue and leave streaming deferred. The 347 MiB
  value is a baseline, not a threshold pass; revisit design if real logs grow
  or an explicit budget is specified.

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

- Re-run on identical fixture hash/dimensions: third peak 346.89 MiB vs
  346.72/346.74 MiB in the first two runs.
- `usage_report::`: 65 passed / 0 failed.
- `audit_experiment_log.js --dir docs/impl/exp --json`: 7 files / findings 0.

## Risks

- Peak working set is platform/runtime specific and the synthetic route mix may
  differ from production; the event records OS/toolchain and assumptions.

## Follow-ups

- T5 is complete. Run T6 canonical verification and document graph closeout.
