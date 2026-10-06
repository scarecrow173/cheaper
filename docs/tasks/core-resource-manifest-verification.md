---
id: "TASK-034zQHAqgNJHPLssPVaFPR"
type: "task"
status: "done"
title: "Core resource manifest verification"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-core-resource-manifest-normalization.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-core-resource-manifest-normalization.md"
    - "docs/tasks/core-resource-manifest-implementation.md"
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
    - "docs/test-specs/core-resource-manifest-normalization.md"
  references: []
  defers: []
  deferred-by: []
---


# Core resource manifest verification

## Goal

canonical check と focused exit-audit により配布可能な終端状態を記録する。

## Work Items

- [x] 対応 IR を開く。
- [x] mise run check の初回失敗と coordinator の延期指示を記録した。失敗4 file を単独再実行し、その結果と残存1 timeout の case-only pass を report に保存する。
- [x] 自 chain の terminal status と reciprocal verified-by / verifies relation を更新する。
- [x] doc-maintenance plan → apply で generated index を再生成する。
- [x] followup-triage、focused exit-audit、scope 差分を記録する。

## Done When

plan completed / tasks done / impl completed と focused graph complete。残存 boundary limitation は coordinator 引継ぎ済み。

## Verification

- [x] focused suite 77 pass / 2 POSIX skip、lint/typecheck/build は exit 0。full canonical は msg_5463b4cce5a1 により deferred-to-coordinator。3 CLI files は単独 pass、hook-contract は 15 pass / 1 skip / 1 timeout、残存 case の単独実行は 1 pass / 16 skip。
- [x] mise exec -- node scripts/check-doc-ids.mjs が exit 0。
- [x] git diff --check が exit 0。
- [x] focused graph exit-audit の各 gate が pass。
