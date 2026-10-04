---
id: "IMPL-034zQKrvQjUkhvZoLktZcb"
type: "impl"
status: "in-progress"
title: "Core resource manifest tests"
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
    - "docs/tasks/core-resource-manifest-tests.md"
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


# Core resource manifest tests

## Summary

承認済み plan Coverage (a)〜(h) と AC1–5 の failing contract tests を作成する。

## Context

spec/design approval msg_51dda51d3eb5、plan approval msg_4f465c6e0d66（coordinator commit 7afaad2）。D4/D5 固定型を変更しない。TDD と incremental-implementation を使用し、dispatched worker の明示手順に従って implementation-flow の profile 再生成は省略する。

## Implementation

### Changes

test/resources.test.ts を作成する。source は RED 確認後の implementation task が所有する。

### Decisions During Implementation

canonical manifest admission を declaration と同じ boundary に集約し、globs と literal identities を分離して検証する。barrel は既存 export 行を置換する。

## Related Experiments

### Adopted

なし（approach は承認済み）。

### Rejected

なし。

## Validation

baseline: core 23 files / 291 passed / 2 skipped、exit 0。focused RED は未実行。

## Risks

POSIX 条件は Windows host では skip。既存 workspace-root boundary と literal subtree symlink residual-risk は scope 外。

## Follow-ups

RED 後 implementation task に進む。

