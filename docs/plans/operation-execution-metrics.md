---
id: "PLAN-034A90VLWrJyT2IhkiTsA0"
type: "plan"
status: "completed"
title: "Operation execution metrics"
created: "2026-10-04"
updated: "2026-10-05"
owners: ["scarecrow173"]
relations:
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  source: []
  implements: ["../specs/operation-execution-metrics.md","../adr/operation-execution-metrics.md"]
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: ["../designs/operation-execution-metrics.md"]
  derived-by: ["../tasks/operation-execution-metrics.md"]
  verifies: []
  verified-by: ["../test-specs/operation-execution-metrics.md"]
  references: ["../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
  defers: []
  deferred-by: []
---

# Operation execution metrics

## Goal

共有 ADR D1/D3/D7 の execution 型を owner snapshot と success/error envelope に実装する。single release intent の一つの chain とする。

## File Map

- tool-runtime/src/execution-metrics.ts: 単一型と純粋 duration reducer、index.ts export。
- tool-runtime/src/operation.ts: owner/global running count/high-water と snapshot。invocation.ts: clock 注入と snapshot 委譲。scheduler.ts は無変更。
- runtime-pi/src/run.ts: WorkerRunMeta.execution、owned invocation への operationClock 注入、buildMeta と outer error snapshot（既存 error code/details/cause/usage/savings を保持）。
- cli/src/envelope.ts: execution 型と partial forwarding。既存 typed meta fixtures と関連 tests を追随する。error envelope 例示 docs を更新する。

## Dependency Graph

1. controlled clock の lifecycle/reducer/invocation tests を追加し RED 確認。
2. tool-runtime 実装 → focused suite GREEN + build。
3. Worker success/error/early error/owned+borrowed clock tests と CLI forwarding/null/zero tests を RED 確認。
4. runtime/CLI 実装と fixture 追随 → focused suites GREEN。
5. lint/typecheck/build、実装 review gate、impl record、generated indexes、scoped exit-audit、logical commit。

## Risks and Rollback

0 duration の max は timestamp だけでは復元できないため manager high-water を使用。detach は running を終了しない。byTool は prototype key を安全に保持。API 変更は additive で dependency 追加なし、revert で復旧できる。usage.jsonl は変更しない。

## Verification Matrix

- tool-runtime: execution-metrics と既存 operation/invocation/scheduler suites。
- runtime-pi: run と scheduler/sibling/usage suites（fake provider）。
- CLI: envelope/main/text-format/usage-log files 単独で実行。
- mise exec -- pnpm lint / pnpm typecheck / pnpm build。
- scoped doc-status findings 0。canonical check deferred to coordinator merge run。

## Tasks

単一 slice: deterministic operation metrics を全 meta 経路へ接続する。upstream spec/ADR/design が承認済み、plan は coordinator approval と implementation review 済み。
