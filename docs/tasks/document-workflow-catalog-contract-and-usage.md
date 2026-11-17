---
id: "TASK-034ZiGQrT3mNkVx9sLpW7d"
type: "task"
status: "done"
title: "Document workflow catalog contract and usage"
created: "2026-11-17"
updated: "2026-11-17"
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
    - "docs/plans/implement-workflow-catalog-and-list.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/add-workflow-list-cli-command.md"
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
    - "docs/test-specs/workflow-catalog-manifest-and-list-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# Document workflow catalog contract and usage

## Work

`workflow list` と catalog file 契約を user-facing guide に文書化する。
`docs/guides/usage.md` に `workflow list` の command/option/envelope/
exit 契約、`docs/guides/parent-agent-integration.md` に project 内の
保存場所規約(`.cheaper/workflows/`)+ catalog が発見用 index で権限/
実行の正本でないこと、`docs/guides/workflow-usage.md` に catalog
manifest schema(schemaVersion 1・bound・entry 項目・diagnostics
kind)と create skill(#204)向けの安全な登録/更新手順例を記載する。

## Classification

- `normal-plan-task`

## Upstream Documents

- plan: `../plans/implement-workflow-catalog-and-list.md`
- spec: `../specs/workflow-catalog-issue-201.md`(AC8)
- design: `../designs/workflow-catalog-manifest-and-read-only-list.md`

## Dependencies

- `add-workflow-list-cli-command.md`(実装契約の確定後に書く)。

## Affected Files

- `docs/guides/usage.md`
- `docs/guides/parent-agent-integration.md`
- `docs/guides/workflow-usage.md`

## Done When

- [ ] `workflow list` の受理 option・envelope result 形状・exit が
  usage.md に記載される。
- [ ] catalog file 契約(schemaVersion・entry 項目・bound・diagnostics)
  と登録/更新例・`workflow list` での検証手順が文書化される。
- [ ] catalog が発見用 index で、権限・実行の正本でないこと、named
  lookup を意味しないことが明記される。
- [ ] project 内の重複・保存場所規約が guide に反映される。

## Verification

- [ ] `mise exec -- node .agents/skills/doc-status/scripts/doc_status.js audit --type all --blocking --json` で blocking finding なし。
- [ ] `mise run check` が exit 0。
