---
id: "IMPL-034zQU77PE0beEruVCSGYz"
type: "impl"
status: "completed"
title: "Core resource manifest implementation"
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
    - "docs/tasks/core-resource-manifest-implementation.md"
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


# Core resource manifest implementation

## Summary

ScopeDeclaration / canonical ResourceClaim の単一正規化境界と、pure resource / claim / manifest predicates を実装した。

## Context

承認済み spec / design / plan と共有 ADR D4/D5 に従う。implementation-flow は dispatch の明示 stack（TDD、incremental-implementation、impl-doc）を使用。source 変更前に本 IR を開いた。

## Implementation

### Changes

resources/normalization.ts は厳密 admission、既存 canonicalizeWorkspaceRoot / resolveWorkspacePath.real、unknown fallback、bucket mode 導出、glob safety / producer ownership、安定 sort/dedup を所有する。predicates.ts は IO-free containment / overlap / conflict。resources/index.ts が types.ts を再 export し、core barrel の既存 resources export 一行を置換した。

### Decisions During Implementation

within-workspace glob は literal ancestor の boundary 検査後に root subtree、deny は ancestor。produces は literal identity / terminal **+subtree のみ。canonical input の metachar は glob に再解釈しない。POSIX backslash は meta、Windows は separator。独自 filesystem traversal、lease、DAG edge、依存変更はない。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- focused RED は前 IR に保存。
- initial focused: 79 cases、76 pass / 2 skip / 1 timeout（Windows symlink fixture、5000ms、実測 10.57s）。
- systematic-debugging: 同一失敗ケースを単独再実行し 1 pass / 78 skip、790ms、exit 0。source の挙動を変更せず suite を再実行。
- focused GREEN: 77 passed / 2 skipped（79）、487ms、exit 0。POSIX case / backslash の2件は host 条件 skip。
- core build: tsc -b、exit 0。
- Biome 対象 source / test / barrel: No fixes applied、exit 0。

## Risks

単独では再現しない一時 filesystem timeout があった。canonical suite で再発を確認する。literal exact/subtree 内 symlink 到達集合 / hard-link / 外部差替え、filesystem-root paths.ts 欠陥は既存限界。

## Follow-ups

canonical check と graph exit-audit を verification task で行う。
