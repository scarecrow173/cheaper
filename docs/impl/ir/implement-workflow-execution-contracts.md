---
id: "IMPL-034AJmHjccYgHI8SdxXrpS"
type: "impl"
status: "blocked"
title: "Implement workflow execution contracts"
created: "2026-10-05"
updated: "2026-10-06"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - { type: file, path: packages/core/src/policy/workflow.ts }
      - { type: file, path: packages/core/src/worker/write-scope.ts }
      - { type: file, path: packages/core/test/policy/workflow.test.ts }
      - { type: file, path: packages/core/test/write-scope.test.ts }
    modified:
      - { type: file, path: packages/core/src/index.ts }
      - { type: file, path: packages/core/src/policy/schema.ts }
      - { type: file, path: packages/core/src/policy/load.ts }
      - { type: file, path: packages/core/src/policy/merge.ts }
      - { type: file, path: packages/core/test/policy/load.test.ts }
      - { type: file, path: packages/runtime-pi/src/run.ts }
      - { type: file, path: packages/workflow/src/types.ts }
      - { type: file, path: packages/workflow/src/compiler.ts }
      - { type: file, path: packages/workflow/test/compiler.test.ts }
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/implement-workflow-execution-contracts.md"
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

# Implement workflow execution contracts

## Summary

S1 の WorkflowPolicy と literal writeScope の core 正本、compiler の validateJobOutput seam を実装した。focused verification は GREEN。coordinator の implementation checkpoint review は未承認であり、IR/task は blocked とする。

## Context

承認済み plan PLAN-034AGr55LYOkphfEZr5Pr2 の S1 と TASK-034AJcLMvVtCF2DauUO4It に従う。plan approval は ctx_7c15503824ef の新 ask で取得した（2026-10-06、review-protocol、P0/P1 なし）。nested workers は common brief により使わず、incremental/TDD を inline 適用する。

## Implementation

### Changes

- core policy に optional typed workflow と normalize/load defaults、schema/safe-integer caps、project min 四キーを追加した。二つの policy.yaml は未変更。
- literal extractWriteScope を core に移し、runtime-pi の既存 private 実装を削除して core import へ置換した。path canonicalization と permission narrowing は S4 の ResourceAccess が所有する。
- compiler に existing JobValidation.output を使う validateJobOutput を追加した。JSON snapshot/INVALID_OUTPUT と unknown job の INVALID_REQUEST を維持する。
- policy/helper/compiler の contract tests を追加し、canonical policy expected object に新 defaults を記録した。

### Decisions During Implementation

schema の workflow/artifacts は部分省略を許可し、trusted merge 後・project 前の resolveWorkflowPolicy で補完する。旧 typed RuntimePolicy を壊さない optional workflow を保持する。writeScope helper のエラー形と literal semantics は旧実装を移し、追加の path normalization を行わない。S1 で runtime-pi/run.ts の import を変える理由は正本を二重に残さないためで、S4 の binding/permission 動作はまだ変更しない。

新 helper の WorkerDefinition import を schema.js とした初回 core build が TS2305 で失敗した。rg で定義元 validate.ts を確認して型 import を修正し、core build を再実行して exit 0 を確認した。既存 canonical load assertion は workflow defaults 追加により差分が生じたため、仕様どおり expected fields を追加した。

### Review Checkpoint

source inspection で shared/local ADR の policy restriction、literal identity、compiler-owned output schema と single-worker regression を照合した。独立 coordinator implementation review を新 ask msg_33814be597e8 で依頼した。計3回・各600000msの待機後も未回答のため、承認前に task/IR を done/completed にしない。現 Dispatch の終了は #62/#69-B の実装完了を意味しない。

## Related Experiments

### Adopted

既知の approved design に従い exp は作成しない。RED/GREEN evidence を本 IR に記録する。

### Rejected

比較実験は行っていない。

## Validation

- RED: core policy/workflow + write-scope の20 tests失敗。legacy workflow omission は undefined、workflow section は schema拒否、helper/resolver未導入を確認した。
- RED: workflow compiler48 tests中2失敗。validateJobOutput が未導入、既存46 testsはpass。
- GREEN: core focused workflow/schema/load/merge/restrict/write-scope の6 files /59 tests pass。
- GREEN: workflow compiler1 file /48 tests pass。
- GREEN: runtime-pi run.test.ts 1 file /42 tests pass（core helper 移設後）。
- core build、workflow build exit 0。
- cross-package pnpm typecheck exit 0。既存 concurrency/process policy 2 files /46 tests pass。
- 引き継ぎ再開時の root `mise exec -- pnpm lint` / `mise exec -- pnpm build` は exit 0。lint の既存 warnings は維持した。
- a851872 は partial defaults test の期待値を resolver 自己比較から仕様の値へ固定した test-only commit。latest policy/workflow 1 file /12 tests pass、focused biome check pass。
- focused biome check --write が formatting/import を整えた。既存 compiler/test の non-null warning は変更しない。新 test の non-null assertion は除去した。
- 2026-10-06 planning named audits plan/task/test-spec は own findings 0、task graph issues []、runnable は S1 のみ。
- diff --check pass。source commits は 91ab4dc /9c0224e、追加 test-only commit は a851872。checkpoint review は未承認。

## Risks

S2 engine、S3 artifact read/publisher、S4 Worker binding/lease release、S5 CLI は未実装。これらの acceptance を S1 focused pass から推測しない。実 provider、canonical check は実施していない。root lint/build/typecheck の成功を統合完了へ拡大しない。

## Follow-ups

S1 checkpoint review を coordinator へ渡したが未回答。承認後に task/IR status と graph evidence を更新し、S2へ進む。settled Dispatch の lifecycle IDs と closed ask を再使用しない。canonical check deferred to coordinator merge run。
