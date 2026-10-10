---
id: "SPEC-034DlpZKFnyWiBZZxGsXK0"
type: "spec"
status: "approved"
title: "devin provider usage accounting contract"
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
  implements: []
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
    - "docs/discovery/devin-provider-usage-accounting-reports-uncached-input-only.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# devin provider usage accounting contract

## What Should Be Built

Pin and document the Devin provider's usage-accounting contract: the
Devin stream reports **uncached input tokens** as `input_tokens` and
prompt-cache hits as a separate `cached_*`/`cache_read` metric.
cheaper maps these verbatim into `meta.usage` (`input`, `cacheRead`,
`cacheWrite`) and computes `totalTokens = input + output`
(cache-read tokens excluded). The deliverable is a regression test
that pins this cache-hit mapping end to end at the provider boundary,
plus user-facing documentation of the semantic so consumers do not
misread `input: 1` as a lost prompt.

## Why Now

Issue #86 observed `input: 1, cacheRead: 3196` for a 3k-token prompt
and suspected a mapping bug. Investigation confirmed the mapping is a
faithful passthrough of Devin's Anthropic-style cache accounting
(uncached delta in `input_tokens`, hit portion in
`cached_input_tokens`), identical to the vendored pi-devin TS code the
Rust port replaced. Without a pinned contract, a future maintainer may
"fix" the asymmetry by folding `cacheRead` into `input`, which would
double-count tokens in the cost formula (cache-read is priced
separately) and diverge from the documented accumulator contract.

## Users and Value

- Consumers of `meta.usage` / `cheaper usage` get a documented
  explanation of why devin `input` can be near zero on cache hits.
- Maintainers get a regression test that makes the intended mapping
  explicit and fails loudly if the passthrough drifts.

## Scope

### In Scope

- A regression test in `crates/cheaper-provider-devin` pinning the
  cache-hit metric mapping at the `decode_chat_frame` →
  `ChatEvent::Usage(ChatUsage)` seam (the hop to Rig `Usage` is a
  verbatim field copy, covered by inspection): uncached
  `input_tokens`, `cached_input_tokens` → `cached_input_tokens`,
  `cache_creation` → `cache_creation_input_tokens`, and
  `total_tokens = input + output`.
- Documentation of the devin usage semantics in
  `docs/guides/providers.md` (devin section) and
  `docs/guides/usage.md` (where `meta.usage` fields are defined):
  `input` is the uncached portion, `cacheRead` carries the cache-hit
  portion, full prompt volume is `input + cacheRead`, `totalTokens`
  excludes `cacheRead` by contract, and `costUsd` prices `cacheRead`
  at its own rate when a nonzero price resolves.

### Out of Scope

- Changing the mapping (folding cacheRead into input, or including
  cacheRead in totalTokens) — explicitly rejected, see ADR.
- `savings.sourceTokens` behavior — it measures successful tool-call
  output tokens, disjoint from provider prompt accounting. (If the
  issue record shows `toolCalls > 0` with `sourceTokens: 0`, that is
  a separate anomaly to file, not part of this spec.)
- Pricing resolution for devin models — `costUsd: 0` comes from an
  all-zero resolved price (`parse_cost` fallback when `cost_summary`
  is absent), which also shadows bundled/user pricing for devin.
  That unknown→free conflation is a real gap but not a usage-mapping
  defect; tracked as a deferred concern.
- Other providers' usage conventions.

## Requirements

- R1: The Devin stream metric `input_tokens` continues to map to
  `usage.input` unchanged (it is the uncached portion on cache hits).
- R2: Metrics matching `cached`/`cache_read` continue to map to
  `usage.cacheRead`; `cache_creation` continues to map to
  `usage.cacheWrite`. The contract is substring matching exactly as
  implemented (`contains`), with the documented hazard that a
  hypothetical `uncached_*` metric would mis-route — benign for all
  observed Devin metric names.
- R3: `usage.totalTokens` remains `input + output` and does not add
  `cacheRead`, matching the vendored TS original and the accumulator
  fallback contract.
- R4: A regression test reproduces the issue's cache-hit shape
  (e.g. `input_tokens=1`, `cached_input_tokens=3196`,
  `output_tokens=211`), asserts the mapped field placement and
  `total_tokens` value, covers `cache_creation` → `cacheWrite`, pins
  that a wire `total_tokens` metric is ignored in favor of recompute,
  and pins that a `cache_creation`-named metric does not land in
  `cacheRead` — so a future double-count or mis-route is caught.
- R5: The providers guide and the usage guide document the semantic
  so `input: 1` on a cache hit is not read as data loss, and tell
  consumers to use `input + cacheRead` for full prompt volume.

## Acceptance Criteria

- [ ] New test asserting the issue-#86 metric shape maps to
      `{input: 1, output: 211, cacheRead: 3196, totalTokens: 212}`,
      plus the R4 coverage points (`cache_creation` → `cacheWrite`,
      wire `total_tokens` ignored, `cache_creation` not in
      `cacheRead`), passes in `cargo test -p cheaper-provider-devin`.
- [ ] `docs/guides/providers.md` and `docs/guides/usage.md` document
      devin's uncached-input / cacheRead split, the `totalTokens`
      definition, and the `input + cacheRead` reading guidance.
- [ ] No change to `decode_usage` field mapping, the accumulator, or
      `usage_cost_usd` (behavior preserved).
- [ ] `mise run check` passes.

## Deferred Design Concerns

- Whether Devin emits an explicit `total_tokens` metric, and whether
  its definition includes cache-read tokens — `decode_usage` ignores
  it and recomputes; revisit only if Devin's wire format changes.
- Latent `decode_usage` passthrough hazards (all inherited from the
  TS original, benign for observed metric names): substring matching
  (`uncached_*` would mis-route, unknown cache-metric names are
  silently dropped), same-class metrics overwrite rather than
  accumulate, and a cache-metrics-only usage frame is dropped.
  Hardening these is a behavior change, out of scope; recorded for
  visibility.
- Devin unknown→free pricing conflation: `parse_cost` maps absent
  `cost_summary` to all-zero rates, which `resolve_pricing` treats
  as a valid observation — bundled/user pricing can never supply a
  devin price. Separate concern from usage mapping; candidate for
  its own issue if nonzero devin pricing becomes needed.
- Test-fixture convention divergence: `FixtureUsage::to_snapshot`
  defaults total to `input + output + cacheRead + cacheWrite`
  (`crates/cheaper-runtime/src/fixture.rs`), opposite of the
  production contract — test-only; revisit if a fixture ever needs
  to model devin cache semantics.
- Provider-agnostic normalization of `input` (e.g. reporting a
  "prompt tokens total" alongside provider-native fields) — deferred;
  would need a cross-provider contract change, out of scope here.

## Sources and Relations

- https://github.com/scarecrow173/cheaper/issues/86
- discovery:
  `docs/discovery/devin-provider-usage-accounting-reports-uncached-input-only.md`
- ADR:
  `docs/adr/keep-devin-uncached-input-usage-passthrough-and-document-cache-semantics.md`
- `crates/cheaper-provider-devin/src/stream.rs:395-455`
- `crates/cheaper-rig-adapter/src/accumulator.rs:86-92`
- `crates/cheaper-runtime/src/meta.rs` (`usage_cost_usd`)
- Vendored TS original (removed commit 6e2e49d):
  `packages/runtime-pi/src/providers/devin/stream.ts`
