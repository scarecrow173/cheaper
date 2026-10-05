---
id: "IMPL-034AH5aBMIDz91t2o5olxp"
type: "impl"
status: "completed"
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

R slice で managed process の timeout state と owner-scoped actual-settle drain を統合した。coordinator implementation review/test-spec acceptance APPROVED、P0/P1 なし。

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
- 承認後の終端 status/index に対する scoped exit-audit も542 files /462 findings /48 global blocking /0 scoped blocking。自 chain findings=0、共有 ADR 既存 nonblocking warning 2件。97 IR audit は findings=[]、git diff --check は pass。

## Risks

Windows W1 と未実行 POSIX を明記する。canonical check は coordinator merge run に defer する。

## Follow-ups

coordinator verdict（question msg_b41a1970f2fd、2026-10-06）: APPROVED implementation review + test-spec acceptance（review-protocol、P0/P1 なし）。ceiling/layering/spawn timeout/first-cause/timed_out/heartbeat/capacity/all-terminal cleanup と全 AC coverage を確認した。P3 nits は non-blocking とされ、追加の必須修正はない。

CLI の1 failure は stale bundle freshness のみと確認され、coordinator が merge 時に pnpm bundle + mise run check を行う defer が APPROVED。worker では bundle 再生成しない。canonical check deferred to coordinator merge run。Windows W1、Job Objects と streaming artifact writer は accepted ADR の延期範囲を保持する。

承認の履歴: active Dispatch 不在による ask 拒否（ef4b88c9-f99b-4c35-b8f2-e790511e94e2）の後、ctx_694611e9a9d7 で質問 msg_b41a1970f2fd を提出した。600000ms の初回/第1 resume が timeout し、同じ質問を第2 resume して上記 verdict を取得。status msg_006f3a8762b3 と escalation msg_c725cb4ec82b で既存質問の回答を依頼し、重複質問・旧 IDs の lifecycle 再送・自己承認は行っていない。

review 前の graph は task-graph-to-active-implementation / implementation-flow、R のみ active、Task DAG issues=[]、approval-required yield。承認後は spec implemented / plan completed / tasks done / IR completed / test-spec approved とし、4 generated indexes は maintenance plan→apply、IR index は共通 brief の例外による own 3 status cells の機械的更新を適用した。scoped exit-audit は blocking 0。declared graph の終端 trace は worker report に記録する。
