---
id: "PLAN-034r2puWPUDxxDyfTSffZR"
type: "plan"
status: "approved"
title: "Align usage.jsonl recording-scope docs — implementation plan"
test-spec-skip: "Doc-only wording alignment; verified by doc audit and diff review inside the task, not by executable behavior tests."
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/usage-log-recording-scope-docs.md"
    - "docs/designs/usage-log-recording-scope-docs.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/designs/usage-log-recording-scope-docs.md"
  implemented-by:
    - "docs/tasks/align-usage-log-docs-with-recording-contract.md"
  depends-on:
    - "docs/adr/usage-jsonl-contract-canonical.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/usage-log-recording-scope-docs.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  plan:
    effort: "small"
    verification-policy: "task-verified"
---

# Align usage.jsonl recording-scope docs — implementation plan

## File Map

| Path | Change | Reason |
|---|---|---|
| `docs/guides/usage.md` | modify | 「計測 log」節を canonical contract（記録対象・対象外の明示）に改訂 |
| `AGENTS.md` | modify | 「delegation 計測ログ」節を同じ contract に揃える |
| `docs/specs/delegation-usage-measurement-log.md` | modify | In Scope・Acceptance Criteria の記録対象表現を実装条件に一致させる |

## Tasks

- **TASK（single）**: 3 文書の記録対象記述を DESIGN-034r2lHf6FAFVelj9LrhZh
  の canonical contract 表に揃え、整合確認（grep・audit・diff）まで
  行う。変更は 1 つの論理修正なので task は 1 つ。

## Dependency Graph

```
single task（doc alignment + consistency verification）
```

## Risk Register

- 修正漏れ: grep で「実行ごとに」「全て記録」等の残存表現を確認。
- 過剰修正: 保存先・record shape・best-effort・quiet の記述には
  触れない（diff review で担保）。
- packages/ への混入: `git diff -- packages/` が空であることを
  verification に含める。

## Rollback

- 3 file の revert で完全復帰。doc のみで挙動影響は無い。

## Verification Matrix

| Requirement | Verification |
|---|---|
| run の記録対象（success + usage 付き error） | 3 文書の記述を `buildUsageRecord()` の分岐と目視突合 |
| run の非記録（usage を持たない failure・utility command） | 同上 |
| count の記録対象（tokens を持つ success のみ） | 同上 |
| 既存 contract の維持 | diff が記録対象節に限定されることを review |
| documentation-only | `git diff -- packages/` が空 |
| audit | `audit_docs.js --type all` no findings・`mise run check` pass |
