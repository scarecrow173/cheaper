---
id: "TASK-034IpPwmpKldYJ5rPUuq4z"
type: "task"
status: "todo"
title: "Wire evaluate_dedup into hook target call sites"
created: "2026-10-19"
updated: "2026-10-19"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/127"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/hook/targets.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-hook-eval-dedup-verdict-cache.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-hook-eval-dedup-verdict-cache.md"
    - "docs/tasks/implement-dedup-helpers-and-eval-cache-module.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/hook-eval-dedup-verdict-cache-contract.md"
  references:
    - "docs/designs/hook-eval-dedup-verdict-cache-and-leader-election.md"
  defers: []
  deferred-by: []
---

# Wire evaluate_dedup into hook target call sites

## Work

plan の T3: `targets.rs` の `handle_hook` 5 arm で `evaluate` 呼出しを
`eval_cache::evaluate_dedup` へ差し替え、dedup 経路を全 hook target で
有効化する。戻り値は同じ `Verdict` で、normalize・render・
`fallback_allow`・`execute_hook` catch-all・完了 `debug!` event は
無変更。

- [ ] `targets.rs` へ `eval_cache` の import を追加(`use
      super::eval_cache;` 等、同 module 内の既存 import 形式に合わせる)
- [ ] 5 arm の `evaluate(&request, &deps.eval_opts()).await` を
      `eval_cache::evaluate_dedup(&request, &deps.eval_opts()).await` へ
      差し替え(`:638` windsurf / `:646` copilot / `:653` pretooluse /
      `:660` devin / `:671` cursor)。戻り値は同じ `Verdict` で
      `verdict_name`・各 `*_render`・完了 `debug!` event は無変更
- [ ] `use super::routing_policy::{...}`(`targets.rs:16`)の import
      list から `evaluate` を除去 — 5 call site 差し替え後は未使用で
      clippy `-D warnings`(unused_imports)に抵触する
- [ ] 既存 5 target 契約に回帰が無いことを確認(Verification 節)

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-hook-eval-dedup-verdict-cache.md (T3 節)
- docs/designs/hook-eval-dedup-verdict-cache-and-leader-election.md

## Dependencies

- `docs/tasks/implement-dedup-helpers-and-eval-cache-module.md` —
  `eval_cache::evaluate_dedup` が無ければ compile しない(plan の直列
  chain: T1+T2 → T3)。

## Affected Files

- modify: `crates/cheaper-cli/src/hook/targets.rs`(5 call site 差し替え
  + import 整理のみ)

## Done When

- [ ] 5 arm 全てが `eval_cache::evaluate_dedup` を呼び、`evaluate` の
      import が残っていない
- [ ] normalize・render・`fallback_allow`・完了 event が無変更(diff が
      call site と import のみ)
- [ ] 既存 `hook::` integration test が全件 pass(5 target 契約不変)

## Verification

- [ ] `mise exec -- cargo clippy -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` が exit 0(unused_imports 無し)
- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook::` が exit 0
- [ ] `grep -n "evaluate(&request" crates/cheaper-cli/src/hook/targets.rs` が 0 件(全 call site が `evaluate_dedup` 経由)
