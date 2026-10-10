---
id: "PLAN-034D8SPOCUdlPoQHojlMCb"
type: "plan"
status: "approved"
title: "Volume-root workspace boundary predicate fix"
created: "2026-10-09"
updated: "2026-10-09"
owners: []
relations:
  source:
    - "../designs/trailing-separator-guard-for-workspace-boundary-predicate.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../designs/trailing-separator-guard-for-workspace-boundary-predicate.md"
    - "../specs/volume-root-workspace-boundary-acceptance.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/volume-root-workspace-boundary-acceptance.md"
    - "../adr/guard-trailing-separator-in-workspace-boundary-predicate.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/trailing-separator-guard-for-workspace-boundary-predicate.md"
  derived-by: []
  verifies:
    - "../specs/volume-root-workspace-boundary-acceptance.md"
  verified-by:
    - "../test-specs/volume-root-workspace-boundary-predicate-behavior.md"
  references: []
  defers: []
  deferred-by: []
---

# Volume-root workspace boundary predicate fix Implementation Plan

## Goal

`is_inside_workspace_root` に trailing-separator guard を追加し、
volume-root workspace（`C:\`、`/`）で boundary predicate が正しく
動作することを regression test とともに実装する（issue #77）。

## Scope

単一の implementation stream。変更は
`crates/cheaper-core/src/workspace/paths.rs` の 1 関数 + 同 file 内
test のみで、分割不要。

## Upstream Documents

- `docs/specs/volume-root-workspace-boundary-acceptance.md`
- `docs/adr/guard-trailing-separator-in-workspace-boundary-predicate.md`
- `docs/discovery/volume-root-workspace-boundary-check-double-separator.md`

## Design Inputs

- `docs/designs/trailing-separator-guard-for-workspace-boundary-predicate.md`
  （approved）

## File Map

- `crates/cheaper-core/src/workspace/paths.rs` —
  `is_inside_workspace_root`（:103-110）の prefix 構築に
  `ends_with(nodepath::SEP)` guard を追加（`resource_contains`
  predicates.rs:36-40 と同一 pattern）。test mod に volume-root
  predicate test（`is_inside_workspace_root_volume_root`）と
  `resolve_workspace_path` の volume-root test
  （`resolve_workspace_path_at_volume_root`）を追加（cfg gate を
  test fn 単位にするため既存 case fn 拡張ではなく新規 fn）。

## Implementation Sequence

- [ ] 1. `is_inside_workspace_root` の prefix 構築を
      `let prefix = if r.ends_with(nodepath::SEP) { r.clone() } else { format!("{r}{}", nodepath::SEP) };`
      に修正（`resource_contains` と同一形。`r` は String で
      `String + char` は compile しない。normalized 比較用文字列 `r`
      側から構築し、`c == r` で後続使用があるため clone する）
- [ ] 2. win32/posix の volume-root unit test を `cfg!` gate で追加
- [ ] 3. `resolve_workspace_path` の volume-root test を追加。root は
      platform の volume root: posix は `canonicalize_workspace_root("/")`
      （canonicalize が trailing-sep 形状を実際に生成する seam も
      pin する）、win32 は raw `"C:\\"` literal（drive 実在を要求
      しない）。tail は nonexistent 一意名
      （`cheaper-volroot-probe-<pid>`）で、期待 `real`/`display` も
      assert する
- [ ] 4. `mise exec -- cargo test -p cheaper-core` → `mise run check`

## Dependencies and Risks

- `nodepath::SEP` は build 時確定（win32 `\` / posix `/`）なので
  volume-root test は platform gate が必須。他 platform shape の test
  は当該 build で fail するため無条件に書かない。
- 実 FS test は root 直下の一意な nonexistent 名
  （`cheaper-volroot-probe-<pid>`）を使い、実在衝突と platform
  symlink（macOS `/tmp` 等）を回避する。
- prefix は raw `root` ではなく normalized `r` から構築する
  （`case_insensitive` で `r` は lowercased。raw から構築すると
  win32 AC が fail する）。
- CI（`.github/workflows/ci.yml`）は `ubuntu-latest` のみで、
  `cfg!(windows)` gated assertion は CI では実行されない。win32 AC
  の検証は手動 Windows `mise run check` に依存する点を明記する
  （本 repo の dev 環境は Windows のため実施可能だが、posix 側
  regression が CI で検出される非対称は残る）。

## Task Breakdown

- T1: predicate 修正 + unit test（1 file に収まるため単一 task で十分）

## Verification

- [ ] `mise exec -- cargo test -p cheaper-core` が pass
- [ ] `mise run check` が pass
- [ ] win32: `is_inside_workspace_root("C:\\", "C:\\resource-probe",
  true)` → true（修正前 fail・修正後 pass）
- [ ] posix: `is_inside_workspace_root("/", "/a", false)` → true
- [ ] `is_inside_workspace_root("C:\\", "D:\\x", true)` → false
- [ ] `resolve_workspace_path` が volume-root で inside path を
  `real`/`display` 期待値どおりに resolve する
- [ ] 修正後の guard が `resource_contains`（predicates.rs:36-40）と
  同一 pattern であることを code review で確認する

## Open Questions / Gaps

- なし（defect・修正・test 構成とも確定済み）

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（単一 file の小さい修正のため
  委譲不要）
- Independent work streams: none
- User approval required before dispatch: yes（review-protocol による
  承認で代替）
- If approved: T1 を実装し verification matrix で検証する。
- If declined or unavailable: 同上（委譲前提ではない）。

## Review Handoff

- reviewer focus: prefix が normalized `r` から構築されているか、
  platform gate が適切か、一意名の衝突可能性が排除されているか。
