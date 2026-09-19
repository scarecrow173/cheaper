---
id: "TASK-TttCTIVey8FHby8zR3UDqb"
type: "task"
status: "done"
title: "Add worker workload profile and auto tier classifier"
created: "2026-09-20"
updated: "2026-09-20"
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
    - "docs/plans/implement-auto-tier-classification.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-auto-tier-classification.md"
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
    - "docs/test-specs/auto-tier-classification.md"
  references: []
  defers: []
  deferred-by: []
---

# Add worker workload profile and auto tier classifier

## Work

core 層の変更:

- `worker/validate.ts`: `cheaper.model.workload.outputPerInput`
  （number ≥ 0・finite）を受理し `WorkerDefinition.cheaper.model.
  workload` に保持。未知 key・不正値を拒否
- `model/classify.ts`: interim `autoClassify` を削除し、
  `DEFAULT_OUTPUT_PER_INPUT`・`AutoClassificationDetail`・
  `buildAutoTierContext`・`classifyModels` を実装。midrank
  percentile と整数比較 boundary
- `packages/cli/workers/*.md` と `packages/apm/.apm/hooks/cli/
  workers/*.md` の built-in worker frontmatter に初期
  `outputPerInput`（bulk-reader 0.05 / code-writer 0.3 /
  doc-writer 1.0）を追加
- `packages/core/test/model/classify.test.ts` と worker validate の
  テストを ranking ベースに更新

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-auto-tier-classification.md`
- `docs/designs/auto-tier-classification.md`

## Dependencies

- なし（core 層の起点 task）
