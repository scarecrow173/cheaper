---
id: "TASK-d1Fg1OH0NcI8xj5WAkWvts"
type: "task"
status: "done"
title: "packages/: hook package 分割(4 sibling + 親 deps + 旧 hooks 削除)"
created: "2026-10-18"
updated: "2026-10-18"
owners:
  - scarecrow173
relations:
  source:
    - "docs/plans/apm-hook-per-target-package-split.md"
  changes:
    added:
      - {type: "dir", path: "packages/apm-hook-windsurf"}
      - {type: "dir", path: "packages/apm-hook-cursor"}
      - {type: "dir", path: "packages/apm-hook-copilot"}
      - {type: "dir", path: "packages/apm-hook-pretooluse"}
    modified:
      - {type: "file", path: "packages/apm/apm.yml"}
    deleted:
      - {type: "dir", path: "packages/apm/.apm/hooks"}
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/apm-hook-per-target-package-split.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/apm-hook-per-target-package-split.md"
    - "docs/tasks/apm-hook-split-test-follow.md"
    - "docs/tasks/apm-hook-split-docs.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/apm-hook-per-target-package-split.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "packages/apm/apm.yml"
    - "packages/apm-hook-claude/.apm/hooks/hooks.json"
    - "packages/apm-hook-codex/.apm/hooks/hooks.json"
  defers: []
  deferred-by: []
---

# Task: packages/ hook package 分割

## やること

1. 4 sibling package を作成。各 `apm.yml` は一意 `name:`
   (`cheaper-hook-windsurf` 等)・親と同一 `version`・対応する
   `target:`/`targets:` を持つ。
2. 各 `.apm/hooks/hooks.json` に現行 `packages/apm/.apm/hooks/hooks.json`
   の該当 event entry をそのまま移す(command/powershell/matcher/
   show_output 不変)。`cheaper.sh`/`cheaper.ps1` を byte 同一でコピー
   (`cheaper.sh` は git mode 100755 を維持)。
3. `packages/apm/apm.yml` に `dependencies.apm` として 4 sibling の
   `path:` dep + per-dep `targets:` を追加。description を
   「skills + thin PATH launchers(hooks は sibling package)」系へ更新。
4. `packages/apm/.apm/hooks/` を削除。

## Acceptance

- fixture consumer(local `path:` dep)で `apm install` が 5 dep を解決し、
  各 target に native event のみ deploy される(0.28.0 で確認)。
- `git ls-files -s` で 4 sibling の `cheaper.sh` が 100755。
