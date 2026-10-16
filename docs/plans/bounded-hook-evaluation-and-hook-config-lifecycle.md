---
id: "PLAN-034H3ODf3yWW5G5NgvUvt8"
type: "plan"
status: "completed"
title: "Bounded hook evaluation and hook-config registration lifecycle"
created: "2026-10-16"
updated: "2026-10-17"
owners:
  - scarecrow173
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/107"
    - "https://github.com/scarecrow173/cheaper/pull/109#issuecomment-6221192905"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/bounded-native-hook-evaluation-and-managed-devin-registration.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/implement-native-tool-normalization-and-devin-bootstrap.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/bounded-native-hook-evaluation-and-managed-devin-registration.md"
    - "docs/designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/adr/range-aware-file-read-and-bounded-search-requests-in-hook-routing.md"
    - "docs/adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md"
  defers: []
  deferred-by: []
---

# Bounded hook evaluation and hook-config registration lifecycle Implementation Plan

## Goal

`RoutingRequest` に範囲 read・Search を導入して issue #107 の根因
（bounds 消失 → 全文計測 deny）を解消し、Devin `exec` routing と
CLI 所有の `hook-config sync|verify|remove`（APM lifecycle 統合）を
実装する。

## Scope

1つの実装 stream。`routing_policy`・`targets`・`hook-config` module・
PS wrapper・docs 追随を含む。APM 本体・Devin 側の変更は範囲外。

## Upstream Documents

- docs/specs/bounded-native-hook-evaluation-and-managed-devin-registration.md
- docs/adr/range-aware-file-read-and-bounded-search-requests-in-hook-routing.md
- docs/adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md

## Design Inputs

- docs/designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md

## File Map

- `crates/cheaper-cli/src/hook/routing_policy.rs` — `ReadRange`・
  `Search` variant・`evaluate_ranged_read`・`evaluate_search`・
  scan cap 定数。
- `crates/cheaper-cli/src/hook/targets.rs` — bounds 正規化・`exec`
  routing・cwd anchor・matcher `read|grep|exec`・capability line。
- `crates/cheaper-cli/src/hook/hook_config.rs`（新規）— sync/verify/
  remove・atomic replace・legacy scan・`--event-stdin` 写像。
- `crates/cheaper-cli/src/args.rs`・`cli.rs` — `hook-config` command
  dispatch・option parsing。
- `packages/apm/.apm/skills/cheaper-usage/scripts/install-devin-hooks.ps1`
  — CLI 委譲の thin wrapper 化。
- `docs/guides/distribution.md`・`packages/apm/.apm/skills/cheaper-*/SKILL.md`
  — lifecycle trust・per-dep `targets:` 移行・verify 手順の追随。
- `crates/cheaper-cli/tests/integration/hook.rs`・`scripts/test/*.mjs`
  — Rust integration + Node e2e。

## Implementation Sequence

- [ ] 1. `RoutingRequest` に `range`/`Search` を追加し evaluate 分岐
- [ ] 2. `evaluate_ranged_read`（skip-inclusive byte cap・`take()`）
- [ ] 3. `evaluate_search`（scan cap・materialize・token 比較）
- [ ] 4. `tool_use_normalize` の bounds/`-i`/cwd anchor 拡張 + `exec`
- [ ] 5. `hook-config` module + CLI dispatch + `--event-stdin`
- [ ] 6. PS wrapper 化 + capability `hookConfig` field
- [ ] 7. docs 追随（distribution・skills・先行 docs）
- [ ] 8. Rust/Node test + POSIX container 検証

## Dependencies and Risks

- `count_text_tokens`・`count_max_bytes`・`nodepath`・regex・serde_json
  （preserve_order）は既存依存。
- Search 実体化は tool 実出力の近似 — over-scan/over-count 方向に倒す。
- lifecycle script は consumer trust 前提・fail-soft（強制機構ではない）。
- 旧 deployed script と新 matcher の組み合わせは version pairing note
  で案内（旧 script 側の matcher equality check が Fail になる
  既知の残余リスク）。

## Task Breakdown

- `bounded-search-and-ranged-read-evaluation` — sequence 1–4。
- `cli-owned-hook-config-lifecycle` — sequence 5–6。
- `bounded-eval-and-registration-docs-e2e` — sequence 7–8。

## Verification

- [ ] `cargo test -p cheaper-cli` の hook integration test が pass
  （ranged read・search・exec・hook-config・event mapping）。
- [ ] `node --test scripts/test/devin-hooks-install.test.mjs` 等が
  pass（wrapper 化後も contract 維持）。
- [ ] POSIX container で `cargo test` + `node --test` が pass。
- [ ] `mise run check` が pass。
- [ ] `node scripts/check-doc-ids.mjs` + doc audit で新規警告なし。

## Open Questions / Gaps

- Devin `read` の `offset` 基数は wire 観測ベースの 1-based 前提。
- `.devin/hooks.v1.json` と `config.json` の precedence は公表未確認
  （verify warn で検出のみ）。
- `.apm/hooks.json` file-based lifecycle（APM PR #1798）が package
  側から使えるかは未検証 — consumer `lifecycle:` を主経路とする。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（subagent 不要の判断で直列実行）。
- Independent work streams: none（routing と hook-config は test で
  交差するため同一 stream）。
- User approval required before dispatch: no（review-protocol で承認済み）。
- If declined or unavailable: execute inline using the same dependency
  graph and verification matrix.

## Review Handoff

- Reviewer A/B（independent）+ cycle-2 adversarial 検証で
  確認済み blocker なし。focus: `hook_config` の schema/atomic write
  contract と `evaluate_search` の cap 境界。
