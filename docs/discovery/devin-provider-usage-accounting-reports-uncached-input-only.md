---
id: "DISC-034Dlp0gvInn5w4EsnfZB0"
type: "discovery"
status: "draft"
title: "devin provider usage accounting reports uncached input only"
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
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "docs/specs/devin-provider-usage-accounting-contract.md"
    - "docs/adr/keep-devin-uncached-input-usage-passthrough-and-document-cache-semantics.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# devin provider usage accounting reports uncached input only

## Exploration Goal

Issue #86: `devin/swe-2` worker run reports
`meta.usage = {input: 1, output: 211, cacheRead: 3196, cacheWrite: 0,
totalTokens: 212, costUsd: 0}`, while the identical payload on the
previous run reported `input: 3197, output: 223, cacheRead: 0`.

Two questions to resolve:

1. Is the field mapping `input` / `cacheRead` / `cacheWrite` /
   `totalTokens` consistent with what the Devin stream actually emits,
   or does cheaper mis-map fields?
2. Is `totalTokens` excluding `cacheRead` an intentional contract that
   matches other providers, or a bug?

## Key Issues and Assumptions

- **Confirmed — the mapping is a faithful passthrough.** The Devin
  stream emits `metric` records by name; `decode_usage` in
  `crates/cheaper-provider-devin/src/stream.rs:439-447` maps
  `input_tokens` → `prompt_tokens` → `usage.input`,
  `output_tokens` → `usage.output`, any metric containing `cached` or
  `cache_read` → `cached_input_tokens` → `usage.cacheRead`, and
  `cache_creation` → `cacheWrite`. No field swap exists in the Rust
  port.
- **Confirmed — identical to the vendored TS original.** The pre-Rust
  `packages/runtime-pi/src/providers/devin/stream.ts` (vendored from
  `pi-devin`, removed in commit 6e2e49d) contained the same logic:
  `promptTokens = input_tokens`,
  `cachedInputTokens` for `cached`/`cache_read` metrics, and
  `totalTokens = promptTokens + completionTokens`. The Rust port did
  not introduce a regression.
