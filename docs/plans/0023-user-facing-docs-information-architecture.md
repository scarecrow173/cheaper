---
id: "PLAN-0023"
type: "plan"
status: "completed"
title: "User-facing documentation IA — implementation plan"
test-spec-skip: "Documentation-only change; verified by link/contract checks against implementation, not executable tests."
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/0023-user-facing-docs-information-architecture.md"
    - "../designs/0028-user-facing-docs-information-architecture.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../designs/0028-user-facing-docs-information-architecture.md"
  implemented-by:
    - "../tasks/0112-create-usage-configuration-guides-and-contributing.md"
    - "../tasks/0113-create-workers-integration-guides-and-reorg-providers.md"
    - "../tasks/0114-restructure-readme-docs-index-and-final-verification.md"
  depends-on:
    - "../adr/0026-readme-as-user-entrypoint-with-canonical-guides.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../designs/0028-user-facing-docs-information-architecture.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  plan:
    effort: "medium"
    verification-policy: "task-verified"
---

# User-facing documentation IA — implementation plan

## File Map

| Path | Change | Reason |
|---|---|---|
| `docs/guides/usage.md` | add | CLI contract の canonical user guide |
| `docs/guides/configuration.md` | add | config discovery/precedence の canonical guide |
| `docs/guides/workers.md` | add | worker の canonical user guide |
| `docs/guides/parent-agent-integration.md` | add | skills/hooks integration の canonical guide |
| `docs/guides/providers.md` | modify | 開発知識を分離し user-facing に再編 |
| `CONTRIBUTING.md` | add | 人間向け contributor entrypoint |
| `docs/README.md` | add | 3 分類の docs index |
| `README.md` | modify | user-facing entrypoint へ再構成 |

## Tasks

- **TASK-0112**: `guides/usage.md`・`guides/configuration.md`・
  `CONTRIBUTING.md` を実コード突合で作成
- **TASK-0113**: `guides/workers.md`・`guides/parent-agent-
  integration.md` 作成・`guides/providers.md` 再編
- **TASK-0114**: `README.md` 再構成・`docs/README.md` 作成・
  link/contract 検証・status 同期・最終検証

## Dependency Graph

```
TASK-0112 (usage/config guides + CONTRIBUTING)
   └─> TASK-0113 (workers/integration guides + providers reorg)
          └─> TASK-0114 (README + docs index + verification)
```

## Risk Register

- guide 内容の実装 drift: 全 command・path・env var を
  args.ts/dirs.ts/policy.yaml/worker 定義/hooks.json と突合
  （TASK-0114 で link + contract 検証）。
- README 早期再構成で guide 未存在 link: task 順で guide を先に
  作り、README は最後。
- providers.md の知識削除しすぎ: 移動先（CONTRIBUTING 検証節・
  EXP/IMPL 参照）を確保してから削る。

## Rollback

- docs のみの変更のため file revert で完全復帰。コード非変更。

## Verification Matrix

| Requirement | Verification |
|---|---|
| README 11 項目構成 | TASK-0114 で構成チェック + build/bundle/path 不在を rg 確認 |
| 5 guide + CONTRIBUTING + docs index 存在 | file 存在確認 |
| canonical source | README 本文に詳細複製がないことを目視 + rg |
| implementation-verified | コマンド・path・worker・env var を実コードと突合 |
| link integrity | README・docs/README.md の全 relative link 存在確認 |
| 回帰 | mise run check（code 非変更でも contract 確認）+ doc audit |
