---
id: "IMPL-034JjbLxN5vUbJ45QnDJdH"
type: "impl"
status: "completed"
title: "Add route and route-refetch usage records"
created: "2026-10-20"
updated: "2026-10-20"
owners: []
relations:
  source:
    - "docs/tasks/add-route-usage-records.md"
    - "docs/test-specs/route-usage-records.md"
    - "docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/usage_log.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/usage_report.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/usage_log.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/usage_report.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/regression/expected/cli-usage-empty.json"
      - type: "file"
        path: "crates/cheaper-cli/tests/regression/expected/cli-usage-seeded.json"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/add-route-usage-records.md"
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
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Add route and route-refetch usage records

## Summary

`usage_log.rs` に `kind:"route"`/`kind:"route-refetch"` record
builder + best-effort append helper を追加し、`usage_report.rs`
の `is_record` gate 拡張と独立 `route` bucket 集計を実装した。
env 値・credential を含まない構造保証(record には `commandHash`
hex のみ)で、既存 run/count 集計は不変。

## Context

task `add-route-usage-records` (B4)。approved design
`DESIGN-034JbqoCWm85pKC033fgfd` §usage record(R34)に従う。
record append の呼出しは `route run`(D1)・`route logs`(D4)
側が担う — 本 task は builder と集計の seam を提供する。

## Implementation

### Changes

- `usage_log.rs` — `RouteRecordInput` + `build_route_record`
  (R34 field 群: route/commandHash/originalTokens/returnedTokens/
  reductionRatio/worker/durationMs/logRef/refetches/
  postprocessFailures/exitCode/ok)、`build_route_refetch_record`
  (`logRef`/`bytesRead`)、`record_route`/`record_route_refetch`
  (record_usage と同一の best-effort append 規約)。
- `usage_report.rs` — `is_record` gate を `"route"`/
  `"route-refetch"` へ拡張し、`route_bucket` 独立集計
  (executions/failures/refetches/refetchBytes/originalTokens/
  returnedTokens/postprocessFailures)を report へ `route` key
  として追加。run filter・`aggregate`・`directlyComparable` 母数
  とは分離(D9)。

### Decisions During Implementation

- `commandHash` は `embedded_fingerprint` と同じ
  `DefaultHasher` hex 規約(同一 toolchain 内のみ安定 — C14f)。
- route bucket は run filter(`--since`/`--worker`)非適用の全
  record 集計 — route 系は別次元の計測で、run 絞り込みに
  連動させる spec 根拠が無いため。
- golden `cli-usage-*` 2 file は `route` 0 値 bucket を追記
  (normalize が durationMs を除去する一方 route は厳密比較
  対象)。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `cargo test -p cheaper-cli --test integration usage_` — 47
  pass(新規: route record shape・commandHash deterministic・
  env 値非含有・refetch shape・独立 bucket join・run 集計非混入)。
- `cargo test -p cheaper-cli --test regression`(dev-seam)—
  golden 全 case pass(cli-usage 2 file 更新込み)。
- `cargo clippy --workspace --all-targets` — warning 無し。
- `cargo fmt --all` — 適用済み。

## Risks

- `DefaultHasher` は toolchain upgrade で hash がドリフト
  し得る — record join は同一 toolchain 内のみ保証
  (embedded_fingerprint と同じ運用制約、C14f)。
- `record_route`/`record_route_refetch` の caller は D1/D4 —
  本 task 時点では builder+集計のみ検証済み。

## Follow-ups

- D1 `route run` が `record_route` を呼ぶ。
- D4 `route logs` が `record_route_refetch` を呼ぶ。
