---
id: "TASK-X2PS53bel6fwygkwU5RpqZ"
type: "task"
status: "done"
title: "consumer 向け・repo 文書の新 layout 追従と falsified doc set 関連付け"
created: "2026-10-18"
updated: "2026-10-18"
owners:
  - scarecrow173
relations:
  source:
    - "docs/plans/apm-hook-per-target-package-split.md"
  changes:
    added: []
    modified:
      - {type: "file", path: "packages/apm/.apm/skills/cheaper-usage/SKILL.md"}
      - {type: "file", path: "docs/guides/distribution.md"}
      - {type: "file", path: "AGENTS.md"}
      - {type: "file", path: "CONTRIBUTING.md"}
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/apm-hook-per-target-package-split.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/apm-hook-packages-split.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/apm-hook-per-target-package-split.md"
    - "docs/tasks/apm-hook-packages-split.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/apm-hook-per-target-package-split.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "packages/apm/.apm/skills/cheaper-usage/SKILL.md"
    - "docs/guides/distribution.md"
  defers: []
  deferred-by: []
---

# Task: 文書追従

## やること

1. `packages/apm/.apm/skills/cheaper-usage/SKILL.md`:
   - bundle root 表を `apm-hook-*` dep name へ更新。
   - `apm update` 案内を uninstall → install へ修正(orphan 回避)。
   - 最小 APM version(≥0.28 推奨)を記載。
2. `docs/guides/distribution.md`:
   - deploy 先 `<hooks_root>/<dep-name>/` が `apm-hook-*` になる旨。
   - dep-edge `targets:` が transitive に伝播しない breaking change
     (design §5.5)と回避策(active target を agent-skills のみに絞る、
     または policy `dependencies.deny`)。
   - `packages/apm` 単体 dir コピー install・`apm pack`/`publish`
     非対応の明記。
   - 移行手順(design §5: uninstall→install、-g scope、旧 ref pin、
     changed-file nonzero exit 時の手動 cleanup)。
3. `AGENTS.md` / `CONTRIBUTING.md` の `packages/apm` 説明を更新。
4. falsified doc set(7 件、design §4 記載)の本文冒頭に
   「hook 配布 layout は SPEC-2w4XxXeDF0glTfgrBVJnWb で変更済み」
   の1行 note を追加。

## Acceptance

- 各 file が新 layout と矛盾しない記述になる。
- issue #118 への close コメントに移行手順を記載する素材が揃う。
