---
id: "TASK-0071"
type: "task"
status: "done"
title: "Create worker delegation skills"
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
    - "docs/plans/0008-apm-delegation-layer.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0008-apm-delegation-layer.md"
    - "docs/tasks/0070-create-cheaper-usage-skill.md"
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

# Create worker delegation skills

## Work

`.apm/skills/cheaper-{bulk-read,code-write,doc-write}/SKILL.md` を作成。
各 skill は WHEN/WHY（delegation 判断）+ input schema に沿った JSON の
組み立て例 + result/meta の消費方法を記述し、CLI 細部は cheaper-usage
を名前参照する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/designs/0013-apm-skills-and-pre-read-hook.md`
- `docs/specs/0008-apm-delegation-skills-and-read-hook.md`

## Dependencies

- TASK-0070

## Affected Files

- `.apm/skills/cheaper-bulk-read/SKILL.md`
- `.apm/skills/cheaper-code-write/SKILL.md`
- `.apm/skills/cheaper-doc-write/SKILL.md`（すべて新規）

## Done When

- [ ] 3 skill が実 worker の input/output schema と一致する
- [ ] CLI 細部の重複記述がなく cheaper-usage を参照している

## Verification

- [ ] schema との目視照合
