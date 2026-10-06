---
id: "TSPEC-034zXj8km4o9pyOGLhz1oM"
type: "test-spec"
status: "approved"
title: "Concurrency policy の信頼境界"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/57"]
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
  derives-from:
    - "docs/designs/bounded-tool-scheduler.md"
  derived-by: []
  verifies:
    - "docs/specs/bounded-tool-scheduler.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Concurrency policy の信頼境界
## Purpose
less-trusted project layer による cap 緩和を防ぐ。policy 層の再設計時も legacy omission と trusted user 設定を区別するため存在する。
## Feature
bounded concurrency の定義、合成、制約。
## Rules
- Rule: complete は concurrency 全体の省略のみ許し、present 時は両 field が必須。partial は field omission を実指定に変えない。
- Rule: user は trusted replace、project は cap min と serialized 優先。default 補完は project restriction より前に共有 resolver で行う。
- Rule: direct CLI-style restriction は cap 増加・serialized から per-resource への緩和を拒否する。
## Examples
- Example: legacy builtin → user 16/per-resource → project 32/per-resource は16/per-resource。
- Example: user 16/per-resource → project 4/serialized は4/serialized。
- Example: user absent → project 16/per-resource は8/serialized。
- Example: user の mutations のみ指定は cap8を保持し、project の field 省略は trusted field を保持する。
- Example: 0、負数、fraction、unknown field/mutation は schema/resolver で拒否し、partial empty object は default を追加しない。
## Guarantees
AC7 の schema/normalize/merge/load/restrict を契約として保証する。他 scalar/array の既存 last-wins は維持する。
## Non-goals
CLI option、workflow/process policy schema、他 permission key の restriction 化を追加しない。
## Risk
補完順や omission を誤ると project が trusted cap を緩和するため、型だけでなく実ファイルの load 経路を検証する。
