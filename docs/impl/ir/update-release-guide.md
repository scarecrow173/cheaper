---
id: "IMPL-034z2rYSzkxUjQSkAXW3IP"
type: "impl"
status: "completed"
title: "update release guide"
created: "2026-10-02"
updated: "2026-10-02"
owners: []
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
    - "docs/tasks/update-release-guide-for-manual-sea-release-workflow.md"
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

# update release guide

## Summary

`docs/guides/release.md` の release 手順を、手動 dispatch
ベースの `release` workflow（`.github/workflows/release.yml`）に
沿った手順へ更新した。

## Context

docs/tasks/update-release-guide-for-manual-sea-release-workflow.md
（PLAN-034z2bk7AEzZ66Mt0uXCyJ task 2）。

## Implementation

### Changes

- 旧 step 6「tag / GitHub Release は自動化していない」を
  手順 6-8 に置換: tag push → Actions の `release` workflow を
  `tag` input で手動 dispatch → workflow が build・smoke・
  artifact upload を実行
- workflow が `workflow_dispatch` のみで自動実行されない点、
  macOS artifact が対象外の点、version 不一致時に fail する
  挙動を明記

### Decisions During Implementation

- guide の記述は workflow 実体（trigger・input・matrix・
  version 検証）と 1:1 で対応させ、workflow 側の変更が
  guide のみのずれを生まないよう対応表として記述

## Related Experiments

### Adopted

なし

### Rejected

なし

## Validation

- `release.yml` の `on:`（`workflow_dispatch` のみ・`tag`
  input 必須）・matrix 3 platform・version 検証 step と
  guide 記述の整合を確認

## Risks

- workflow 変更時に guide の追随漏れリスク（両者は同じ plan
  の管理下）

## Follow-ups

- macOS target 追加時に guide の platform 記述を更新
