---
id: "PLAN-034zTcqSwE6JHL46EJBnKP"
type: "plan"
status: "in-progress"
title: "OperationManager lifecycle の実装計画"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/56"]
  implements: ["../specs/operation-manager-lifecycle.md"]
  derives-from: ["../designs/operation-manager-lifecycle.md", "../adr/operation-manager-settlement.md"]
  verified-by: ["../test-specs/operation-lifecycle-settlement.md"]
  implemented-by: ["../tasks/implement-operation-manager-state.md", "../tasks/wire-canonical-operation-ownership.md"]
  references: ["../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
---

# OperationManager lifecycle の実装計画

## Goal and Scope

一つの実装 stream: manager の state/settle を定義してから canonical runtime と run abort に接続する。
scheduler/lease/Workflow/metrics/依存/hot file は対象外。共有 D1/D3/D7 を保持する。

## File Map

| Path | Responsibility |
| --- | --- |
| packages/tool-runtime/src/operation.ts | OperationState/record、validated lifecycle、owner cancellation、bounded drain |
| packages/tool-runtime/src/index.ts | manager/state 型の export |
| packages/tool-runtime/test/operation.test.ts | 全 transition matrix、signal reason、timestamp、manual/tracked settlement、finite drain |
| packages/tool-runtime/src/registry.ts | budget admission 内の tracked run、scope cancel/drain、read-only operation snapshot |
| packages/tool-runtime/src/invocation.ts | 1 manager を共有、root/owner lifetime、facade snapshot |
| packages/tool-runtime/test/invocation.test.ts | root/child/call signals、non-cooperative late settle、admission 閉鎖、sibling isolation |
| packages/tool-runtime/test/runtime.test.ts | failure/budget/boundary accounting と canonical result 保持 |
| packages/runtime-pi/src/run.ts | runAbort.signal.reason へ first-cause CheaperError を渡す最小 wiring |
| packages/runtime-pi/test/run.test.ts | timeout/signal/budget が scope signal.reason に渡ること |

## API and Compatibility

OperationManager は create/queue/start/complete/fail/cancel/cancelOwner/cancelAll/run/records/drain/dispose を提供する。
run は実 executor Promise に対して settle を一度だけ通知し result/throw をそのまま返す。
signal は owner/call/controller composite、first-cause の code/name/message を record に保存する。
queued cancel は即 timestamp を付けて terminal、running cancel は実 settle を待つ。
grace 超過で settlement=unsettled/owned=false/finishedAt absent、late settle で terminal を記録する。
InvocationToolRuntime と ToolRuntime は read-only records の snapshot 関数を持ち、binding に manager bag を露出しない。
createToolRuntime の standalone 使用は private manager、invocation の scope は同一 manager を借りる。
all/owner cancel は該当 admission を閉じる。terminal への明示遷移は reject、aggregate cancel は terminal skip。

## Implementation Sequence

1. manager の契約 tests を書く。
2. focused red を実行し未実装による失敗を確認する。
3. manager と barrel export を実装する。
4. manager focused green を実行する。
5. invocation/runtime と run reason の契約 tests を書く。
6. focused red を実行する。
7. registry/invocation/run reason の最小 wiring を実装する。
8. tool-runtime suite と runtime-pi run/registry の focused tests を実行する。
9. lint/typecheck/build を実行する。
10. impl evidence と task status を記録する。
11. 論理的変更を commit する。
12. index を plan/apply し scoped exit-audit を実行する。

## Dependency Graph

| Slice | Depends on | Spec coverage |
| --- | --- | --- |
| manager lifecycle | approved spec/design | AC1/AC2/AC3/AC4 |
| canonical invocation wiring | manager lifecycle | AC2/AC3/AC4/AC5/AC6 |

critical path は manager → wiring。共有責務のため inline sequential execution、追加 worker は不要。

## Risk Register

| Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- |
| 無協力処理の遅延 mutation | medium | high | 未 settle を明示、停止を偽装せず bounded detach のみ |
| result/accounting の互換性破壊 | low | medium | 既存 suite と result identity/error propagation assertions |
| 外部依存 availability | low | low | 依存追加・実 provider 使用なし、setup 済み toolchain を使用 |
| sibling scope の停止 | low | high | owner isolation regression tests |

## Rollback Strategy

永続 migration はない。review 時の regression が trigger なら coordinator が対応 logical commit を revert し、同じ focused suite/lint/typecheck/build で確認する。
本 worker は履歴を書き換えない。

## Verification Matrix

| Slice | Command | Pass |
| --- | --- | --- |
| manager | mise exec -- pnpm --filter @cheaper/tool-runtime exec vitest run test/operation.test.ts | green、全 matrix |
| wiring | mise exec -- pnpm --filter @cheaper/tool-runtime test | green、既存 18 tests も維持 |
| Pi reason | mise exec -- pnpm --filter @cheaper/runtime-pi exec vitest run test/run.test.ts test/tools/registry.test.ts test/tool-runtime-contract.test.ts | green |
| static/build | mise exec -- pnpm lint / pnpm typecheck / pnpm build | 各 exit 0 |
| docs | doc-status all + own chain/shared ADR filter、impl-record audit | scope blocking 0 |

canonical check deferred to coordinator merge run。CLI files は変更しないため CLI suite は実行しない。

## Self-Review and Handoff

AC1–6 を二つの slice と test-spec の guarantees に対応させた。未解決 dependency/要件はない。
plan approval 後に test-spec → tasks → in-progress IR を作り、各 slice を TDD で inline 実行する。
Dispatch-specific override: 明示された chain と skill stack を使い profile discovery/sub-worker は使わない。
review は coordinator の承認ゲートと完了報告で受ける。

2026-10-04 coordinator receipt: APPROVED: plan approval（P0/P1 なし）。test-spec で既存 abortGraceMs/default 5000・detachedAt・TIMEOUT/TimeoutError reason mapping を pin する。
