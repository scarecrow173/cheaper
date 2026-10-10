---
id: "PLAN-034DlEwsubKsxcWfA08Kb4"
type: "plan"
status: "approved"
title: "devin usage accounting contract test and docs"
created: "2026-10-10"
updated: "2026-10-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/86"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/devin-provider-usage-accounting-contract.md"
    - "docs/adr/keep-devin-uncached-input-usage-passthrough-and-document-cache-semantics.md"
  implemented-by: []
  depends-on:
    - "docs/designs/devin-usage-accounting-contract-test-and-docs.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/devin-usage-accounting-contract-test-and-docs.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  plan:
    effort: "small"
    verification-policy: "task-verified"
test-spec-skip: "Contract-pin tests ARE the deliverable of this plan;
  no separate executable test-spec needed — the new tests are
  specified by spec R4 and asserted via cargo test."
---

# devin usage accounting contract test and docs Implementation Plan

## Goal

Pin the devin provider's usage-accounting contract (uncached `input`,
separate `cacheRead`/`cacheWrite`, `totalTokens = input + output`)
with regression tests and document the semantic in the two consumer
guides — with zero production-code change (issue #86).

## Scope

Single implementation stream: one test file edit (helper hoist + two
new tests) and two guide doc edits. No production code changes.

## Upstream Documents

- `docs/specs/devin-provider-usage-accounting-contract.md` (approved)
- `docs/adr/keep-devin-uncached-input-usage-passthrough-and-document-cache-semantics.md`
  (accepted)
- `docs/discovery/devin-provider-usage-accounting-reports-uncached-input-only.md`

## Design Inputs

- `docs/designs/devin-usage-accounting-contract-test-and-docs.md`
  (approved)

## File Map

- `crates/cheaper-provider-devin/src/tests/stream.rs` — hoist the
  nested `metric(name, value)` helper to module scope; add
  `decode_chat_frame_maps_devin_cache_hit_usage_shape` (issue-#86
  shape + boundary coverage) and a cache-metrics-only frame test.
- `docs/guides/providers.md` — add a `usage 計上` bullet in the
  Devin Local section after the credential bullets, before
  `model catalog` (~line 238-239).
- `docs/guides/usage.md` — add one sentence to the `meta.usage`
  field description (~lines 61-62).

## Implementation Sequence

- [ ] In `crates/cheaper-provider-devin/src/tests/stream.rs`, move
      the `metric(name, value)` helper out of
      `decode_chat_frame_reads_usage_metrics` to module scope
      (mechanical hoist; helper captures nothing).
- [ ] Add test `decode_chat_frame_maps_devin_cache_hit_usage_shape`:
      metrics `input_tokens=1`, `output_tokens=211`,
      `cached_input_tokens=3196`, `cache_creation_input_tokens=7`,
      `total_tokens=9999`; assert
      `ChatEvent::Usage(ChatUsage{prompt_tokens:Some(1),
      completion_tokens:Some(211), total_tokens:Some(212),
      cached_input_tokens:Some(3196),
      cache_creation_input_tokens:Some(7)})`.
- [ ] Add a second test asserting a frame containing only
      `cached_input_tokens` produces no `ChatEvent::Usage` event,
      with a comment noting it pins *current* behavior (deferred
      concern in the spec).
- [ ] `cargo test -p cheaper-provider-devin` — confirm all tests
      pass.
- [ ] Insert the `usage 計上` bullet in `docs/guides/providers.md`
      (devin reports uncached input in `input`; `cacheRead` carries
      the cache-hit portion; `totalTokens = input + output`;
      full prompt volume = `input + cacheRead`; `costUsd` is 0 when
      the resolved devin price is all zeros).
- [ ] Insert the parallel note in `docs/guides/usage.md` next to the
      `meta.usage` field list (~lines 61-62).
- [ ] `git diff crates/cheaper-provider-devin/src/` — confirm the
      only source diff is `src/tests/stream.rs`.
- [ ] `mise run check`.

## Dependencies and Risks

- None blocking; docs-only-plus-test change. Risk that the pinned
  values enshrine behavior that could shift if Devin's wire format
  changes — accepted per ADR; spec deferred concerns cover it.

## Task Breakdown

- Task 1: contract-pin tests in
  `crates/cheaper-provider-devin/src/tests/stream.rs`
  (helper hoist + two tests + `cargo test -p cheaper-provider-devin`).
- Task 2: guide documentation in `docs/guides/providers.md` and
  `docs/guides/usage.md`. (Independent of task 1; may be done in any
  order — sequence as listed for single-author simplicity.)

## Verification

- [ ] `cargo test -p cheaper-provider-devin` — all tests pass,
      including the two new ones.
- [ ] `git diff crates/cheaper-provider-devin/src/` shows changes in
      `src/tests/` only (zero production-code diff).
- [ ] Both guide notes present and consistent with spec R5.
- [ ] `mise run check` passes.

## Open Questions / Gaps

- None for implementation. Deferred (spec): wire `total_tokens`
  semantics if Devin's format changes; devin unknown→free pricing
  conflation; `toolCalls` vs `sourceTokens` cross-check when
  reporting back on issue #86.

## Self-Review

- [x] Every spec requirement maps to a task (R1-R4 → Task 1 tests;
      R5 → Task 2 docs).
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are
      consistent.

## Implementation Handoff

- Recommended mode: inline execution (small, well-specified diff).
- Independent work streams: none — two sequential tasks.
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation
  capabilities in the current environment, then execute task-by-task
  with review checkpoints.
- If declined or unavailable: execute inline using the same
  dependency graph and verification matrix.

## Review Handoff

- Reviewers should confirm: the tests pin the mapping without
  touching production code; helper hoist is mechanical; guide notes
  are consistent with providers.md / usage.md style and Japanese
  prose; `cache_creation_input_tokens` lands in `cacheWrite` not
  `cacheRead` (substring boundary); wire `total_tokens` is ignored.
