---
id: "TASK-vhu8Z2b2Ylehk8LEqt43Ke"
type: "task"
status: "todo"
title: "Wire auto classification into resolver, runtime, and CLI"
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
    - "TASK-TttCTIVey8FHby8zR3UDqb"
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

# Wire auto classification into resolver, runtime, and CLI

## Work

- `model/resolve.ts`: `resolveModel` / `resolveExplicitModel` に
  `outputPerInput` opt を追加し eligible 母集団で `classifyModels`
  一括分類。`ResolvedModel.auto` に分類根拠を載せる
- `runtime-pi/model-runtime.ts`: `resolvePiModel` /
  `resolveExplicitPiModel` に `outputPerInput` を伝播、
  `ResolvedPiModel.auto` を追加
- `runtime-pi/run.ts`: `worker.cheaper.model.workload?.
  outputPerInput` を resolver に渡し、`meta.model` に
  `effectiveCost` / `effectiveCostPercentile` / `workload` を記録
- `cli/args.ts`: `cheaper models --worker <name>` を追加
- `cli/cli.ts`: `executeModels` が worker workload で分類し、行に
  分類根拠を含める
- 対応する unit / e2e テストを更新

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-auto-tier-classification.md`
- `docs/designs/auto-tier-classification.md`

## Dependencies

- `TASK-TttCTIVey8FHby8zR3UDqb`（core classifier API に依存）
