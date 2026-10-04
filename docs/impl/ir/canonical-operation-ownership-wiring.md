---
id: "IMPL-034zTD17WKcBN1E1xeYf41"
type: "impl"
status: "completed"
title: "Canonical operation ownership wiring"
created: "2026-10-04"
updated: "2026-10-04"
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
    - "docs/tasks/wire-canonical-operation-ownership.md"
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

# Canonical operation ownership wiring

## Summary

canonical invocation の共有 manager と abort wiring を実装・検証した。code 変更前に IR を作成済み。

## Context

TASK-034zTjXTqV7nvYYkf1bUH9、manager commit 8ce0860、approved spec/design/plan と共有 D1/D3/D7。

## Implementation

### Changes

registry/invocation の shared manager と records、run.ts の reason 渡し、各契約 tests。

### Decisions During Implementation

既存 admitted callback と output Promise の意味を保持する。manager は scope owner を固定し、call signal は operation だけを cancel。Active stack は TDD/git-commit/impl-doc、追加 worker/profile routing は明示 dispatch chain で override。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

commitGate baseline は 8ce0860 と新規 IR/index dirty 集合。
invocation/runtime tests は 8 failures/14 pass の red（operations facade 未実装）→ tool-runtime 全 4 files/74 tests pass。
Pi reason tests は 4 failures の red（汎用 AbortError の reason）→ run/registry/contract 全 3 files/69 tests pass（2026-10-04 14:10 JST）。
最後の tool-runtime は 14:14 JST に 74 tests pass、invalid-input/unknown-tool が record 未生成で snapshot state を外から書換えできないことも確認。
`mise exec -- pnpm lint` exit 0（既存 18 warnings: bundled CLI maxSize と Workflow assertions）、`mise exec -- pnpm typecheck` exit 0、`mise exec -- pnpm build` exit 0。
`git diff --check` exit 0。canonical result identity、error throw、journal/meter/accounting、scope grace override を実 tests で確認。
Self-review: queued terminal と actual settle terminal の timestamp、root/owner/call first-cause、owner admission closure、bounded unsettled classification を AC と照合。coordinator の immutable commit review と scoped exit-audit を完了報告に記録する。
担当 chain + 共有 ADR の doc-status audit は blocking=0（共有 ADR の既存 reciprocal relation warning 2 件のみ）。global は 455 findings / 41 blockers（test-spec-invalid-verifies-target 12、test-spec-missing-verifies 3、traceability-missing-upstream 15、unresolved-legacy-reference 11）で ownership 外を変更していない。
impl-record audit は 76 files / findings=0。doc-maintenance plan/apply の今回の生成対象は adr/plans/tasks indexes。IR index は承認済み例外 method 1（既存 updateIndexForMarkdownDir を一時 .cjs から実行、削除）で更新した。

## Risks

bounded detach は実停止保証ではない。

## Follow-ups

Workflow integration は #62、scheduler は #57、ResourceManager/lease は #61 の後続基準。現 slice では実装しない。
canonical check deferred to coordinator merge run。
