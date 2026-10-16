---
id: "TASK-034H3PzQISOeM552EcP64F"
type: "task"
status: "done"
title: "Bounded search and ranged read evaluation"
created: "2026-10-16"
updated: "2026-10-17"
owners:
  - scarecrow173
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/107"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/bounded-hook-evaluation-and-hook-config-lifecycle.md"
  implemented-by: []
  depends-on:
    - "docs/plans/bounded-hook-evaluation-and-hook-config-lifecycle.md"
  blocks:
    - "docs/tasks/cli-owned-hook-config-lifecycle.md"
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
    - "docs/designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md"
  defers: []
  deferred-by: []
---

# Bounded search and ranged read evaluation

## Work

`RoutingRequest` に `ReadRange`・`Search` を導入し、
`evaluate_ranged_read`・`evaluate_search`・`exec`/bounds 正規化・
cwd anchor（payload `cwd` → `DEVIN_PROJECT_DIR` → process cwd）を
実装する。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/bounded-hook-evaluation-and-hook-config-lifecycle.md
- docs/designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md
- docs/adr/range-aware-file-read-and-bounded-search-requests-in-hook-routing.md

## Dependencies

- なし（最初の task）。

## Affected Files

- `crates/cheaper-cli/src/hook/routing_policy.rs`
- `crates/cheaper-cli/src/hook/targets.rs`
- `crates/cheaper-cli/tests/integration/hook.rs`

## Done When

- [ ] `read{offset,limit}` が window のみ評価し、巨大 file の小窓が
  allow・巨大窓が deny・無効値は全文縮退。
- [ ] `Search` が file/dir/省略 path・`max_results`・context・glob・
  output_mode・`-i` を反映し、scan cap 到達で deny。
- [ ] devin `exec` が Shell へ route され、matcher が
  `read|grep|exec` に拡張される。
- [ ] windsurf `tool_info.offset/limit`・copilot `view_range` も
  bounds として受理される。

## Verification

- [ ] `cargo test -p cheaper-cli hook` が pass。
- [ ] `mise exec -- cargo clippy -p cheaper-cli --all-targets -- -D warnings`
  が pass。
- [ ] `cargo fmt --check` が pass。
