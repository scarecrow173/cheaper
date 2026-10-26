---
id: "TSPEC-034MGCCC9F0R4NwYCGFOlk"
type: "test-spec"
status: "proposed"
title: "Hook and Route decision evidence"
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

# Hook and Route decision evidence

## Purpose

Preserve the distinction between a Hook/Route decision, delegation advice, and
an actual downstream execution. Retire this spec only if typed decision
evidence is replaced while native Hook and Route contracts remain explicitly
unchanged.

## Feature

Feature: Hook and Route decisions emit bounded typed History evidence without
changing native verdict behavior or claiming unobserved execution.

## Rules

- A meaningful Hook/Route evaluator result writes one decision record with
  finite reason/outcome fields, typed measurements, Route viability, and both
  evaluator and final native verdict where they differ.
- New Hook records use separate `evaluatorVerdict` and `nativeVerdict`, optional
  `fallbackReason`, and finite `hookCacheProvenance`; generic `verdict` is not
  duplicated on those records. Search reasons come only from observed typed
  branches (`search.no_candidate`, `search.below_threshold`,
  `search.threshold_exceeded_delegate_advised`, `search.scan_limited`, or
  `search.measurement_unavailable`).
- Hook Deny/delegation advice is not a Worker start; Rewrite emission and an
  eventual Route run are separate records. Unsupported Rewrite records the
  fallback Deny without claiming the external host executed it.
- Route evaluation, no-match/no-grant/no-pin/not-runnable, rewrite/fallback,
  and an actual Route run remain distinguishable; existing `runId` and `logRef`
  are preserved.
- Deduplicated evaluation records are written only by the cache leader that
  produced and published the typed evidence; cache hits/followers do not
  duplicate decisions, while explicit bypass/fallback may record its result.
- Decision persistence is best-effort and cannot change stdout/stderr, exit
  code, grant/policy decisions, or Route/Worker execution behavior.

## Examples

- Given a read below threshold or byte fast path, when the evaluator returns,
  then the decision records its typed reason and measurement without a Worker
  execution record.
- Given a Route with no match or no grant, when evaluation finishes, then the
  corresponding viability/reason is recorded and no Route run is implied.
- Given supported Rewrite and unsupported Rewrite outcomes, when native targets
  render the response, then evaluator Rewrite and fallback Deny remain
  separate while native output stays unchanged.
- Given each observed Search outcome, when evidence is recorded, then only the
  matching finite reason and actually measured bounded metrics are persisted;
  raw query/error text is absent.
- Given concurrent cache followers for one evaluation, when evidence is
  returned, then only the leader creates History and all callers observe the
  same verdict contract.

## Guarantees

- Contract fixtures cover allow/deny/rewrite, measurement-limited/null values,
  Route viability, fallback, cache provenance, and native output parity.
- The record contains no path, raw Hook input, command text, prompt, tool
  payload, or free-form error; only allowlisted identifiers and bounded
  measurements are projected.
- A later external host action is never inferred, and a missing decision write
  never causes a Worker/Route or Usage row to be synthesized.

## Non-goals

- Complete Hook event auditing, external host behavior after stdout, and the
  full Route execution lifecycle are outside this spec.
- Storage publication/retention, query pagination, and Usage aggregation are
  covered by the corresponding Issue 192 test specs and existing contracts.

## Risk

- Conflating advice with execution can mislead operators and produce phantom
  runs. Duplicate cache evidence inflates history, while changing native output
  or leaking Hook input can break integrations and expose sensitive data.
