---
id: "TASK-034DlFISpHRPrXyhhlFXfa"
type: "task"
status: "done"
title: "Pin devin cache-hit usage mapping in stream tests"
created: "2026-10-10"
updated: "2026-10-10"
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
  implements:
    - "docs/plans/devin-usage-accounting-contract-test-and-docs.md"
  implemented-by:
    - "docs/impl/ir/devin-usage-accounting-contract-pin-tests-and-guide-notes.md"
  depends-on:
    - "docs/plans/devin-usage-accounting-contract-test-and-docs.md"
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
---

# Pin devin cache-hit usage mapping in stream tests

## Work

Hoist the `metric(name, value)` helper in
`crates/cheaper-provider-devin/src/tests/stream.rs` to module scope
and add two contract-pin tests for the devin cache-hit usage shape
from issue #86 — no production-code change.

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/devin-usage-accounting-contract-test-and-docs.md`
- `docs/specs/devin-provider-usage-accounting-contract.md`
- `docs/adr/keep-devin-uncached-input-usage-passthrough-and-document-cache-semantics.md`

## Dependencies

- none

## Affected Files

- `crates/cheaper-provider-devin/src/tests/stream.rs` (only file)

## Done When

- [x] `metric(name, value)` helper at module scope, shared by
      existing and new tests
- [x] `decode_chat_frame_maps_devin_cache_hit_usage_shape` asserts
      `{input_tokens:1, output_tokens:211, cached_input_tokens:3196,
      cache_creation_input_tokens:7, total_tokens:9999}` →
      `ChatUsage{prompt:1, completion:211, total:212,
      cached_input:3196, cache_creation:7}` (wire `total_tokens`
      ignored, `cache_creation` lands in cacheWrite)
- [x] Second test: cache-metrics-only frame yields no
      `ChatEvent::Usage` (pins current latent edge, comment says so)

## Verification

- [x] `cargo test -p cheaper-provider-devin` passes
- [x] `git diff crates/cheaper-provider-devin/src/` touches only
      `src/tests/stream.rs`
