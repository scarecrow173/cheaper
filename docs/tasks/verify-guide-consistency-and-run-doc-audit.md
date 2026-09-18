---
id: "TASK-034r2qodyDJxqrVcytvrso"
type: "task"
status: "done"
title: "Verify guide consistency and run doc audit"
created: "2026-09-19"
updated: "2026-09-19"
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
    - "docs/plans/provider-auth-guide-runtime-sync.md"
  implemented-by: []
  depends-on:
    - "docs/plans/provider-auth-guide-runtime-sync.md"
    - "docs/tasks/sync-providers-guide-with-runtime-auth-surface.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/provider-auth-guide-runtime-sync.md"
    - "docs/specs/provider-auth-docs-runtime-sync.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/provider-auth-docs-runtime-sync.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# TASK-034r2qodyDJxqrVcytvrso: Verify guide consistency and run doc audit

## Work

TASK-034r2qodyq0oWVUNGquMgr の修正後、周辺文書との整合を確認し
必要なら最小修正を行い、最終検証を実施する。

### 1. 整合確認

- `docs/guides/usage.md`: `cheaper auth` の command/option
  記述が providers.md の新記述と矛盾しないか確認。
- `docs/guides/configuration.md`: credential・config path
  （`<userConfigDir>/pi/auth.json`）の記述と矛盾しないか確認。
- `README.md`: provider/auth への言及と矛盾しないか確認。
- `packages/apm/.apm/skills/cheaper-usage/SKILL.md`:
  全面同期は Issue #34 の scope。本 task では providers.md
  修正と矛盾する記述（例: OAuth 例示・auth status の役割）
  があるか確認し、矛盾があれば最小修正に留める。

### 2. status 同期

- SPEC-034r2qodmutbif9L0DLskN → `implemented`
- PLAN-034r2qodyxvN3g3xktwPOF → `completed`
- TASK-034r2qodyq0oWVUNGquMgr → `done`
- 本 task → `done`

### 3. 最終検証

- `audit_docs.js --type all` no findings
- `mise run check`
- guide 間の relative link 存在確認

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/provider-auth-guide-runtime-sync.md`
- `docs/specs/provider-auth-docs-runtime-sync.md`
- `docs/designs/provider-auth-guide-runtime-sync.md`

## Dependencies

- `docs/tasks/sync-providers-guide-with-runtime-auth-surface.md`

## Affected Files

- `docs/guides/usage.md`（check / 必要時のみ modify）
- `docs/guides/configuration.md`（check）
- `README.md`（check）
- `packages/apm/.apm/skills/cheaper-usage/SKILL.md`（check /
  矛盾時のみ最小 modify）

## Done When

- [x] 4 つの整合確認対象に providers.md 修正と矛盾する記述が
  ない（または最小修正済み）
- [x] 上流 doc の status が同期されている
- [x] `audit_docs.js --type all` が findings なし
- [x] `mise run check` が pass

## Verification

- [x] `rg -n "auth|oauth|xai|provider" docs/guides/usage.md
  docs/guides/configuration.md README.md` で矛盾記述を総当たり確認
- [x] `audit_docs.js --type all` no findings
- [x] `mise run check` pass
