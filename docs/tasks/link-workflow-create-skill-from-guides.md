---
id: "TASK-034Zuee3qE5UZ7VZ4nRMwG"
type: "task"
status: "done"
title: "link workflow create skill from guides"
created: "2026-11-18"
updated: "2026-11-18"
owners: []
relations:
  source: []
  changes:
    added:
      - {"type":"doc","path":"docs/impl/ir/link-workflow-create-skill-from-guides.md"}
    modified:
      - {"type":"doc","path":"docs/guides/workflow-usage.md"}
      - {"type":"doc","path":"docs/guides/parent-agent-integration.md"}
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-workflow-create-skill.md"
  implemented-by:
    - "docs/impl/ir/link-workflow-create-skill-from-guides.md"
  depends-on:
    - "docs/plans/implement-workflow-create-skill.md"
    - "docs/tasks/author-cheaper-workflow-create-skill.md"
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
    - "docs/test-specs/workflow-create-skill-verification.md"
  references: []
  defers: []
  deferred-by: []
---

# link workflow create skill from guides

## Work

`cheaper-workflow-create` skill への最小導線を guide へ追加する。
`docs/guides/workflow-usage.md` の catalog 節(登録・更新と検証)に
create skill が手順を担う旨を追記し、
`docs/guides/parent-agent-integration.md` の skills 一覧を 6 件へ
更新して `cheaper-workflow-create` の役割(Workflow JSON v1 の
設計・検証・登録、定義保存と実行の分離)を記載する。impl record
を作成する。

## Classification

- `normal-plan-task`

## Upstream Documents

- plan: `../plans/implement-workflow-create-skill.md`
- design: `../designs/workflow-create-skill.md`
- test-spec: `../test-specs/workflow-create-skill-verification.md`

## Dependencies

- `author-cheaper-workflow-create-skill.md`(確定した skill 名・手順を
  guide が指すため)。

## Affected Files

- `docs/guides/workflow-usage.md`
- `docs/guides/parent-agent-integration.md`
- `docs/impl/ir/link-workflow-create-skill-from-guides.md` (new)

## Done When

- [ ] workflow-usage.md の catalog 節が create skill への導線を
  持ち、登録手順の正本として整合する。
- [ ] parent-agent-integration.md の skills 一覧が 6 件で
  `cheaper-workflow-create` を含み、承認境界(skill は実行・権限
  変更をしない)と整合する。
- [ ] impl record が変更・検証・裏付け実行内容を記録する。

## Verification

- [ ] `mise exec -- node .agents/skills/doc-status/scripts/doc_status.js audit --type all --blocking --json` — blocking 0。
- [ ] `git diff --check` clean。
- [ ] `mise run check` — exit 0(integration 完了時 1 回)。
