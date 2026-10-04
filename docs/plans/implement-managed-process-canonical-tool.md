---
id: "PLAN-034A91M2XAsWK6QNypcEPX"
type: "plan"
status: "completed"
title: "managed process canonical tool を実装する"
created: "2026-10-04"
updated: "2026-10-05"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/59"]
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/managed-process-canonical-tool.md"
  implemented-by: ["../tasks/implement-managed-process-policy.md", "../tasks/implement-managed-process-executor.md"]
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../specs/managed-process-canonical-tool.md"
    - "../designs/managed-process-canonical-tool.md"
  derived-by: []
  verifies: []
  verified-by: ["../test-specs/managed-process-task-authorization.md", "../test-specs/managed-process-lifecycle.md"]
  references: ["../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md", "../adr/managed-process-named-task-lifecycle.md"]
  defers: []
  deferred-by: []
---

# managed process canonical tool を実装する Implementation Plan

## Goal

認可済み named task を、既存 operation/scheduler と同じ process canonical tool から実行する。

## Scope

単一 release intent の依存する二 slice。A は core config/resource 契約、B は shell-free executor と adapter。依存追加、CLI 実行面、root scripts、共有 ADR、overview は変更しない。

## Upstream Documents

- approved spec `docs/specs/managed-process-canonical-tool.md`、accepted ADR `docs/adr/managed-process-named-task-lifecycle.md`。共有 ADR D1/D2/D4/D5/D7。

## Design Inputs

- `docs/designs/managed-process-canonical-tool.md`（coordinator 承認済み、P1/P2 補足反映済み）。

## File Map

| Path | Responsibility | Slice |
| --- | --- | --- |
| packages/core/src/policy/process.ts | task/schema/resolver、pure I/O declaration helper | A |
| packages/core/src/policy/schema.ts, load.ts, index.ts | optional process、default補完、export | A |
| packages/core/test/policy/process.test.ts | config/auth/merge/load/I/O 契約 | A |
| packages/cli/policy.yaml, packages/apm/.apm/hooks/cli/policy.yaml | process defaults（tools.allow の追加は B） | A/B |
| packages/core/src/tools/vocabulary.ts, worker/validate.ts | canonical process 名、mutation predicate | B |
| packages/tool-runtime/src/process.ts, process-executable.ts | metadata、cwd/I/O/scope、spawn/capture/tree kill、native executable 解決 | B |
| packages/tool-runtime/src/catalog.ts, registry.ts, invocation.ts, scheduler.ts, index.ts | process 登録、policy snapshot/継承、shared pool admission | B |
| packages/runtime-pi/src/tools/registry.ts, run.ts | capability に従う同一 executor への接続 | B |
| packages/workflow/src/resource-inference.ts | metadata fallback と producer authority の接続 | B |
| packages/tool-runtime/test/process.test.ts, process-executable.test.ts, fixtures/process-child.mjs | 実 process/SEA resolver/queue/pool/cancel検証 | B |
| packages/runtime-pi/test/process.test.ts, packages/workflow/test/process.test.ts | Pi adapter と Workflow compile/materialize/direct executor | B |
| packages/tool-runtime/test/runtime.test.ts, packages/workflow/test/resource-inference.test.ts | catalog 名と process 未宣言ケースの期待更新 | B |
| docs/guides/managed-process.md | 操作例、native/shim、declared-I/O/孫/認可の trust boundary | B |

## Implementation Sequence

- [x] plan 承認後、test-spec を作成する。
- [x] A/B の task と依存を作成し graph projection を確認する。
- [x] A の in-progress IR を開く。
- [x] core process policy の failing contract tests を実行する。
- [x] core config/resource 宣言の唯一の helper を実装する。
- [x] A の focused suite、lint/typecheck/build を検証する。
- [x] A の code/tests/IR/task を coherent commit にまとめる。
- [x] B の in-progress IR を開く。
- [x] 実 process、adapter、resource、scheduler の failing tests を実行する。
- [x] process を canonical runtime に登録し最初から capture と tree cancel を実装する。
- [x] invocation pool と scope policy を接続する。
- [x] Pi/Workflow adapter と guide を完成させる。
- [x] B focused suites と lint/typecheck/build を検証する。
- [x] coordinator に実装 review を ask し指摘を修正する。
- [x] task/IR/plan/spec を終端 status に更新し managed index を preview→apply する。
- [x] scoped exit-audit を実施する。
- 最終 commit、report、worker_done は branch の終了 handoff として実施する。

## Dependencies and Risks

- A → B。既存 OperationManager/ToolScheduler/Resource normalization を再実装しない。
- Windows native taskkill を利用する。setsid/re-parent 孫と正常終了後の孫残留は保証範囲外。
- project task 登録は任意 executable の事前認可。denyPaths は未宣言 process I/O を制約できない。
- pool の先頭待ちが後続の他 tool を stall する FIFO 契約を保持する。
- rollback は本 chain の二 coherent commit を revert。永続 format 移行や依存変更はない。

## Task Breakdown

- A: named task authorization/config/ResourceManifest declaration を core に実装する。
- B: A に依存し canonical spawn/capture/cancel/pool/adapters を実装する。

## Verification

| Slice | Command | Pass condition |
| --- | --- | --- |
| A | `mise exec -- pnpm exec vitest run packages/core/test/policy packages/core/test/resources.test.ts` | schema/resolver、unknown task、shell/cwd/env、trusted/project maxConcurrent、output ownership の全 test pass |
| B | `mise exec -- pnpm exec vitest run packages/tool-runtime/test packages/runtime-pi/test/process.test.ts packages/runtime-pi/test/tool-runtime-contract.test.ts packages/runtime-pi/test/tool-scheduler.test.ts packages/workflow/test` | readonly overlap/pool/global cap、FIFO、queued/running cancel、PID/孫 cleanup、bounded/nonzero result、Pi/Workflow と producer authority の全 test pass |
| A/B | `mise exec -- pnpm lint` | exit 0 |
| A/B | `mise exec -- pnpm typecheck` | exit 0 |
| A/B | `mise exec -- pnpm build` | exit 0 |

必要に応じ CLI/builtin policy の関連 test file を単独実行。canonical check deferred to coordinator merge run。resource-inference.test.ts:314–318（review時）の旧「process 未宣言」期待も更新する。既存 global doc findings は変更せず、自 chain + 共有 ADR の scoped audit を完了条件とする。

## Open Questions / Gaps

未解決の要件はない。plan approval と implementation review は coordinator 承認済み。test-spec は coordinator が approved に更新（msg_53bfa57ca3c4 / msg_723cd17c4ff4）した。

## Self-Review

- [x] 全 spec 要件を A/B と Verification Matrix に対応づけた。
- [x] placeholder はない。
- [x] path/relation/status 用語を確認した。

## Implementation Handoff

- 実装は inline。独立 stream はなく A→B の依存順を守る。
- 共通 brief の指示に従い nested dispatch は行わず、review は coordinator ask で行う。
- TDD と verification-before-completion を使用し、実測 evidence を IR に保存する。

## Review Handoff

- 重点: native executable/SEA、cap/default/restrict、catalog producer authority、writeScope、real PID cleanup と declared-I/O threat model。
- design P1/P2 補足を反映済み。stdin ignore、task空時 exposure拒否、scope省略時inherit、argv制約を維持する。
