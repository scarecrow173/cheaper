---
id: "IMPL-034AGM8MC9yqj5X6IozzSu"
type: "impl"
status: "completed"
title: "Implement process tree lifecycle"
created: "2026-10-05"
updated: "2026-10-06"
owners: []
relations:
  source: []
  changes:
    added:
      - { type: "file", path: "packages/tool-runtime/src/process-tree.ts" }
    modified:
      - { type: "file", path: "packages/tool-runtime/src/process.ts" }
      - { type: "file", path: "packages/tool-runtime/test/process.test.ts" }
      - { type: "file", path: "packages/tool-runtime/test/fixtures/process-child.mjs" }
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/implement-process-tree-lifecycle.md"
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

# Implement process tree lifecycle

## Summary

E slice の executor-owned timeout/capture/heartbeat と全 terminal path の tree cleanup を実装した。coordinator implementation review は APPROVED、P0/P1 なし。

## Context

approved PLAN-034AEwZy3GcNygp87aXrL7 と TASK-034AGiFXd7uNwKAYJMW2TP。Windows ancestry-loss W1 は承認済み limitation、Job Objects と artifact handoff は延期する。

## Implementation

### Changes

新規 backend に OS helper deadline と identity snapshots を隔離し、executor に timers と idempotent cleanup を集約する。

POSIX group は SIGTERM→grace→SIGKILL。Windows は2秒/1MiB bounded helper と creation identity を用いて taskkill /T→/T /F を行い、root exit 後の live child/grandchild を追う。capture は shared 16MiB ceiling と resolved policy cap を使用し drain を維持する。診断は bounded runtime stderr JSON のみ、backpressure では drop する。

### Decisions During Implementation

既存 close-only 待機は descendant が pipe を保持すると settle しない。exit で cleanup を開始して close と cleanup の両方を待つ。abort reason は cleanup error より優先する。

## Related Experiments

### Adopted

なし。承認済み設計を実装した。

### Rejected

なし。

## Validation

RED→GREEN の evidence を以下に追記する。

- RED: `mise exec -- pnpm exec vitest run packages/tool-runtime/test/process.test.ts -t 'policy combined output cap|heartbeat only'` は3 failed。31/98304 caps が65536になり heartbeat が欠落。失敗 test の pending promise の rejection handler を追加した。
- E suite: 21 tests 中19 passed / POSIX-only 1 skipped / 既存 cancellation 1 timeout。新実 tree normal/nonzero/abort と timeout tests は pass。既定5秒の test deadline は新 bounded Windows helper+grace に不足した。
- 修正後: `mise exec -- pnpm exec vitest run packages/tool-runtime/test/process.test.ts -t 'managed parent and descendant'` は1 passed /20 deselected（9.25s）、test deadline のみ15秒に拡張。
- `mise exec -- pnpm --filter @cheaper/tool-runtime build` exit 0。
- Biome changed files check/write exit 0。最終全 focused suite は R で検証する。
- 最終 core/tool-runtime/Pi focused suites は21 files /329 passed/1 POSIX-only skipped（94.02s）。Windows の normal/nonzero/abort/timeout 三世代 tree と native spawn failure の coverage を R で追加し GREEN。POSIX native は未実行で、backend の5 deterministic tests は別の evidence とする。

## Risks

Windows の初回 identity snapshot 前に ancestry を失った descendants は W1 limitation。POSIX は本 Windows checkout で未実行。

## Follow-ups

R slice で OperationManager timed_out と managed drain を統合し final focused/root checks を完了。coordinator verdict（question msg_b41a1970f2fd）で implementation review/test-spec acceptance APPROVED、P0/P1 なし。bundle freshness の1 CLI failure は coordinator merge 時の pnpm bundle + mise run check に defer することが承認された。canonical check deferred to coordinator merge run。
