---
id: "DESIGN-034DlAaOWn3RyI3onq5uj1"
type: "design"
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
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/devin-provider-usage-accounting-reports-uncached-input-only.md"
    - "docs/adr/keep-devin-uncached-input-usage-passthrough-and-document-cache-semantics.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/devin-provider-usage-accounting-contract.md"
    - "docs/adr/keep-devin-uncached-input-usage-passthrough-and-document-cache-semantics.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# devin usage accounting contract test and docs

## Context

Issue #86: `meta.usage` for a devin worker run showed
`{input: 1, output: 211, cacheRead: 3196, totalTokens: 212,
costUsd: 0}` and looked like a field-mapping bug. Investigation
(confirmed against `stream.rs` decode, the accumulator, and the
vendored TS original) shows it is Devin's Anthropic-style cache
accounting reported verbatim: `input_tokens` is the uncached
portion, `cached_input_tokens` is the cache-hit portion, and
`totalTokens` is defined as `input + output`.

Per the approved spec and ADR, the fix is a contract pin, not a
behavior change: a regression test that locks the mapping, plus
documentation of the semantic for consumers. No production code
changes.

## Scope

### In Scope

1. New test(s) in `crates/cheaper-provider-devin/src/tests/stream.rs`
   pinning the cache-hit usage mapping at the `decode_chat_frame` →
   `ChatEvent::Usage(ChatUsage)` seam.
2. A short usage-semantics note added to the Devin section of
   `docs/guides/providers.md` (after the existing Windows credential
   snapshot bullet, before `model catalog`).
3. A parallel note in `docs/guides/usage.md` where `meta.usage`
   fields are defined, telling consumers devin `input` is the
   uncached portion and full prompt volume is `input + cacheRead`.

### Out of Scope

- Any change to `decode_usage`, `rig_provider.rs`, the accumulator,
  `meta.rs`, `usage_report.rs`, or `fixture.rs`.
- Pricing/catalog changes (devin unknown→free conflation is a
  deferred concern in the spec).
- `savings.sourceTokens` semantics.

## Components and Boundaries

- `crates/cheaper-provider-devin/src/tests/stream.rs` — existing
  `decode_chat_frame_reads_usage_metrics` builds metric records via
  a local `metric(name, value)` helper; the new test reuses that
  style.
- `decode_usage` contract being pinned
  (`crates/cheaper-provider-devin/src/stream.rs:395-455`):
  - `input_tokens` → `prompt_tokens` → `usage.input`
  - `output_tokens` → `completion_tokens` → `usage.output`
  - `contains("cached") || contains("cache_read")` →
    `cached_input_tokens` → `usage.cacheRead`
  - `contains("cache_creation")` → `cache_creation_input_tokens` →
    `usage.cacheWrite`
  - `total_tokens = prompt + completion` (recomputed; a wire
    `total_tokens` metric matches no branch and is ignored)
  - no metrics at all → `ChatEvent::Usage` not emitted unless prompt
    or completion is present.

## Data and Control Flow

New test
`decode_chat_frame_maps_devin_cache_hit_usage_shape` in
`src/tests/stream.rs`:

1. Extract the `metric(name, value)` helper from
   `decode_chat_frame_reads_usage_metrics` to module scope so both
   tests share it (mechanical hoist, no behavior change).
2. Build a usage frame reproducing the issue-#86 shape plus the
   coverage points:
   - `input_tokens = 1`, `output_tokens = 211`,
     `cached_input_tokens = 3196`
   - `cache_creation_input_tokens = 7` → must land in
     `cache_creation_input_tokens`, not `cached_input_tokens`
     (substring-match boundary)
   - `total_tokens = 9999` → ignored; assert recomputed
     `Some(1 + 211)`
3. Assert `ChatEvent::Usage(ChatUsage { prompt_tokens: Some(1),
   completion_tokens: Some(211), total_tokens: Some(212),
   cached_input_tokens: Some(3196),
   cache_creation_input_tokens: Some(7) })`.
4. A second small test asserts a cache-metrics-only frame (only
   `cached_input_tokens`, no prompt/completion) yields no usage
   event — pinning the documented latent edge, not changing it
   (test comment notes it records *current* behavior per the spec's
   deferred concerns).

Docs:

- `docs/guides/providers.md`, Devin Local section — add a
  `usage 計上` bullet after the credential bullets: devin reports
  uncached input only; `cacheRead` carries the cache-hit portion;
  `totalTokens = input + output` excludes `cacheRead` by contract;
  full prompt volume = `input + cacheRead`; `costUsd` currently
  resolves to 0 for devin models whose `cost_summary` is absent
  (all-zero resolved price).
- `docs/guides/usage.md`, the `meta.usage` field list
  (`usage`（input/output/cacheRead/cacheWrite/totalTokens/costUsd）,
  ~lines 61-62) — add one sentence: providers with prompt caching
  (e.g. devin) report uncached input in `input` and the hit portion
  in `cacheRead`; `totalTokens` is `input + output`, so use
  `input + cacheRead` for full prompt volume.

## Non-Functional Constraints

- Zero production-code diff: the deliverable is a contract pin plus
  prose; any behavior change is out of scope and would re-enter
  through a new spec.
- Japanese prose style per guide convention (`docs/guides/` is
  Japanese).

## Risks and Trade-offs

- Pinning current behavior could enshrine a future-wrong value if
  Devin changes wire semantics — accepted; the test documents
  *implemented* behavior, and the spec's deferred concerns cover the
  wire-format drift case.
- The `input + cacheRead` guidance could confuse consumers of
  providers whose `input` already includes cache — mitigated by
  scoping the note to providers that report cache metrics.

## Deferred Design Concerns

- All items in the spec's Deferred Design Concerns (wire
  `total_tokens` semantics, substring-matching hazards, cache-only
  frame drop, devin unknown→free pricing conflation, fixture
  total-fallback divergence, cross-provider `input` normalization).

## Verification Notes

- [ ] `cargo test -p cheaper-provider-devin` passes with the new
      tests.
- [ ] `git diff crates/cheaper-provider-devin/src/` shows no
      production-code change (tests file only).
- [ ] `docs/guides/providers.md` and `docs/guides/usage.md`
      updated.
- [ ] `mise run check` passes.
