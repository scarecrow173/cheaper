---
id: "IMPL-034D9oWccQJEPfXsMNRpH5"
type: "impl"
status: "completed"
title: "Volume-root workspace boundary predicate の trailing separator guard"
created: "2026-10-09"
updated: "2026-10-09"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - { type: file, path: crates/cheaper-core/src/workspace/paths.rs }
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/fix-volume-root-workspace-boundary-predicate.md"
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
  verifies:
    - "../../test-specs/volume-root-workspace-boundary-predicate-behavior.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Volume-root workspace boundary predicate の trailing separator guard

## Summary

`is_inside_workspace_root` の boundary prefix 構築に
`ends_with(nodepath::SEP)` guard を追加し、volume-root workspace
（win32 `C:\` / posix `/`）で配下 path が outside と誤判定される
issue #77 を修正した。

## Context

- Issue: GitHub #77
- Task: `docs/tasks/fix-volume-root-workspace-boundary-predicate.md`
- Plan: `docs/plans/volume-root-workspace-boundary-predicate-fix.md`
  （approved）
- Design: `docs/designs/trailing-separator-guard-for-workspace-boundary-predicate.md`
  （approved、Option A）
- ADR: `docs/adr/guard-trailing-separator-in-workspace-boundary-predicate.md`

## Implementation

### Changes

- `crates/cheaper-core/src/workspace/paths.rs`
  - `is_inside_workspace_root`: prefix を normalized `r` から
    `if r.ends_with(nodepath::SEP) { r.clone() } else { format!("{r}{}", nodepath::SEP) }`
    で構築（`resource_contains` predicates.rs:36-40 と同一 pattern）。
  - test `is_inside_workspace_root_volume_root`: win32 gate で
    `("C:\\","C:\\resource-probe",true)`→true + trailing-sep 非
    volume root `C:\work\` case、posix gate で `("/","/a",false)`
    →true + `/work/` case、ungated で cross-drive
    `("C:\\","D:\\x",true)`→false を assert。
  - test `resolve_workspace_path_at_volume_root`:
    `cheaper-volroot-probe-<pid>` の nonexistent 一意名 tail で
    `real`/`display` を assert。win32 は raw `"C:\\"` root、
    posix は `canonicalize_workspace_root("/")`（trailing-sep
    生成 seam を pin）。

### Decisions During Implementation

- test は既存 `is_inside_workspace_root_cases` の拡張ではなく新規
  test fn にした。`#[cfg]` gate を fn 単位で配置する方が per-case
  gate より構造が明確なため（plan の file map を実装形に合わせて
  更新済み）。
- `r.clone()` は `resource_contains` の `parent.clone()` と同一の
  形で、normalized `r` が `c == r` で後続使用されるため必須。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `mise exec -- cargo test -p cheaper-core`: 22 test file suite 全
  pass（新規 2 test 含む）。
- `mise exec -- cargo test -p cheaper-core --lib volume_root`:
  新規 2 test が個別に pass。実行 host は Windows のため win32
  gate の assertion（`C:\` root + raw `C:\` での
  `resolve_workspace_path`）も実際に検証済み。
- `mise run check`: biome / cargo fmt / clippy -D warnings /
  version / doc-id / cargo test --workspace / node --test
  （scripts + benchmarks）/ bundle 再生成 / payload / freshness
  全 pass。
- review-protocol: 2 reviewer（cooperative/adversarial）が実装を
  検証。confirmed P0-P2 なし。P3 は task doc 更新・`r.clone()`
  nit（parity で意図的）・test 配置の doc 記述差分（plan 修正済み）。
  verdict: ACCEPT。
- `git diff` の変更範囲は `paths.rs` のみ（他 code file への
  混入なし）。

## Risks

- posix gate の test は本 host（Windows）では実行されない。Ubuntu
  CI（`.github/workflows/ci.yml`）で継続検証される。
- trailing-sep root `C:\work\` と sep なし `C:\work`（同一 dir）
  の一致は contract 外（入力は canonicalize 済み前提、spec で
  明示済み）。

## Follow-ups

- issue #77 への修正反映コメント + close。
