---
id: "IMPL-034A9Jn7YGuSj9pYwoT7f1"
type: "impl"
status: "completed"
title: "Implement managed process executor"
created: "2026-10-04"
updated: "2026-10-05"
owners: []
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
    - "docs/tasks/implement-managed-process-executor.md"
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

# Implement managed process executor

## Summary

Named process tool を native spawn、bounded capture、tree cancellation、invocation pool、Pi/Workflow 共有で実装した。

## Context

Task B / approved plan と shared ADR D2/D4/D5/D6/D7 に従う。承認 msg_126e0d3e23d0 / msg_e3d23f7a3334 の範囲。

## Implementation

### Changes

tool-runtime process/executable/catalog/registry/invocation/scheduler/index、core vocabulary/mutation predicate、Pi run/registry、Workflow inference、builtin allowlist ×2 と focused tests/guide を変更。既存 OperationManager と ResourceManifest normalization を再利用した。

### Decisions During Implementation

Invocation policy を snapshot、scope の omission は inherit、明示 mismatch は拒否。process pool と global capacity は実 settle まで保持。Workflow task.outputs を producer authority として使い underclaim/overclaim を拒否する。Windows libuv の required env 自動コピーを空値で遮断し、NODE_V8_COVERAGE の Node 自動コピーを own undefined で遮断する。content は64文字で折り返した表示、raw stream は details に保持する。

## Related Experiments

### Adopted

bounded text の64文字折返しにより、dense output に対する既存同期 BPE の停滞を防止した。専用 experiment log は作成せず focused test で確認した。

### Rejected

初回 binary output test は同期 tokenizer 停滞で worker を停止し、pass evidence にしなかった。JSON 分割表示も約26秒かかったため採用しなかった。

## Validation

- RED: Workflow process compile は旧 seam 必須チェックで失敗し、catalog fallback 実装後に通過。
- RED: queued input.task mutation により admitted readonly task が別の output task へ変わる回帰 test が失敗。registry で process input を一度 snapshot し、declaration/executor の両方に同じ値を渡して修正した。
- GREEN: `mise exec -- pnpm exec vitest run packages/tool-runtime/test packages/runtime-pi/test/process.test.ts packages/runtime-pi/test/run.test.ts packages/runtime-pi/test/tool-runtime-contract.test.ts packages/runtime-pi/test/tool-scheduler.test.ts packages/workflow/test packages/core/test/policy packages/cli/test/hook/routing-policy.test.ts` — 最終402 passed、27 files、exit 0（snapshot 修正前401 passed）。
- 追加の core tools/vocabulary、worker、resources、CLI workers/usage-report — 163 passed、2 platform skips、8 files、exit 0。
- 先行 focused native process/resolver test — 15 passed、2 files。
- `mise exec -- pnpm lint` — exit 0（既存 warnings と新 test の non-null assertion warnings、error 無し）。書式1件の初回 failure は修正して再確認。
- `mise exec -- pnpm typecheck` / `mise exec -- pnpm build` — snapshot 修正後も exit 0。lint も再度 exit 0。
- 実 parent/descendant PID の cancellation、生 stdout/stderr combined cap、非ゼロ failure、readonly pool/global cap、FIFO queued abort、Pi parity/runWorker propagation、Workflow compiled order と producer authority を検証。
- doc-status scoped audit: 自 chain の findings 0、共有 ADR の既存 warning 2件（非ブロッキング reciprocal relation）、scope 内 blocking 0。global は462 findings / 48 blocking で ownership 外。
- managed spec/plan/task index は doc-maintenance の preview→apply で再生成。IR index は共通 brief の承認済み例外に従い frontmatter と同じ status cell へ機械的置換した。

## Risks

Declared I/O は OS tracing/sandbox ではない。project task は arbitrary executable の事前認可。denyPaths/writeScope は宣言の検証に限る。setsid/re-parent で管理外になった子孫、正常終了後の子孫残留検査は対象外。process の未宣言 mutation を journal で網羅しない。

## Follow-ups

Coordinator implementation review: msg_602089b5d40f、APPROVED: implementation。13 契約の検証と queued snapshot 修正を確認し P0/P1 無し。8件の P2 note は非ブロッキングとの回答。canonical check deferred to coordinator merge run。#61 lease は別 issue。

Test-spec 二件は coordinator が approved に更新済み（msg_53bfa57ca3c4 / msg_723cd17c4ff4）。status message で届いた承認を inbox から受領し、自己承認せず確認した。
