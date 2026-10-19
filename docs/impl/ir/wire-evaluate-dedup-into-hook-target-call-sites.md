---
id: "IMPL-034IqcrVJL4pxtau4WVtRV"
type: "impl"
status: "completed"
title: "Wire evaluate_dedup into hook target call sites"
created: "2026-10-19"
updated: "2026-10-19"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/hook/targets.rs"
      - type: "file"
        path: "docs/tasks/wire-evaluate-dedup-into-hook-target-call-sites.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/wire-evaluate-dedup-into-hook-target-call-sites.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/hook-eval-dedup-verdict-cache-and-leader-election.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/hook-eval-dedup-verdict-cache-contract.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Wire evaluate_dedup into hook target call sites

## Summary

issue #127(重複 hook firing)対応の plan T3 を実装した。
`targets.rs` の `handle_hook` 5 arm(windsurf / copilot / pretooluse /
devin / cursor)で直接呼ばれていた `routing_policy::evaluate` を
`eval_cache::evaluate_dedup` へ差し替え、canonical request
fingerprint ベースの verdict cache + leader election 経路を全 hook
target で有効化した。戻り値は同じ `Verdict` で、normalize・render・
`fallback_allow`・完了 `debug!` event は無変更(diff は call site と
import のみ)。

## Context

- task: `docs/tasks/wire-evaluate-dedup-into-hook-target-call-sites.md`
  (TASK-034IpPwmpKldYJ5rPUuq4z)
- plan: `docs/plans/implement-hook-eval-dedup-verdict-cache.md`
  (T3 — targets.rs 配線)
- design: `docs/designs/hook-eval-dedup-verdict-cache-and-leader-election.md`
  (DESIGN-034IefWW6OJ9rvESaNvKUK)
- 前提: T1+T2(`eval_cache::evaluate_dedup` と fingerprint helpers)は
  856eec1 で commit 済み。

## Implementation

### Changes

- `crates/cheaper-cli/src/hook/targets.rs`
  - `use super::eval_cache;` を追加(既存 `use super::routing_policy::
    {...};` の直上、同 module の既存 import 形式に合わせた)。
  - `handle_hook` 5 arm の
    `evaluate(&request, &deps.eval_opts()).await` を
    `eval_cache::evaluate_dedup(&request, &deps.eval_opts()).await` へ
    差し替え — windsurf(:639)・copilot(:647)・pretooluse(:654)・
    devin(:661)・cursor(:672)。
  - `use super::routing_policy::{...}` の import list から
    `evaluate` を除去(全 call site 差し替え後は未使用)。
- `docs/tasks/wire-evaluate-dedup-into-hook-target-call-sites.md` —
  status `done`、checklist 全項目 checked。

### Decisions During Implementation

- `evaluate_dedup` の実際の signature は `(request, opts)` の 2 引数で、
  `FingerprintContext` は内部で `fingerprint_targets(request)` から構築
  される(task 文の指示どおり module の API をそのまま使い、ctx を
  呼出し側で組み立てる形にはしなかった)。
- diff は call site と import のみに限定 — verdict render・
  `verdict_name`・完了 event・catch-all は task 契約どおり一切触らない。

## Related Experiments

### Adopted

なし(既知の実装 task のため Experiment Log は作成していない)。

### Rejected

なし。

## Validation

- `mise exec -- cargo check -p cheaper-cli --features
  cheaper-cli/dev-seam` — exit 0。
- `mise exec -- cargo clippy -p cheaper-cli --features
  cheaper-cli/dev-seam --all-targets -- -D warnings` — exit 0
  (unused_imports・警告無し)。
- `mise exec -- cargo test -p cheaper-cli --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration
  hook::` — 24 passed / 0 failed(既存 5 target 契約に回帰無し)。
- `grep "evaluate(" crates/cheaper-cli/src/hook/targets.rs` — 0 件
  (全 call site が `evaluate_dedup` 経由)。

## Risks

- dedup 経路固有の残リスク(fingerprint 取りこぼし・lock stale 誤判定
  等)は T1+T2 の IR に記録済み。本 task の変更は呼出し経路の差し替え
  のみで新たなリスクは追加しない — `evaluate_dedup` は全失敗を
  `evaluate` 直接呼出しへ畳む fail-open 設計のため、cache 層の障害でも
  host 観測挙動は従来どおり。

## Follow-ups

- T4: `tests/integration/hook_eval_dedup.rs` + `main.rs` mod 宣言
  (dedup 協調の integration 契約 — module seam `evaluate_dedup_with` /
  `DedupLimits` と binary dual-spawn test)。
