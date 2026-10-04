---
id: "IMPL-034zTWHPv92Mp7Ql9gnE5d"
type: "impl"
status: "completed"
title: "Workflow resource inference implementation"
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
    - "docs/tasks/implement-workflow-resource-inference.md"
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

# Workflow resource inference implementation

## Summary
catalog に optional pure resources descriptor を追加し、WorkspaceResourceInference を公開した。全 job safety manifest と既知 reads を分離し、producer dependency・conflict・duplicate ownership error を deterministic に導出する。

## Context
承認済み plan slice 1、local ADR と共有 ADR D1/D4/D5。spec/design gate msg_b28e978f1490、ADR gate、plan gate APPROVED を受領済み。dispatched inline stack は TDD + incremental-implementation + impl-doc + git-commit。taskBudgetCount=2、maxHops=11。

## Implementation
### Changes
tool-runtime catalog descriptor、workflow process/result 型、resource-inference.ts、index export、resource-inference.test.ts。
### Decisions During Implementation
宣言-only テストは metadata 非公開の catalog fixture を使用する。安全な fallback read は dependency knowledge に使わない。process config 自体は明示宣言なので outputs を omission fallback で消さない。known process produces の underclaim も拒否する。job ID の prototype key は null-prototype manifest record で保護する。
core normalizeResourceManifest/normalizeResourceClaim/resourceKey/claimContains/resourcesOverlap/claimsConflict の公開 API を利用。path identity/glob 解釈/境界処理は再実装していない。

## Related Experiments
なし。

## Validation
- 実装前 RED: workflow test command exit 1（未実装 inference export を参照する新規テスト）。
- 初回実装後 RED: 99 tests 中 process-config-input-output-edge の 1 failure。source outputs が fallback produces=[] へ消える責務違反を修正。
- slice 1 checkpoint workflow suite GREEN: 100 tests / 5 files（2026-10-04）。
- tool-runtime suite GREEN: 18 tests / 3 files。
- pnpm lint / pnpm typecheck / pnpm build: exit 0。
- canonical check deferred to coordinator merge run。

## Risks
Worker ancestor mkdir は root exclusive safety で保守的に serialize し得る。process executor/engine/scheduler は別 issue の責務。

## Follow-ups
slice 2 compiler consumer の統合を実施する。最終 review と scoped exit-audit はその後に実施。

## 最終 integration 検証
implementation review は APPROVED（msg_fb4b2821f57e、最終 code head f4a9976）。自己点検の authority regressions を含む最終 workflow suite は 109 tests / 5 files、tool-runtime は 18 tests / 3 files。lint/typecheck/build exit 0。初期 slice の 100 / compiler 初回 106 / 修正途中 108 は履歴として区別する。P2 mutation predicate 共有は coordinator 直接指示に基づく最小 core export と workflow import で対応済み。
