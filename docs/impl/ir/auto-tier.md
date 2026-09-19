---
id: "IMPL-034rC4k38aJJf0vfVaefeV"
type: "impl"
status: "completed"
title: "auto tier 分類ガイドの更新"
created: "2026-09-19"
updated: "2026-09-19"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "file"
        path: "docs/guides/workers.md"
      - type: "file"
        path: "docs/guides/configuration.md"
      - type: "file"
        path: "docs/guides/providers.md"
      - type: "file"
        path: "docs/guides/usage.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/update-guides-for-auto-tier-classification.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/auto-tier-classification.md"
    - "docs/impl/ir/worker-workload-profile-auto-tier-classifier-core.md"
    - "docs/impl/ir/auto-resolver-runtime-pi-cli.md"
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

# auto tier 分類ガイドの更新

## Summary

issue #46 の auto tier 分類（workload 加重 effectiveCost +
percentile ranking）を user-facing guide に反映し、#45 の interim
heuristic 記述を置き換えた。

## Context

- task: `docs/tasks/update-guides-for-auto-tier-classification.md`
- spec: `docs/specs/auto-tier-classification.md`

## Implementation

### Changes

- `docs/guides/workers.md`: frontmatter 例に
  `workload.outputPerInput` を追加し説明を追記。built-in worker 表に
  各 `outputPerInput`（0.05 / 0.3 / 1.0）と暫定値である旨を記載。
- `docs/guides/configuration.md`: `models.yaml` の分類記述を
  interim heuristic から effectiveCost percentile ranking（下位
  1/3 → low 等）へ更新。`cheaper models --worker` を案内。
- `docs/guides/providers.md`: 同様に分類ロジック記述を更新し、
  `tierSource` / `effectiveCost` / `effectiveCostPercentile` の
  確認方法を追記。
- `docs/guides/usage.md`: option 表に `--worker`、meta.model の auto
  詳細フィールド、`cheaper models` の使用例と出力項目を追記。

### Decisions During Implementation

- ガイドでは internal 実装名（midrank・整数比較）ではなく、結果の
  振る舞い（下位/中位/上位 1/3、同額は同一 tier、単独 priced model
  は medium）を説明する方針とした。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `mise run check`（doc-ids・lint 含む）pass。

## Risks

- なし。

## Follow-ups

- なし。
