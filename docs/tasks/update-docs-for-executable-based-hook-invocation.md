---
id: "TASK-034z52VFF0DTrVukTAbzaf"
type: "task"
status: "done"
title: "update docs for executable-based hook invocation"
created: "2026-10-02"
updated: "2026-10-02"
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
    - "docs/plans/implement-cheaper-hook-subcommands-and-migrate-hooks-json-off-node-adapters.md"
  implemented-by:
    - "docs/impl/ir/docs-for-executable-hook-invocation.md"
  depends-on:
    - "docs/plans/implement-cheaper-hook-subcommands-and-migrate-hooks-json-off-node-adapters.md"
    - "docs/tasks/verify-launcher-deploy-and-switch-hooks-json-to-executable-invocations.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/cheaper-hook-subcommands-and-hooks-json-executable-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# update docs for executable-based hook invocation

## Work

hook 呼び出しが executable 経由に切り替わったことを docs に反映する。

- `packages/apm/.apm/skills/cheaper-usage/SKILL.md` の transitional
  記述（`node ./cli/bin/cheaper.mjs` fallback・hook entries will
  migrate ...）を削除・更新
- `docs/guides/parent-agent-integration.md` の adapter file 名
  言及（:76,82,86,100 付近）と `cli/bin/cheaper.mjs` 検証コマンド
  を executable 経由に更新
- `cli/bin/cheaper.mjs` の去就を fallback として明記
- usage.jsonl に hook 計測が記録されなくなることを `docs/guides/
  usage.md` 等に明示

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-cheaper-hook-subcommands-and-migrate-hooks-json-off-node-adapters.md

## Dependencies

- docs/tasks/verify-launcher-deploy-and-switch-hooks-json-to-executable-invocations.md

## Affected Files

- `packages/apm/.apm/skills/cheaper-usage/SKILL.md`
- `docs/guides/parent-agent-integration.md`
- `docs/guides/usage.md`（必要に応じて）

## Done When

- [ ] docs 上の `node ./...` 前提の記述が executable 経由に更新さ
      れる
- [ ] `usage.jsonl` 非記録の決定が文書化される

## Verification

- [ ] `mise run check` 緑（doc-id check 含む）
