---
id: "TASK-0065"
type: "task"
status: "done"
title: "Update docs indexes, supersede relations, and verify writer restructure"
created: "2026-09-16"
updated: "2026-09-16"
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
    - "docs/plans/0006-restructure-built-in-writer-workers.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/0063-migrate-cheap-writer-to-code-writer.md"
    - "docs/tasks/0064-add-doc-writer-worker.md"
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
---

# Update docs and verify writer restructure

## Work

README の alias 記述を `code-write`/`doc-write` へ更新し、SPEC-0003・DESIGN-0007
を superseded に更新（`superseded-by` 関係を設定）。specs/designs/plans/tasks/
discovery の各 index README に新規文書を登録し、全体検証と doc-status audit を
実行する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/specs/0006-code-writer-and-doc-writer-built-in-workers.md`
- `docs/designs/0011-writer-worker-contracts.md`

## Dependencies

- TASK-0063、TASK-0064

## Affected Files

- `README.md`
- `docs/specs/0003-cheap-writer-built-in-worker.md`（status → superseded）
- `docs/designs/0007-cheap-writer-contract.md`（status → superseded）
- `docs/{specs,designs,plans,tasks,discovery}/README.md` index

## Done When

- [ ] README が新しい built-in worker 構成を示す
- [ ] SPEC-0003/DESIGN-0007 が superseded で `superseded-by` が新文書を指す
- [ ] 各 index README に DISC-0004/SPEC-0006/DESIGN-0011/PLAN-0006/TASK-0063–0065 が登録される

## Verification

- [ ] `mise exec -- pnpm -r build && pnpm -r test && pnpm lint && pnpm typecheck` が全 green
- [ ] `node .claude/skills/doc-status/scripts/audit_docs.js --type spec` 等で重大な不整合がない
