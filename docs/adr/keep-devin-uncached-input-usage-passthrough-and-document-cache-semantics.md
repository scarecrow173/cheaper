---
id: "ADR-034Dlq0a0IpzuDPMae9zdn"
type: "adr"
status: "accepted"
title: "Keep Devin uncached-input usage passthrough and document cache semantics"
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
    - "docs/specs/devin-provider-usage-accounting-contract.md"
    - "docs/designs/devin-usage-accounting-contract-test-and-docs.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/devin-provider-usage-accounting-reports-uncached-input-only.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# Keep Devin uncached-input usage passthrough and document cache semantics

## Context and Problem Statement

Issue #86 reported `meta.usage = {input: 1, output: 211,
cacheRead: 3196, totalTokens: 212, costUsd: 0}` for a devin/swe-2
worker run and suspected a mapping bug, since the same payload had
previously reported `input: 3197, cacheRead: 0`.

Investigation showed the Devin stream emits `input_tokens` as the
**uncached** portion of the prompt and reports the prompt-cache-hit
portion separately as `cached_input_tokens`. cheaper maps the metrics
verbatim (`stream.rs:439-447`), exactly as the vendored pi-devin TS
implementation it replaced did. The question is whether to keep this
provider-native passthrough (and document it) or normalize the fields
so `input` looks like "all prompt tokens".

## Decision Drivers

* Parity with the vendored pi-devin TS implementation — the Rust port
  deliberately preserves its semantics.
* `usage_cost_usd` prices input, output, cacheRead and cacheWrite at
  separate rates; folding cacheRead into `input` would double-count
  those tokens whenever pricing data exists.
* Anthropic's API reports prompt-cache usage the same way
  (`input_tokens` excludes `cache_read_input_tokens`), so Devin's
  convention is not anomalous.
* Consumers summing `input` alone undercount prompt tokens on cache
  hits; that confusion must be resolved by documentation, not by
  distorting the data.
* `costUsd: 0` in the issue is a resolved all-zero price
  (`parse_cost` maps absent `cost_summary` to zeros and
  `resolve_pricing` treats zero as valid "free"), which also
  shadows bundled/user pricing for devin — a real gap, but separate
  from the usage-mapping question.

## Considered Options

* Option 1: keep the passthrough, pin it with a regression test, and
  document the semantic in the providers guide.
* Option 2: fold `cacheRead` into `input` for the devin provider.
* Option 3: keep fields as-is but include `cacheRead` in
  `totalTokens`.

## Decision Outcome

Chosen option: "Option 1", because the mapping faithfully reflects
what Devin reports, matches the vendored TS original, and keeps the
cost formula free of double counting; the only real gap was missing
documentation and a test pinning the contract.

### Consequences

* Good, because `meta.usage` remains a faithful passthrough of
  provider-native accounting, consistent with Anthropic-style cache
  reporting and the vendored implementation.
* Good, because a regression test now pins the cache-hit shape so the
  contract cannot silently drift.
* Bad, because `input` for devin means "uncached input" while for
  providers without cache metrics it is the whole prompt — the
  asymmetry is documented rather than normalized away.

### Confirmation

* New regression test in `crates/cheaper-provider-devin` asserting the
  issue-#86 metric shape (`input_tokens=1`,
  `cached_input_tokens=3196`, `output_tokens=211`) maps to
  `{input: 1, output: 211, cacheRead: 3196, totalTokens: 212}`.
* `docs/guides/providers.md` devin section explains the semantic.

## Implementation Plan

* Affected paths: `crates/cheaper-provider-devin/src/tests/stream.rs`
  (new test; no production-code change), `docs/guides/providers.md`,
  `docs/guides/usage.md`.
* Patterns to follow: existing `decode_chat_frame_reads_usage_metrics`
  test; existing guide prose style.
* Patterns to avoid: changing `decode_usage` field mapping, the
  accumulator, or `usage_cost_usd` — behavior is preserved.
* Dependencies or configuration: none.

## Verification

* [ ] `cargo test -p cheaper-provider-devin` passes with the new
      cache-hit regression test.
* [ ] `mise run check` passes.
* [ ] Providers and usage guides document the uncached-input /
      cacheRead split, the `totalTokens = input + output` definition,
      and the `input + cacheRead` full-prompt-volume reading.

## Pros and Cons of the Options

### Option 1 — passthrough + docs + test

* Good, because no behavior change, full parity with the vendored TS
  original, and the cost formula stays correct.
* Bad, because `input` remains provider-semantic rather than
  normalized; consumers must read the guide.

### Option 2 — fold cacheRead into input

* Good, because `input` would match the "total prompt tokens"
  intuition.
* Bad, because it double-counts tokens against the cache-read price
  rate, breaks parity with the vendored TS implementation, and lies
  about what Devin actually billed.

### Option 3 — include cacheRead in totalTokens

* Good, because `totalTokens` would count every processed token.
* Bad, because consumers computing `input + cacheRead` per the
  documented semantic would double-count against a cache-inclusive
  `totalTokens`; it also diverges from the TS original and the
  accumulator's documented `input + output` fallback contract.

## More Information

* https://github.com/scarecrow173/cheaper/issues/86
* `crates/cheaper-provider-devin/src/stream.rs:395-455`
* `crates/cheaper-rig-adapter/src/accumulator.rs:86-93`
* `crates/cheaper-runtime/src/meta.rs` (`usage_cost_usd`)
* Known latent hazards recorded in the spec's Deferred Design
  Concerns: substring metric matching, same-class metric overwrite,
  cache-only frames dropped, wire `total_tokens` ignored, devin
  unknown→free pricing conflation, and the test-fixture total
  fallback (`input + output + cacheRead + cacheWrite` in
  `crates/cheaper-runtime/src/fixture.rs`) using the opposite
  convention.
* Vendored TS original: `packages/runtime-pi/src/providers/devin/
  stream.ts` @ f8b9b3f (removed in 6e2e49d)
