---
id: "TASK-034z3zPzG5T5Q8QpwsaoTJ"
type: "task"
status: "done"
title: "Track APM executables with Git LFS and keep freshness check valid"
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
    - "docs/plans/assemble-apm-executable-payload-with-platform-launchers.md"
  implemented-by:
    - "docs/impl/ir/track-executables-with-git-lfs.md"
  depends-on:
    - "docs/plans/assemble-apm-executable-payload-with-platform-launchers.md"
    - "docs/tasks/implement-build-apm-bundle-mjs-assembling-executables-into-hooks-tree.md"
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
    - "docs/test-specs/apm-executable-payload-layout-and-launcher-dispatch-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# Track APM executables with Git LFS and keep freshness check valid

## Work

`.gitattributes` に `packages/apm/.apm/hooks/cli/bin/` 配下の
executable 用 LFS rule を追加する:

```text
packages/apm/.apm/hooks/cli/bin/**/cheaper filter=lfs diff=lfs merge=lfs -text
packages/apm/.apm/hooks/cli/bin/**/cheaper.exe filter=lfs diff=lfs merge=lfs -text
```

- `git lfs install` 前提を確認し、add 時に pointer file 化される
  ことを `git lfs status` / `git cat-file` で検証
- `mise run check` の bundle freshness step（`git diff --exit-code`）
  が LFS pointer 比較として引き続き機能することを確認
- dev 手順（AGENTS.md または release guide）に `git lfs install`
  の前提を追記

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/assemble-apm-executable-payload-with-platform-launchers.md
- docs/test-specs/apm-executable-payload-layout-and-launcher-dispatch-contract.md

## Dependencies

- `implement-build-apm-bundle-mjs-...`（executable が tree に
  配置されてから LFS 化を検証する）

## Affected Files

- `.gitattributes`（変更）
- `docs/guides/release.md` または `AGENTS.md`（`git lfs` 前提の追記）

## Done When

- [ ] <!-- completion criterion -->

## Verification

- [ ] <!-- command, test, review step, or observable behavior -->
