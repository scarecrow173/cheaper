---
id: "PLAN-034r2qodyxvN3g3xktwPOF"
type: "plan"
status: "completed"
title: "provider/auth guide runtime sync — implementation plan"
test-spec-skip: "Documentation-only change; verified by contract/link checks against implementation and doc audit, not executable tests."
created: "2026-09-19"
updated: "2026-09-19"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/provider-auth-docs-runtime-sync.md"
    - "../designs/provider-auth-guide-runtime-sync.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../designs/provider-auth-guide-runtime-sync.md"
  implemented-by:
    - "../tasks/sync-providers-guide-with-runtime-auth-surface.md"
    - "../tasks/verify-guide-consistency-and-run-doc-audit.md"
  depends-on:
    - "../adr/provider-auth-guide-auth-status-canonical-surface-auth-type.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../designs/provider-auth-guide-runtime-sync.md"
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

# provider/auth guide runtime sync — implementation plan

## File Map

| Path | Change | Reason |
|---|---|---|
| `docs/guides/providers.md` | modify | OAuth 例示・`auth status`・model 確認経路を実 contract に同期 |
| `docs/guides/usage.md` | check | `cheaper auth` surface の記述が矛盾しないか確認 |
| `docs/guides/configuration.md` | check | credential/config path の記述が矛盾しないか確認 |
| `README.md` | check | provider/auth への言及が矛盾しないか確認 |
| `packages/apm/.apm/skills/cheaper-usage/SKILL.md` | check | 全面同期は Issue #34、矛盾のみ確認 |

## Tasks

- **TASK-034r2qodyq0oWVUNGquMgr**: `docs/guides/providers.md` を
  runtime auth surface に同期（OAuth 例示・auth status・
  meta.model 確認経路）
- **TASK-034r2qodyDJxqrVcytvrso**: 整合確認（usage /
  configuration / README / cheaper-usage skill）・doc audit・
  status 同期・最終検証

## Dependency Graph

```
TASK-034r2qodyq0oWVUNGquMgr (providers.md sync)
   └─> TASK-034r2qodyDJxqrVcytvrso (consistency + verification)
```

## Risk Register

- xAI を OAuth 例から外すと `auth status xai` の
  `["api_key","oauth"]` output との差が気になる:
  例示は網羅ではなく、`auth status` を canonical 確認 command
  として明記することで吸収する。
- 修正中に他 guide との矛盾を見落とす: TASK-034r2qodyDJxqrVcytvrso
  で rg 突合を行う。

## Rollback

- docs のみの変更のため file revert で完全復帰。code 非変更。

## Verification Matrix

| Requirement | Verification |
|---|---|
| xAI が OAuth 例に含まれない | `rg "xai" docs/guides/providers.md` で OAuth 例示文脈に残っていないことを確認 |
| `auth status` が model 一覧を返す記述なし | `rg "model" docs/guides/providers.md` で記述突合 |
| `auth status` = auth capability 確認 command | 実 output（`result.providers[].methods` 等）と記述を突合 |
| model 実利用確認 = `meta.model` | envelope contract と記述を突合 |
| 他 guide との整合 | rg で auth/provider 記述を総当たり確認 |
| 回帰 | `mise run check` + `audit_docs.js --type all` no findings |
