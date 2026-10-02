---
id: "TASK-034z2pjfqDxeH4X4QdxAQx"
type: "task"
status: "todo"
title: "Verify release workflow locally and record smoke evidence"
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
    - "docs/plans/integrate-sea-executables-into-tag-triggered-github-release-workflow.md"
  implemented-by: []
  depends-on:
    - "docs/plans/integrate-sea-executables-into-tag-triggered-github-release-workflow.md"
    - "docs/tasks/create-manual-dispatch-release-workflow-with-3-platform-sea-matrix.md"
    - "docs/tasks/update-release-guide-for-manual-sea-release-workflow.md"
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
    - "docs/test-specs/release-workflow-manual-dispatch-and-artifact-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# Verify release workflow locally and record smoke evidence

## Work

release workflow の検証を current host で実施し、結果を
Implementation Record に記録する。実 dispatch（GitHub Release 作成）
は行わない。

確認内容:

- `release.yml` の YAML parse と `on:` が `workflow_dispatch` のみ
  であること
- workflow の各 step 相当を local で再現: mise の locked Node が
  `node --version` で ≥26.9.0、`pnpm install --frozen-lockfile`
  成功、`node scripts/build-executable.mjs --target win32-x64` が
  artifact を生成、`--version` / `models` smoke が通る
- matrix の runner→target→artifact 名の対応が
  `build-executable.mjs` の命名規則と一致
- `docs/guides/release.md` の記述と workflow の整合

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/integrate-sea-executables-into-tag-triggered-github-release-workflow.md
- docs/test-specs/release-workflow-manual-dispatch-and-artifact-contract.md

## Dependencies

- `create-manual-dispatch-release-workflow-with-3-platform-sea-matrix`
- `update-release-guide-for-manual-sea-release-workflow`

## Affected Files

- `.github/workflows/release.yml`（検証対象）
- `docs/impl/ir/`（検証記録）

## Done When

- [ ] 上記確認項目がすべて実施済みで結果が IR に記録されている
- [ ] `mise run check` が pass

## Verification

- [ ] `node --version` が v26.9.0 以上
- [ ] `node scripts/build-executable.mjs --target win32-x64` が成功
- [ ] 生成 exe で `--version`・`models` が exit 0
- [ ] `mise run check` pass
