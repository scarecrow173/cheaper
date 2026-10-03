---
id: "TASK-034z3zQE9PpSPE6CLsbpTP"
type: "task"
status: "done"
title: "Verify launchers and APM payload assembly locally"
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
    - "docs/impl/ir/verify-launchers-and-apm-payload.md"
  depends-on:
    - "docs/plans/assemble-apm-executable-payload-with-platform-launchers.md"
    - "docs/tasks/add-posix-and-powershell-platform-launchers-to-hooks-payload.md"
    - "docs/tasks/track-apm-executables-with-git-lfs-and-keep-freshness-check-valid.md"
    - "docs/tasks/update-cheaper-usage-skill-guidance-for-executable-based-cli.md"
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

# Verify launchers and APM payload assembly locally

## Work

payload 組み立てと launcher の動作を local で検証し、結果を
Implementation Record に記録する。

- `node scripts/build-apm-bundle.mjs --exe-dir dist/sea` の実行と
  tree 構成の確認
- Git Bash / pwsh で launcher 経由 `--version`・`models` の実行
- LFS pointer 化の確認（`git lfs ls-files`）
- APM install 後の検証は可能な範囲で（`apm install` が local で
  動くなら deploy 後 path/mode 確認）
- `mise run check`

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/assemble-apm-executable-payload-with-platform-launchers.md
- docs/test-specs/apm-executable-payload-layout-and-launcher-dispatch-contract.md

## Dependencies

- `add-posix-and-powershell-platform-launchers-...`
- `track-apm-executables-with-git-lfs-...`
- `update-cheaper-usage-skill-guidance-...`

## Affected Files

- `.github/`（検証対象外）
- `docs/impl/ir/`（検証記録）

## Done When

- [ ] <!-- completion criterion -->

## Verification

- [ ] <!-- command, test, review step, or observable behavior -->
