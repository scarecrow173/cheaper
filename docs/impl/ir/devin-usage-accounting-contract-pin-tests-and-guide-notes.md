---
id: "IMPL-034DlQMMNbwZ8DSoKZkGNF"
type: "impl"
status: "completed"
title: "devin usage accounting contract pin tests and guide notes"
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
    - "docs/tasks/pin-devin-cache-hit-usage-mapping-in-stream-tests.md"
    - "docs/tasks/document-devin-usage-semantics-in-providers-and-usage-guides.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/devin-usage-accounting-contract-test-and-docs.md"
    - "docs/designs/devin-usage-accounting-contract-test-and-docs.md"
    - "docs/specs/devin-provider-usage-accounting-contract.md"
    - "docs/adr/keep-devin-uncached-input-usage-passthrough-and-document-cache-semantics.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/devin-usage-accounting-contract-test-and-docs.md"
  derived-by: []
  verifies:
    - "docs/specs/devin-provider-usage-accounting-contract.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# devin usage accounting contract pin tests and guide notes

## Summary

Issue #86 の調査結果（`input: 1, cacheRead: 3196` は Devin の
cache-hit 計上をそのまま写した正しい passthrough）を、contract
pin test と guide 文書で固定した。production code の変更はゼロ。

## Context

- plan: `docs/plans/devin-usage-accounting-contract-test-and-docs.md`
  （approved、`test-spec-skip` 宣言済み — test 自体が deliverable）
- design:
  `docs/designs/devin-usage-accounting-contract-test-and-docs.md`
- spec: `docs/specs/devin-provider-usage-accounting-contract.md`
- ADR:
  `docs/adr/keep-devin-uncached-input-usage-passthrough-and-document-cache-semantics.md`
  （Option 1: passthrough 維持 + test + docs）

## Implementation

### Changes

- `crates/cheaper-provider-devin/src/tests/stream.rs`
  - nested `metric(name, value)` helper を module scope へ
    mechanical hoist（capture なし、新旧テストで共有）。
  - `decode_chat_frame_maps_devin_cache_hit_usage_shape` 追加:
    issue #86 の形状（`input_tokens=1`、`output_tokens=211`、
    `cached_input_tokens=3196`）+ boundary カバレッジ
    （`cache_creation_input_tokens=7` → cacheWrite、
    wire `total_tokens=9999` → 無視され `212` に再計算）。
  - `decode_chat_frame_drops_cache_only_usage_frame` 追加:
    cache-metric のみの frame が usage event を出さない現行挙動を
    pin（spec deferred concern、変更ではなく記録）。
- `docs/guides/providers.md` — Devin Local 節に `usage 計上`
  bullet 追加（uncached `input` / `cacheRead` 分担、
  `totalTokens = input + output` 契約、`input + cacheRead` の
  読み方、all-zero resolved price → `costUsd: 0`）。
- `docs/guides/usage.md` — `meta.usage` 欄に同じ semantic の
  1 項目を追加。

### Decisions During Implementation

- **2 つめの test で latent edge を pin**: briefing review で
  指摘された cache-metrics-only frame drop は変更対象外だが、
  現行挙動を記録する test として追加（comment で deferred
  concern である旨を明記）。
- **costUsd の説明を修正**: 初版は「pricing entry 不在」を
  機序としていたが、review で devin model は常に
  `cost: Some(parse_cost(...))`（all-zero fallback）を持ち
  `resolve_pricing` がゼロを有効値と扱うことが判明。docs の
  表現を「all-zero の価格として解決」に修正し、unknown→free
  conflation を spec の deferred concern として記録した。

## Related Experiments

### Adopted

なし（experiment log は作成していない）。

### Rejected

なし。

## Validation

- `cargo test -p cheaper-provider-devin`: 76/76 pass（新規 2 test
  含む）。
- `git diff crates/cheaper-provider-devin/src/`: 変更は
  `src/tests/stream.rs` のみ（production code 差分ゼロ、spec
  AC「No change to decode_usage…」を満たす）。
- `mise run check` 全 pass（biome/fmt/clippy/version/doc-id/
  cargo test --workspace/node --test/bundle・payload/freshness）。
- review-protocol: briefing で独立 2 reviewer（協調・
  adversarial）の ACCEPT-WITH-FIXES 指摘を全反映、design で
  1 reviewer ACCEPT（P3 のみ・反映済み）。

## Risks

- pin した `totalTokens` 定義は Devin の wire format が変われば
  future-wrong になりうる — spec の Deferred Design Concerns で
  再検討条件を記録済み。

## Follow-ups

- issue #86 へ報告する際、対象 record の `toolCalls` と
  `sourceTokens` を照合する（`toolCalls > 0` かつ
  `sourceTokens: 0` なら別 anomaly として別 issue 化）。
- devin の unknown→free pricing conflation（`parse_cost`
  fallback が snapshot/user pricing を shadow）— nonzero の
  devin pricing が必要になった時点で別 issue 候補。
