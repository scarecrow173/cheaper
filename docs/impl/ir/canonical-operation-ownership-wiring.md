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
実装レビューの非ブロッキング P2: pre-admission abort の素の Error は実 reason を保持しない。bare drain/dispose の first-cause は generic AbortError。未知 owner の drain は closedOwners に追加される。既存契約との互換性を保持したまま後続で検討し、本 slice の追加 runtime 修正は不要との coordinator 判断。

## Follow-ups

Workflow integration は #62、scheduler は #57、ResourceManager/lease は #61 の後続基準。現 slice では実装しない。
canonical check deferred to coordinator merge run。

## Review and graph completion

2026-10-04 coordinator の `APPROVED: implementation review` を受領（質問 msg_b176f79d5902、immutable range bdacfc0..9aecfb6）。遷移行列、first-cause、detach/late settle、double-settle 防御、bounded grace、signal 三段合成、accounting、D7 ownership と scope を独立検証済み。test-spec の draft status は指摘に従い approved へ整合させた。
follow-up は #57/#61/#62 と上記 P2 の scope 外改善候補のみで、担当 chain の未解決作業はない。graph は run-until-yield、taskBudgetCount=2/maxHops=11 を継続する。manager/wiring の commitGate は 8ce0860/9aecfb6 と clean worktree により解消した。終端の完全 GraphRoute、EffectOutcome、fingerprints は worker-56-report.md に保存する。
終端 trace: implementation-to-followup-triage → followup-triage-terminal → exit-audit-to-complete → complete（status=terminal / blockers=[]）。requiredAudits の安定順序 impl-record → task は findings=0 / blocking=0。all audit と doc-status delegate は担当 chain + 共有 ADR の blocking=0 を再確認した（global 455 findings / 41 blockers は ownership 外）。doc-maintenance plan/apply は test-spec index の status を再生成した。全実装 task が done、plan completed、spec/design/test-spec approved、local ADR accepted、IR completed。追加修正を要する follow-up はない。
