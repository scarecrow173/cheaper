---
id: "TASK-0051"
type: "task"
status: "todo"
title: "Run document status audit"
created: "2026-09-15"
updated: "2026-09-15"
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
    - "docs/plans/0003-implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0003-implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md"
    - "docs/tasks/0050-write-implementation-record-and-experiment-log.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/0003-implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Run document status audit

## Work

PLAN-0003 step 13。`audit_docs.js`（全 type）を実行し No findings であること、各 index が最新であることを確認。task の status を done に更新して完了とする。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0003-implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md`

## Dependencies

- `docs/tasks/0050-write-implementation-record-and-experiment-log.md` — 依存先（Dependency Graph）

## Affected Files

- `docs/tasks/README.md`
- `docs/plans/README.md`

## Done When

- [ ] 失敗するテスト（または scaffold 検証コマンド）を書く
- [ ] 対象テストを実行し red を確認する
- [ ] 最小実装を行う
- [ ] 再実行で green を確認する
- [ ] `pnpm biome check` と `tsc -b` が pass
- [ ] audit が No findings、index が最新

## Verification

- [ ] `mise exec -- node .agents/skills/doc-status/scripts/audit_docs.js --type task` exits 0
- [ ] `mise exec -- node .agents/skills/doc-status/scripts/audit_docs.js --type plan` exits 0

## Out of Scope

- exit-audit ノードの全ゲート確認は route_graph 側の責務。
