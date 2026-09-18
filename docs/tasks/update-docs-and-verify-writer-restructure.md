---
id: "TASK-034r05iHQeQpDpQ62tRnRU"
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
    - "docs/plans/restructure-built-in-writer-workers.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/migrate-cheap-writer-to-code-writer.md"
    - "docs/tasks/add-doc-writer-worker.md"
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

README の alias 記述を `code-write`/`doc-write` へ更新し、SPEC-034r05iHK8bA7g7YRKGyQr・DESIGN-034r05iHrZUE6Wy5X0Hc88
を superseded に更新（`superseded-by` 関係を設定）。specs/designs/plans/tasks/
discovery の各 index README に新規文書を登録し、全体検証と doc-status audit を
実行する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/specs/code-writer-and-doc-writer-built-in-workers.md`
- `docs/designs/writer-worker-contracts.md`

## Dependencies

- TASK-034r05iHQeQpDpMK3ZMC7C、TASK-034r05iHQeQpDpOWHdF7Wo

## Affected Files

- `README.md`
- `docs/specs/cheap-writer-built-in-worker.md`（status → superseded）
- `docs/designs/cheap-writer-contract.md`（status → superseded）
- `docs/{specs,designs,plans,tasks,discovery}/README.md` index

## Done When

- [ ] README が新しい built-in worker 構成を示す
- [ ] SPEC-034r05iHK8bA7g7YRKGyQr/DESIGN-034r05iHrZUE6Wy5X0Hc88 が superseded で `superseded-by` が新文書を指す
- [ ] 各 index README に DISC-034r05iHy2gdnp7GkO7QDK/SPEC-034r05iHK8bA7gaTnVhXWI/DESIGN-034r05iHy2gdnoDFAeW1DN/PLAN-034r05iHE1FbkoTVD7CTNi/TASK-034r05iHQeQpDpMK3ZMC7C–0065 が登録される

## Verification

- [ ] `mise exec -- pnpm -r build && pnpm -r test && pnpm lint && pnpm typecheck` が全 green
- [ ] `node .claude/skills/doc-status/scripts/audit_docs.js --type spec` 等で重大な不整合がない
