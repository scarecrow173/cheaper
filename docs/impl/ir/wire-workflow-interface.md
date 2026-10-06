---
id: "IMPL-034zTIoMYLPhPUbedvunjx"
type: "impl"
status: "completed"
title: "wire-workflow-interface"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "packages/cli/test/workflow-interface.test.ts"
      - type: "file"
        path: "packages/cli/test/e2e/workflow-interface.test.ts"
    modified:
      - type: "file"
        path: "packages/cli/src/args.ts"
      - type: "file"
        path: "packages/cli/src/cli.ts"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/wire-workflow-interface.md"
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

# wire-workflow-interface

## Summary

Task A 完了後に Task B を開始。コード変更前にこの IR を開き、CLI の RED テストから進める。

## Context

承認済み workflow-file-interface spec/design/plan と共有 ADR D1/D5/D6。supervised dispatch の明示 skill 指示を優先し、meta profile 探索を省略して inline RED/GREEN・verification・review・Conventional Commit を実行する。

## Implementation

### Changes

workflow interface の args・HELP・utility envelope・error format 判定を接続した。公開 result は canonical WorkflowInterface の schemaVersion/inputs/outputs のみで、private jobs/DAG/prompt/tool parameters を含めない。

### Decisions During Implementation

metadata factory は既存 resolveWorker/createToolCatalog を使用する。Worker error は get 時まで保留し、canonical structural/schema/default error の優先順位を維持する。不在・不正 Worker name は undefined とし canonical definition error へ委譲する。compile/resource inference/LLM/tool/Worker を実行しない。parseArgs tokens により explicit --input - も拒否する。

## Related Experiments

### Adopted

なし（承認済み design に沿う既知の adapter）。

### Rejected

なし。

## Validation

CLI unit の追加直後は未実装 command により RED。実装後は新規 unit 26 tests、e2e 9 tests、既存 args 31 tests、main 22 tests が各単独 GREEN。単独 e2e は --config packages/cli/vitest.config.ts を指定し、既存 20s timeout を適用した。最初の root config 実行は 5s timeout と version 不足の policy fixture で失敗し、適切な config と version: 1 へ修正した。

mise exec -- pnpm lint（既存 warning 18 件、exit 0）/ pnpm typecheck / pnpm build は pass。差分 review で option 許可範囲、catch の kind、metadata error precedence、privacy と非実行境界を確認（P0/P1 なし）。canonical check deferred to coordinator merge run。

## Risks

global doc-status の既存別 chain findings は ownership 外。bundled CLI は coordinator の merge refresh 待ち。

## Follow-ups

#69 workflow run/engine は #62 依存で対象外。loader と validateInputs API は再利用可能。
