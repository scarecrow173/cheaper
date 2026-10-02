---
id: "IMPL-034z3CEqPwEgMOcgeJ69S5"
type: "impl"
status: "completed"
title: "update cheaper-usage guidance"
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
    - "docs/tasks/update-cheaper-usage-skill-guidance-for-executable-based-cli.md"
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

# update cheaper-usage guidance

## Summary

`cheaper-usage` skill の CLI location guidance を launcher/
executable 基準に更新した。

## Context

docs/plans/assemble-apm-executable-payload-with-platform-launchers.md task 4。

## Implementation

### Changes

- `Locating the CLI` 節: APM 導入 workspace では
  `hooks/cheaper/cheaper`（POSIX）/ `cheaper.ps1`（Windows）
  launcher → `cli/bin/<platform>/cheaper[.exe]` を推奨経路に
- transitional 期間中は `cli/bin/cheaper.mjs`（Node 必須）も
  併存する旨を明記

### Decisions During Implementation

- hook 経路はまだ JS adapter のため、旧経路を削除せず
  fallback として残した（Phase 4 で移行）

## Related Experiments

### Adopted

なし

### Rejected

なし

## Validation

- 記述 path が実 tree（`hooks/cheaper`・
  `cli/bin/win32-x64/cheaper.exe`）と一致することを確認

## Risks

- なし

## Follow-ups

- Phase 4 完了時に transitional 記述を削除
