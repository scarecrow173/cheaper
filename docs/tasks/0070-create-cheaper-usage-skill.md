---
id: "TASK-0070"
type: "task"
status: "done"
title: "Create cheaper-usage skill"
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

# Create cheaper-usage skill

## Work

`.apm/skills/cheaper-usage/SKILL.md` を作成。cheaper CLI の全 command
（`run <worker>`・alias・`count-tokens`）、stdin JSON contract、envelope
（ok/result/meta/error）、exit code、`--cwd`/`--format`/`--quiet`、
provider/model 設定（models.yaml・policy）の入口を英語で記述する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/designs/0013-apm-skills-and-pre-read-hook.md`
- `docs/specs/0008-apm-delegation-skills-and-read-hook.md`

## Dependencies

- なし

## Affected Files

- `.apm/skills/cheaper-usage/SKILL.md`（新規）

## Done When

- [ ] SKILL.md が `name`/`description` frontmatter を持つ
- [ ] CLI contract が実装（cli.ts/envelope/errors）と一致する

## Verification

- [ ] frontmatter と本文の目視確認
