---
id: "IMPL-034Mp9JnWzYzVnWA3PkOnh"
type: "impl"
status: "completed"
title: "Implement usage route window and byRoute aggregation issue 184 T2"
created: "2026-10-26"
updated: "2026-10-26"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/usage_report.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/usage_report.rs"
      - type: "file"
        path: "docs/tasks/implement-usage-route-window-and-by-route.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/implement-usage-route-window-and-by-route.md"
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
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Implement usage route window and byRoute aggregation issue 184 T2

## Summary

`route_bucket` を since-aware route window と `build_route_report` に置換し、
window-wide logRef join/orphan、route scoping、`byRoute` 行を実装した。
レビューゲート後に T2 を完了化する。

## Context

承認済み #184 plan/design/spec の T2 slice。T1 compile seam と pins は
`4465f95` に完了済み。#192 の ID は opaque metadata のまま扱い、
join は必ず Usage record の `logRef` を使う。

## Implementation

### Changes

- `route_window` は route/refetch kind のみを `--since` で選別し、
  since がない場合は欠損・不正 timestamp を含めて保持する。
- `build_route_report` は window 全体で first-wins owner map、joined/orphan
  refetch、unkeyed count を計算する。route filter は route totals/rows と
  scoped refetch だけを絞り、run aggregate と window-wide diagnostics を
  混同しない。
- `BTreeMap` による route 名順と固定 field order を保ち、token/duration
  measurability、ratio、空 `byRoute` を出力する。group selector 無指定時は
  additive diagnostic keys と `byRoute` を出さない。
- `build_usage_report` の順序を route window/report → run filter/aggregate
  に変更し、module overview を更新した。

### Decisions During Implementation

- T2 route pins が発見した実装ミスを同一 slice 内で修正:
  (1) filter 指定で owner がない route の refetch は全 window refetch へ
  fallback せず 0 件、(2) `unkeyedRecords` は scoped rows と独立して window
  全体で計数する。
- T1 review の route-filter 強化候補は P2 non-blocking。ここでは既承認の
  T1 pin contract を変更せず、T2 scoped aggregation test 群で既定契約を検証。

## Related Experiments

### Adopted

なし。承認済み設計を実装し、探索的な代替案比較は行っていない。

### Rejected

なし。

## Validation

- `cargo fmt --all -- --check`: exit 0。
- `cargo clippy -p cheaper-cli --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam -- -D warnings`: exit 0。
- T2 focused route pins 15件を個別実行: 全て 1 passed / 0 failed。
- `cargo test ... --test integration usage_report::`: 51 passed / 14 failed。
  残る14件は T3/T4 所有の expected-red（T3 9、T4 module 内5）。
- `cargo test ... --test regression -- --nocapture`: regression golden 61/61 pass。
  `git diff --exit-code -- crates/cheaper-cli/tests/regression`: exit 0。
- 独立 cooperative/adversarial review はともに NO CANDIDATES。T2 を受け入れ。

## Risks

- T3 `byError` aggregation と T4 CLI/text/guide が未実装のため、
  `usage_report::` 全体と `mise run check` はこの段階で green ではない。

## Follow-ups

- T2 はレビュー・検証後に完了化済み。次の graph-runnable task T3 は
  `byError` aggregation。
