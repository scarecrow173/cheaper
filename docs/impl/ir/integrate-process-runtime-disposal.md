---
id: "IMPL-034AH5aBMIDz91t2o5olxp"
type: "impl"
status: "in-progress"
title: "Integrate process runtime disposal"
created: "2026-10-05"
updated: "2026-10-06"
owners: []
relations:
  source: []
  changes:
    added:
      - { type: "file", path: "packages/tool-runtime/test/process-tree.test.ts" }
    modified:
      - { type: "file", path: "packages/tool-runtime/src/registry.ts" }
      - { type: "file", path: "packages/tool-runtime/src/process.ts" }
      - { type: "file", path: "packages/tool-runtime/src/process-tree.ts" }
      - { type: "file", path: "packages/tool-runtime/test/process.test.ts" }
      - { type: "file", path: "packages/tool-runtime/test/fixtures/process-child.mjs" }
      - { type: "file", path: "packages/runtime-pi/test/process.test.ts" }
      - { type: "file", path: "docs/guides/managed-process.md" }
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/integrate-process-runtime-disposal.md"
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

# Integrate process runtime disposal

## Summary

R slice で managed process の timeout state と owner-scoped actual-settle drain を統合する。

## Context

TASK-034AGiKxX1qSMznOGzJrLb と approved PLAN-034AEwZy3GcNygp87aXrL7。E commit b320ad4 の backend/executor を再利用する。

## Implementation

### Changes

registry の AsyncLocalStorage operation ID から timeout の cancel を通知し、process invocation promises のみ actual settlement を待つ。

timer の onTimeout は running operation のみ cancel し、scheduler の finally が容量を解放するまで process promise を追跡する。generic manager.drain の後に process promises を await する。同期 native spawn failure も canonical bounded details にする。POSIX force phase は signal 送信だけで完了せず group 消滅を確認し、Linux zombie を稼働状態から除く。Pi adapter には policy cap と canonical result の保証を追加した。

### Decisions During Implementation

generic tools の detach contract と scheduler/resource finally を変更しない。managed cleanup は abortGraceMs より長くなり得るが backend の deadline で bounded とする。

## Related Experiments

### Adopted

なし。承認済み設計の実装であり exp は作成しない。

### Rejected

なし。

## Validation

RED→GREEN と final verification を以下に追記する。

- RED: scoped timeout/dispose tests は2 failed。TIMEOUT が failed と記録され、abortGraceMs=0 の dispose 後も tree が生存することを再現。
- RED: malformed native header の spawn failure は raw UNKNOWN で canonical TOOL_EXECUTION_FAILURE にならない。同期 spawn boundary を修正。
- GREEN: process/process-tree suites は31 passed /1 POSIX-only skipped（74.94s）。実 Windows tree、first-cause cleanup failure、backpressure、owner isolation、capacity を確認した。
- queue wait が deadline を超えたことを startedAt-createdAt>1000ms で直接確認する assertion と stubborn 三世代 fixture を追加。対象2 tests は GREEN（14.75s）。
- backend の deterministic tests は5 passed。graceful 後の force 回避、非 /F failure 後の escalation、PID reuse exclusion、identity query fail-closed、POSIX force 後の実終了待ちを確認。POSIX native の実実行 evidence ではない。
- `mise exec -- pnpm --filter @cheaper/tool-runtime build` pass。全 focused/CLI/root verification は下に追記する。
- 最終: `mise exec -- pnpm exec vitest run packages/core/test/policy packages/tool-runtime/test packages/runtime-pi/test/process.test.ts packages/runtime-pi/test/tool-runtime-contract.test.ts packages/runtime-pi/test/run.test.ts packages/runtime-pi/test/session.test.ts` は21 files /329 passed/1 POSIX-only skipped（94.02s）。
- `mise exec -- pnpm lint` / `pnpm typecheck` / `pnpm build` はすべて exit 0。canonical check は実行していない。
- CLI hook-contract の初回は13 passed/3 failed/1 skipped。5秒 test timeout 2件と bundled parity failure 1件。`--testTimeout 30000` の独立 rerun は15 passed/1 failed/1 skipped（114.88s）。全 suite GREEN とはしない。
- bundle の schema に maxOutputBytes がなく、bundled count-tokens を isolated config で直接実行すると `/process` の4 additional properties により INVALID_RUNTIME_POLICY となった。hook の fail-open が parity failure の原因。brief の bundle 再生成禁止を遵守し、coordinator に defer/例外許可を求める。
- 最終 doc-status all:542 files /462 findings /48 blocking。自 chain findings=0、共有 ADR の既存 warning 2件は nonblocking。既存 global findings は変更しない。task index は maintenance plan→apply で同期し、index mismatch 2件を解消した。

## Risks

Windows W1 と未実行 POSIX を明記する。canonical check は coordinator merge run に defer する。

## Follow-ups

実装 review と test-spec acceptance は coordinator の新 active Dispatch を必要とし、settled ID は再利用しない。

旧 terminal check は consumer_fenced。direct continuation status msg_370ea82a40f2 を Run に送り、新 Dispatch を依頼した。worker_done/heartbeat/ask に旧IDsを使っていない。test-spec TSPEC-034AGg47u5fWQqP0pEJfI9 は proposed、plan/task/IR の最終 closure は実装 review 待ち。

最終 gate の ask は active supervised Dispatch がないため拒否（mutation request ef4b88c9-f99b-4c35-b8f2-e790511e94e2）。pending question は作られていない。承認済み実装を commit し gate と CLI artifact constraint を記録したまま引き継ぐ。canonical check deferred to coordinator merge run。

最終 graph projection は task-graph-to-active-implementation、delegate=implementation-flow、Task DAG issues=[]、active は R のみ。run-until-yield の approval-required で停止し、implementation-verified / followup-terminal / exit-audit-pass の終端 signals はまだ送らない。IR と task/plan の最終 closure は review と bundle artifact verdict 後に行う。