- **Strongly corroborated — Devin reports uncached input only.**
  First run of the same payload: `input: 3197, cacheRead: 0`. Second
  run: `input: 1, cacheRead: 3196`. The numbers only reconcile if
  `input_tokens` is the uncached portion and `cached_input_tokens`
  is the prompt-cache-hit portion (1 + 3196 = 3197 = the earlier
  run's `input`) — the same accounting convention Anthropic uses
  (`input_tokens` excludes `cache_read_input_tokens`). Inferred from
  the observed pair, not verified against Devin docs.
- **Confirmed — `totalTokens = input + output` is intentional.**
  Both the TS original and the Rust port compute it the same way, and
  the accumulator comment records the contract: provider-supplied
  total wins, fallback is `input + output`
  (`crates/cheaper-rig-adapter/src/accumulator.rs:86-93`).
- **Confirmed — `costUsd: 0` is unrelated to the mapping, and its
  mechanism is a resolved all-zero price, not a missing entry.**
  Devin catalog models always carry `cost: Some(parse_cost(...))`
  (`crates/cheaper-provider-devin/src/models.rs`); `parse_cost`
  returns all-zero rates when `cost_summary` is absent/unparseable,
  and `resolve_pricing` treats an all-zero catalog cost as a valid
  "free" observation — so bundled snapshot / user `pricing.json` can
  never override a devin price. `usage_cost_usd`
  (`crates/cheaper-runtime/src/meta.rs`) then computes 0
  arithmetically. Unknown→free conflation is a real gap worth
  recording, but not a usage-mapping defect.
- **`savings.sourceTokens` is a different measurement.** It sums
  successful tool-call output text tokens via the tool meter
  (`crates/cheaper-runtime/src/run.rs:403-407`), disjoint from
  provider usage decoding. `sourceTokens: 0` is legitimate if the
  run made no successful tool calls; if the issue's record shows
  `toolCalls > 0` with `sourceTokens: 0`, that is a separate meter
  anomaly worth a follow-up issue.
- **Latent passthrough hazards in `decode_usage` (inherited from
  the TS original, benign for observed metric names):**
  - substring matching: `contains("cached")` would mis-route a
    hypothetical `uncached_*`/`non_cached*` metric into `cacheRead`;
    cache metrics with other names (e.g. `cache_hit_tokens`) are
    silently dropped.
  - a `total_tokens` metric matches no branch and is overwritten by
    the `input + output` recompute — identical value only while
    Devin's total also excludes cache.
  - same-class metrics overwrite rather than accumulate (e.g. both
    `cached_input_tokens` and `cache_read_input_tokens` → last wins).
  - a usage frame carrying only cache metrics is dropped entirely
    (early `None` when prompt and completion are both absent).

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A: keep mapping, document semantics, add cache-hit regression test | Zero behavior change; matches vendored pi-devin parity and Anthropic convention; cost formula already prices cacheRead separately | `input` no longer means "all prompt tokens" for devin; consumers summing `input` alone undercount | yes |
| B: fold cacheRead into `input` for devin | `input` matches "total prompt tokens" intuition | Double-counts tokens in the cost formula (cacheRead priced separately); breaks parity with vendored TS; distorts providers that report uncached input only | no |
| C: include cacheRead in `totalTokens` | total reflects all processed tokens | Diverges from TS parity and the documented accumulator contract (`input + output` fallback); inconsistent with the field's existing meaning | no |
| D: report a bug upstream / wait for Devin | Would fix at source if Devin's semantics are wrong | Devin's split reporting is a defensible convention, not clearly a bug; no upstream fix needed | no |

## Tentative Conclusions and Hypotheses

- **Confirmed:** no field-mapping bug. The observed
  `input: 1, cacheRead: 3196` is Devin's cache-hit accounting reported
  verbatim; the earlier `input: 3197` run was a cache miss.
- **Confirmed:** `totalTokens` excluding `cacheRead` matches the TS
  original and the accumulator fallback contract, so it is the
  established convention rather than a new defect. It should be
  documented as such rather than changed. Note the repo is not fully
  self-consistent: the test-fixture fallback total is
  `input + output + cacheRead + cacheWrite`
  (`crates/cheaper-runtime/src/fixture.rs`), the opposite convention —
  test-only, but worth recording.
- **Hypothesis:** the durable gaps are documentation and a pinned
  contract — nothing tells a consumer that devin's `input` is the
  uncached portion and `cacheRead` holds the rest (`cheaper usage`
  aggregates fields verbatim, so consumers should read
  `input + cacheRead` as full prompt volume). A regression test
  pinning the cache-hit mapping prevents a future "fix" that
  double-counts.

## Open Questions

- Does Devin ever emit an explicit `total_tokens` metric, and if so
  does it include cache-read tokens? `decode_usage` ignores such a
  metric and recomputes — identical output only while Devin's total
  also excludes cache. Unverifiable from the repo; pinned by the new
  regression test as currently-implemented behavior.
- If the issue's `usage.jsonl` record shows `toolCalls > 0` together
  with `sourceTokens: 0`, a separate meter anomaly exists — to be
  checked when reporting back on the issue.

## Promotion Candidates

- [x] spec-doc needed: pin the devin usage-accounting contract
      (uncached input vs cacheRead, totalTokens definition) so future
      changes don't "fix" it into a double count
- [x] adr-doc needed: record the decision to keep provider-native
      passthrough (vs normalizing `input`/`totalTokens`) — done:
      `docs/adr/keep-devin-uncached-input-usage-passthrough-and-document-cache-semantics.md`

## Sources and Relations

- https://github.com/scarecrow173/cheaper/issues/86
- `crates/cheaper-provider-devin/src/stream.rs:395-455` (decode_usage)
- `crates/cheaper-provider-devin/src/rig_provider.rs:308-320`
  (ChatUsage → Rig Usage)
- `crates/cheaper-rig-adapter/src/accumulator.rs:58-95,156-173`
- `crates/cheaper-runtime/src/meta.rs` (WorkerUsage, usage_cost_usd)
- `crates/cheaper-runtime/src/run.rs:403-412` (sourceTokens meter)
- Pre-migration TS:
  `packages/runtime-pi/src/providers/devin/stream.ts` @ f8b9b3f /
  removed in 6e2e49d — vendored pi-devin `decodeUsage`, identical
  semantics
